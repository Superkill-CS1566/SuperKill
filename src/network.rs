use bevy::prelude::*;
use tokio::net::UdpSocket;
use postcard::{from_bytes, to_allocvec};
use crate::protocol::{
    ClientMessage, PlayerInput, ServerMessage,
};
use tokio::sync::mpsc;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_network_client)
            .add_systems(Update, send_player_input);
    }
}

#[derive(Resource)]
pub struct NetworkClient {
    pub outgoing: mpsc::Sender<ClientMessage>,
}

fn start_network_client(mut commands: Commands) {
    
    let (outgoing_sender, mut outgoing_receiver) = 
        mpsc::channel::<ClientMessage>(64);

    commands.insert_resource(NetworkClient {
        outgoing: outgoing_sender,
    });

    std::thread::spawn(move || {

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        runtime.block_on(async {
            println!("Tokio network runtime started");

            let socket = UdpSocket::bind("127.0.0.1:0").await
                .expect("Failed to bind client UDP socket");

            let server_address = "127.0.0.1:5000";

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
                tokio::select! {
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
        block: keyboard.pressed(KeyCode::KeyS),
    };

    let message = ClientMessage::PlayerInput(input);

    if let Err(error) = network.outgoing.try_send(message) {
        println!("Failed to queue player input: {error}");
    }
}
