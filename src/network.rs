use bevy::prelude::*;
use tokio::net::UdpSocket;
use postcard::{from_bytes, to_allocvec};
use crate::protocol::{
    ClientMessage, PlayerInput, ServerMessage,
};
use tokio::sync::mpsc;
use std::sync::Mutex;
use crate::player::{Player, PlayerSide};

pub struct NetworkPlugin;

// adds client networking systems to the Bevy Application
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_network_client)
            .add_systems(
                Update, 
                (send_player_input, apply_server_messages),
            );
    }
}

// stores channels used to communicate with the Tokio network thread
#[derive(Resource)]
pub struct NetworkClient {
    pub outgoing: mpsc::Sender<ClientMessage>,
    pub incoming: Mutex<mpsc::Receiver<ServerMessage>>,
}

// starts the UDP client on a separate Tokio thread
fn start_network_client(mut commands: Commands) {
    
    // sends Bevy messages to the network thread
    let (outgoing_sender, mut outgoing_receiver) = 
        mpsc::channel::<ClientMessage>(64);

    // sends server messages back to Bevy
    let (incoming_sender, incoming_receiver) = 
        mpsc::channel::<ServerMessage>(64);

    commands.insert_resource(NetworkClient {
        outgoing: outgoing_sender,
        incoming: Mutex::new(incoming_receiver),
    });

    // keep asynchronous networking separate from Bevy's main thread
    std::thread::spawn(move || {

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        runtime.block_on(async {
            println!("Tokio network runtime started");

            // use any available local port for this client
            let socket = UdpSocket::bind("127.0.0.1:0").await
                .expect("Failed to bind client UDP socket");

            let server_address = "127.0.0.1:5000";

            // register this client with server
            let connect_msg = ClientMessage::Connect;
            let bytes = to_allocvec(&connect_msg).expect("Failed to serialize Connect");
            let bytes_sent = socket.send_to(&bytes, server_address).await
                .expect("Failed to send Connect message");

            println!("Bevy client sent {bytes_sent} bytes to {server_address}");

            let client_address = socket.local_addr()
                .expect("Failed to read client address");
            println!("Bevy client bound to {client_address}");

            let mut receive_buffer = [0u8; 1024];
            
            loop {

                // wait for either outgoing input or an incoming server message
                tokio::select! {

                    // serialize and send a queued client message
                    Some(outgoing_message) = outgoing_receiver.recv() => {
                        match to_allocvec(&outgoing_message) {
                            Ok(bytes) => {
                                if let Err(error) =
                                    socket.send_to(&bytes, server_address).await
                                {
                                    println!("Failed to send message: {error}");
                                }
                            }
                            Err(error) => {
                                println!("Failed to serialize message: {error}");
                            }
                        }
                    },

                    // receive and deserialize the next server message
                    receive_result =
                        socket.recv_from(&mut receive_buffer) => {
                        match receive_result {
                            Ok((length, sender)) => {
                                match from_bytes::<ServerMessage>(
                                    &receive_buffer[..length]
                                ) {
                                    Ok(message) => {
                                        println!(
                                            "Received from {sender}: {message:?}"
                                        );

                                        if let Err(error) = incoming_sender.try_send(message) {
                                            println!("Failed to queue server message: {error}");
                                        }
                                    }
                                    Err(error) => {
                                        println!(
                                            "Failed to decode server message: {error}"
                                        );
                                    }
                                }
                            }

                            Err(error) => {
                                println!(
                                    "Failed to receive server message: {error}"
                                );
                                break;
                            }
                        }
                    },
                }
            }
        });
    });
}

// read keyboard input and queue it for the server
fn send_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    network: Res<NetworkClient>,
) {
    let mut movement_x = 0.0;

    if keyboard.pressed(KeyCode::KeyA) {
        movement_x -= 1.0;
    }

    if keyboard.pressed(KeyCode::KeyD) {
        movement_x += 1.0;
    }

    let input = PlayerInput {
        movement_x,
        movement_y: 0.0, 
        jump: keyboard.just_pressed(KeyCode::KeyW),
        crouch: keyboard.pressed(KeyCode::KeyS),
    };

    let message = ClientMessage::PlayerInput(input);

    if let Err(error) = network.outgoing.try_send(message) {
        println!("Failed to queue player input: {error}");
    }
}

// apply server-controlled player states to the Bevy world
fn apply_server_messages(
    network: Res<NetworkClient>,
    mut player_query: Query<(&PlayerSide, &mut Player, &mut Transform)>,
) {
    let Ok(mut incoming) = network.incoming.lock() else {
        return;
    };

    while let Ok(message) = incoming.try_recv() {
        match message {
            ServerMessage::Connected { player_id } => {
                println!("This client is Player {player_id}");
            }

            ServerMessage::Rejected { reason } => {
                println!("Server rejected connection: {reason}");
            }

            ServerMessage::PlayerJoined { player_id } => {
                println!("Player {player_id} joined");
            }

            ServerMessage::GameState { players: snapshots } => {
                for (side, mut player, mut transform) in &mut player_query {
                    let player_id = match side {
                        PlayerSide::Left => 1,
                        PlayerSide::Right => 2,
                    };

                    if let Some(snapshot) = snapshots
                        .iter()
                        .find(|snapshot| snapshot.player_id == player_id)
                    {
                        player.prev_crouching = player.is_crouching;
                        player.is_crouching = snapshot.is_crouching;

                        // lower the shorter sprite so its feet remain on the ground
                        let crouch_offset = if player.is_crouching {
                            (player.height - player.crouch_height) / 2.0
                        } else {
                            0.0
                        };
                        transform.translation.x = snapshot.position_x;
                        transform.translation.y = snapshot.position_y - crouch_offset;
                    }
                }
            }
        }
    }
}
