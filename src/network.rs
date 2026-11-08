use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use bevy::prelude::*;
use postcard::{from_bytes, to_allocvec};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::common::AppState;
use crate::game::{GameState, Match};
use crate::loading::despawn_with;
use crate::player::{PendingInputs, PlayerSide};
use crate::protocol::{ClientMessage, ServerMessage, INPUT_HISTORY, SERVER_PORT};
use crate::sim::{Input, MatchState};

// Online play. The server runs the match; this client sends its own inputs
// and draws whatever state the server sends back. Local play doesn't use any
// of this — it only switches on with --online.

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let online = std::env::args().any(|arg| arg == "--online");
        if !online {
            return;
        }

        app.insert_resource(OnlineSession {
            server_address: server_address(),
            ..default()
        })
        .add_systems(Startup, start_network_client)
        .add_systems(FixedUpdate, online_tick)
        .add_systems(OnEnter(AppState::InGame), spawn_connection_status)
        .add_systems(
            Update,
            update_connection_status.run_if(in_state(AppState::InGame)),
        )
        .add_systems(OnExit(AppState::InGame), despawn_with::<ConnectionStatus>);
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

/// Present only in online play — game.rs checks for it to know not to run
/// the match itself.
#[derive(Resource, Default)]
pub struct OnlineSession {
    server_address: String,
    /// Which fighter this client controls, once the server says.
    side: Option<PlayerSide>,
    /// Why the server last turned us away, if it has.
    rejected: Option<String>,
    /// How many players the server last reported.
    players: u8,
    /// Number of the next input to send.
    next_seq: u32,
    /// The last few inputs sent, repeated in every packet.
    recent: VecDeque<Input>,
    /// The newest snapshot received but not yet shown.
    newest: Option<(u32, MatchState)>,
    /// The server tick of the snapshot on screen.
    shown_tick: u32,
}

// stores channels used to communicate with the Tokio network thread
#[derive(Resource)]
struct NetworkClient {
    outgoing: mpsc::Sender<ClientMessage>,
    incoming: Mutex<mpsc::Receiver<ServerMessage>>,
}

// ---------------------------------------------------------------- network thread

// starts the UDP client on a separate Tokio thread
fn start_network_client(mut commands: Commands, session: Res<OnlineSession>) {
    let server_address = session.server_address.clone();

    let (outgoing_sender, outgoing_receiver) = mpsc::channel::<ClientMessage>(64);
    let (incoming_sender, incoming_receiver) = mpsc::channel::<ServerMessage>(64);

    commands.insert_resource(NetworkClient {
        outgoing: outgoing_sender,
        incoming: Mutex::new(incoming_receiver),
    });

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        runtime.block_on(run_connection(server_address, outgoing_receiver, incoming_sender));
    });
}

async fn run_connection(
    server_address: String,
    mut outgoing: mpsc::Receiver<ClientMessage>,
    incoming: mpsc::Sender<ServerMessage>,
) {
    // 0.0.0.0 rather than 127.0.0.1, so the socket can reach other machines
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(socket) => socket,
        Err(error) => {
            println!("Failed to bind client UDP socket: {error}");
            return;
        }
    };
    println!("Connecting to server at {server_address}");

    // keep asking to join until the server answers, so it doesn't matter
    // whether the server or the game was started first
    let mut retry = tokio::time::interval(Duration::from_secs(1));
    let mut connected = false;
    let mut buffer = [0u8; 4096];

    loop {
        tokio::select! {
            _ = retry.tick(), if !connected => {
                send(&socket, &server_address, &ClientMessage::Connect).await;
            }

            Some(message) = outgoing.recv() => {
                if connected {
                    send(&socket, &server_address, &message).await;
                }
            }

            result = socket.recv_from(&mut buffer) => {
                match result {
                    Ok((length, _sender)) => match from_bytes::<ServerMessage>(&buffer[..length]) {
                        Ok(message) => {
                            match &message {
                                ServerMessage::Connected { .. } => connected = true,
                                // turned away, or the server restarted — ask again
                                ServerMessage::Rejected { .. } => connected = false,
                                _ => {}
                            }
                            // snapshots arrive every tick, so if the game falls
                            // behind, dropping one here is harmless
                            let _ = incoming.try_send(message);
                        }
                        Err(error) => println!("Failed to decode server message: {error}"),
                    },
                    // Windows reports an unreachable server as an error on the
                    // next receive. Keep listening; the retry finds it once it's up.
                    Err(_) => {}
                }
            }
        }
    }
}

