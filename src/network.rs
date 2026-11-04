use bevy::prelude::*;
use tokio::net::UdpSocket;
use postcard::{from_bytes, to_allocvec};
use crate::combat::{AttackType, Dead, Priority, HITSTUN_DURATION};
use crate::common::UiCamera;
use crate::protocol::{
    ClientMessage, PlayerSnapshot, ServerMessage,
};
use tokio::sync::mpsc;
use std::sync::Mutex;
use crate::player::{NetworkControlled, Player, PlayerSide};

/// The port the server listens on.
const SERVER_PORT: u16 = 5000;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let online = std::env::args().any(|arg| arg == "--online");
        if !online {
            return;
        }

        app.add_systems(Startup, start_network_client)
            .add_systems(
                Update,
                (mark_network_controlled, send_player_state, apply_server_messages),
            );
    }
}

/// Which server to connect to: the argument after `--online`, or this machine
/// if none was given. A bare address like `192.168.1.50` gets the default port.
///
///   cargo run -- --online                   connects to this machine
///   cargo run -- --online 192.168.1.50      connects to another device
fn server_address() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut address = String::from("127.0.0.1");

    for i in 0..args.len() {
        if args[i] != "--online" {
            continue;
        }
        if let Some(next) = args.get(i + 1) {
            if !next.starts_with("--") {
                address = next.clone();
            }
        }
    }

    if address.contains(':') {
        address
    } else {
        format!("{address}:{SERVER_PORT}")
    }
}

// stores channels used to communicate with the Tokio network thread
#[derive(Resource)]
pub struct NetworkClient {
    pub outgoing: mpsc::Sender<ClientMessage>,
    pub incoming: Mutex<mpsc::Receiver<ServerMessage>>,
    pub local_player_id: Option<u8>,
}