async fn send(socket: &UdpSocket, address: &str, message: &ClientMessage) {
    match to_allocvec(message) {
        Ok(bytes) => {
            if let Err(error) = socket.send_to(&bytes, address).await {
                println!("Failed to send message: {error}");
            }
        }
        Err(error) => println!("Failed to encode message: {error}"),
    }
}

// ---------------------------------------------------------------- each tick

// One tick of online play, on the same fixed clock local play uses: send this
// client's input, then show the newest state the server has sent.
fn online_tick(
    network: Option<Res<NetworkClient>>,
    mut session: ResMut<OnlineSession>,
    mut pending: ResMut<PendingInputs>,
    app_state: Res<State<AppState>>,
    game_state: Res<State<GameState>>,
    game: Option<ResMut<Match>>,
) {
    let Some(network) = network else {
        return;
    };
    receive(&network, &mut session);

    // Input goes out every tick while connected, even from the menus — it
    // doubles as the heartbeat that tells the server we're still here.
    if session.side.is_some() {
        let inputs = pending.take();
        let playing =
            *app_state.get() == AppState::InGame && *game_state.get() == GameState::Playing;
        // only one person plays on each machine online, so either set of keys
        // works. In a menu or paused, the fighter stands still.
        let input = if playing {
            either(inputs[0], inputs[1])
        } else {
            Input::default()
        };
        send_input(&network, &mut session, input);
    }

    // previous is kept for smooth drawing, exactly as in local play
    if let Some(mut game) = game {
        let current = game.state.clone();
        game.previous = current;
        if let Some((tick, state)) = session.newest.take() {
            game.state = state;
            session.shown_tick = tick;
        }
    }
}

fn receive(network: &NetworkClient, session: &mut OnlineSession) {
    let Ok(mut incoming) = network.incoming.lock() else {
        return;
    };

    while let Ok(message) = incoming.try_recv() {
        match message {
            ServerMessage::Connected { side } => {
                if session.side != Some(side) {
                    println!("Connected — you are {}", side.label());
                }
                session.side = Some(side);
                session.rejected = None;
            }
            ServerMessage::Rejected { reason } => {
                println!("Server rejected the connection: {reason}");
                session.side = None;
                session.rejected = Some(reason);
            }
            ServerMessage::PlayerJoined { side } => println!("{} joined", side.label()),
            ServerMessage::PlayerLeft { side } => println!("{} left", side.label()),
            ServerMessage::Snapshot {
                server_tick,
                players,
                state,
            } => {
                session.players = players;
                // UDP can deliver out of order — keep only the newest
                let newest_so_far = match &session.newest {
                    Some((tick, _)) => *tick,
                    None => session.shown_tick,
                };
                if server_tick > newest_so_far {
                    session.newest = Some((server_tick, state));
                }
            }
        }
    }
}

// Each packet repeats the last few inputs, so a press survives a lost packet.
fn send_input(network: &NetworkClient, session: &mut OnlineSession, input: Input) {
    session.recent.push_back(input);
    while session.recent.len() > INPUT_HISTORY {
        session.recent.pop_front();
    }

    let message = ClientMessage::Input {
        seq: session.next_seq,
        inputs: session.recent.iter().copied().collect(),
    };
    session.next_seq = session.next_seq.wrapping_add(1);

    // the channel only fills up if the network thread has stopped
    let _ = network.outgoing.try_send(message);
}

fn either(a: Input, b: Input) -> Input {
    Input {
        left: a.left || b.left,
        right: a.right || b.right,
        crouch: a.crouch || b.crouch,
        jump: a.jump || b.jump,
        attack: a.attack || b.attack,
        block: a.block || b.block,
        throw: a.throw || b.throw,
    }
}

// ---------------------------------------------------------------- status text

#[derive(Component)]
struct ConnectionStatus;

fn spawn_connection_status(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(20.0),
            ..default()
        },
        ConnectionStatus,
    ));
}

// Says what the connection is waiting on, and disappears once a match is running.
fn update_connection_status(
    session: Res<OnlineSession>,
    mut status: Query<(&mut Text, &mut Visibility), With<ConnectionStatus>>,
) {
    let message = match (&session.side, &session.rejected) {
        (None, Some(reason)) => format!("Can't join: {reason}. Retrying..."),
        (None, None) => format!("Connecting to {}...", session.server_address),
        (Some(_), _) if session.players < 2 => "Waiting for the other player...".to_string(),
        (Some(_), _) => String::new(),
    };

    for (mut text, mut visibility) in &mut status {
        *visibility = if message.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if text.0 != message {
            text.0 = message.clone();
        }
    }
}