// starts the UDP client on a separate Tokio thread
fn start_network_client(mut commands: Commands) {

    // read before the thread starts, then moved into it
    let server_address = server_address();

    let (outgoing_sender, mut outgoing_receiver) =
        mpsc::channel::<ClientMessage>(64);

    let (incoming_sender, incoming_receiver) =
        mpsc::channel::<ServerMessage>(64);

    commands.insert_resource(NetworkClient {
        outgoing: outgoing_sender,
        incoming: Mutex::new(incoming_receiver),
        local_player_id: None,
    });

    std::thread::spawn(move || {

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        runtime.block_on(async {
            println!("Tokio network runtime started");

            // 0.0.0.0 rather than 127.0.0.1, so the socket can reach other
            // machines and not just this one
            let socket = UdpSocket::bind("0.0.0.0:0").await
                .expect("Failed to bind client UDP socket");

            println!("Connecting to server at {server_address}");

            let connect_msg = ClientMessage::Connect;
            let bytes = to_allocvec(&connect_msg).expect("Failed to serialize Connect");
            let _ = socket.send_to(&bytes, server_address.as_str()).await
                .expect("Failed to send Connect message");

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
                                    socket.send_to(&bytes, server_address.as_str()).await
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
                            Ok((length, _sender)) => {
                                match from_bytes::<ServerMessage>(
                                    &receive_buffer[..length]
                                ) {
                                    Ok(message) => {
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

// ensures exactly the local side is not network-controlled
fn mark_network_controlled(
    mut commands: Commands,
    network: Res<NetworkClient>,
    players: Query<(Entity, &PlayerSide)>,
) {
    let Some(local_id) = network.local_player_id else {
        return;
    };
    let Some(local_side) = PlayerSide::from_player_id(local_id) else {
        return;
    };

    for (entity, side) in &players {
        if *side == local_side {
            commands.entity(entity).remove::<NetworkControlled>();
        } else {
            commands.entity(entity).insert(NetworkControlled);
        }
    }
}

// sends the locally controlled player's state to the server
fn send_player_state(
    network: Res<NetworkClient>,
    mut players: Query<(&mut Player, &Transform, &PlayerSide), Without<NetworkControlled>>,
) {
    let Some(local_id) = network.local_player_id else {
        return;
    };
    let Some(local_side) = PlayerSide::from_player_id(local_id) else {
        return;
    };

    for (mut player, transform, side) in &mut players {
        if *side != local_side {
            continue;
        }

        let snapshot = PlayerSnapshot {
            player_id: local_id,
            position_x: transform.translation.x,
            position_y: transform.translation.y,
            velocity_x: player.velocity.x,
            velocity_y: player.velocity.y,
            facing: player.facing,
            is_crouching: player.is_crouching,
            is_grounded: player.is_grounded,
            is_dropping: player.is_dropping,
            health: player.health,
            has_weapon: player.has_weapon,
            is_blocking: player.is_blocking,
            is_stunned: player.is_stunned(),
            current_attack: player.current_attack.map(|a| a.to_u8()),
            damage_to_other: player.damage_out,
        };

        player.damage_out = 0;

        if let Err(error) = network.outgoing.try_send(ClientMessage::PlayerState(snapshot)) {
            println!("Failed to queue player state: {error}");
        }
    }
}

// applies remote snapshots to the network-controlled fighter
fn apply_server_messages(
    mut commands: Commands,
    mut network: ResMut<NetworkClient>,
    mut priority: ResMut<Priority>,
    mut remote_players: Query<
        (Entity, &mut Player, &mut Transform, &PlayerSide, Has<Dead>, &Visibility),
        (With<NetworkControlled>, Without<UiCamera>),
    >,
    mut local_players: Query<
        (Entity, &mut Player, &PlayerSide),
        (Without<NetworkControlled>, Without<UiCamera>),
    >,
) {
    let local_id = network.local_player_id;
    let mut pending_id: Option<u8> = None;

    {
        let Ok(mut incoming) = network.incoming.lock() else {
            return;
        };

        while let Ok(message) = incoming.try_recv() {
            match message {
                ServerMessage::Connected { player_id } => {
                    pending_id = Some(player_id);
                    println!("This client is Player {player_id}");
                }

            ServerMessage::Rejected { reason } => {
                println!("Server rejected connection: {reason}");
            }

            ServerMessage::PlayerJoined { player_id } => {
                println!("Player {player_id} joined");
            }

            ServerMessage::GameState { players: snapshots } => {
                for snapshot in &snapshots {
                    if local_id == Some(snapshot.player_id) {
                        continue; // our own echo, already authoritative locally
                    }
                    let Some(side) = PlayerSide::from_player_id(snapshot.player_id) else {
                        continue;
                    };

                    // remote damage against us — the attacker's snapshot carries it
                    if snapshot.damage_to_other > 0 {
                        for (entity, mut player, p_side) in &mut local_players {
                            if *p_side != side {
                                continue;
                            }
                            player.health = player.health.saturating_sub(snapshot.damage_to_other);
                            player.hitstun = Timer::from_seconds(HITSTUN_DURATION, TimerMode::Once);
                            player.velocity.x = 0.0;

                            if player.health == 0 {
                                commands
                                    .entity(entity)
                                    .insert((Dead::new(snapshot.position_x), Visibility::Hidden));
                                priority.0 = Some(side);
                            }
                        }
                    }

                    for (entity, mut player, mut transform, p_side, dead, visibility) in
                        &mut remote_players
                    {
                        if *p_side != side {
                            continue;
                        }

                        transform.translation.x = snapshot.position_x;
                        transform.translation.y = snapshot.position_y;
                        player.velocity = Vec2::new(snapshot.velocity_x, snapshot.velocity_y);
                        player.facing = snapshot.facing;
                        player.is_crouching = snapshot.is_crouching;
                        player.is_grounded = snapshot.is_grounded;
                        player.is_dropping = snapshot.is_dropping;
                        player.has_weapon = snapshot.has_weapon;
                        player.is_blocking = snapshot.is_blocking;
                        player.current_attack =
                            snapshot.current_attack.and_then(AttackType::from_u8);

                        // death and revival driven by the remote health value
                        if snapshot.health == 0 && !dead {
                            commands
                                .entity(entity)
                                .insert((Dead::new(snapshot.position_x), Visibility::Hidden));
                        } else if snapshot.health > 0 && dead {
                            commands
                                .entity(entity)
                                .remove::<Dead>()
                                .insert(Visibility::Inherited);
                        } else if snapshot.health > 0 && visibility != Visibility::Inherited {
                            commands.entity(entity).insert(Visibility::Inherited);
                        }
                    }
                }
            }
        }
    }
    }

    if let Some(id) = pending_id {
        network.local_player_id = Some(id);
    }
}