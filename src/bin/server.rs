use tokio::net::UdpSocket;
use std::io;
use std::collections::HashMap;
use std::net::SocketAddr;
use postcard::{from_bytes, to_allocvec};
use std::time::{Duration, Instant};
use tokio::time::{interval, MissedTickBehavior};

// import the shared network messages and gameplay simulation
use superkill::{
    protocol::{ClientMessage, PlayerInput, ServerMessage},
    simulation::{MatchState, TICK_SECONDS},
};

#[tokio::main]
async fn main() -> io::Result<()> {

    // create the UDP socket that listens for client messages
    let socket = UdpSocket::bind("0.0.0.0:5000").await?;
    let mut buf = [0; 1024];
    
    // maps each network address to its assigned player ID
    let mut players: HashMap<SocketAddr, u8> = HashMap::new();
    
    // store both players in the shared match state
    let mut game = MatchState::new();

    // keep each player's latest input until the next simulation tick
    let idle_input = PlayerInput {
        movement_x: 0.0,
        movement_y: 0.0,
        jump: false,
        crouch: false,
        attack: false,
    };

    let mut latest_inputs = [idle_input.clone(), idle_input];
    let mut pending_jumps = [false; 2];
    let mut pending_attacks = [false; 2];
    let mut last_input_at: [Option<Instant>; 2] = [None; 2];

    let mut ticker = interval(Duration::from_secs_f32(TICK_SECONDS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    println!("Server listening on 0.0.0.0:5000");

    loop {

        tokio::select! {
            received = socket.recv_from(&mut buf) => {
                let (len, addr) = received?;

                // convert the received Postcard bytes into a client message
                let msg: ClientMessage = match from_bytes(&buf[..len]) {
                    Ok(m) => m,
                    Err(e) => {
                        println!("Failed to parse message from {addr}: {e}");
                        continue;
                    }
                };

                match msg {
                    ClientMessage::Connect => {

                        // reject addresses that have already connected
                        if players.contains_key(&addr) {
                            let reply = ServerMessage::Rejected { reason: "Already connected".into() };
                            if let Ok(bytes) = to_allocvec(&reply) {
                                let _ = socket.send_to(&bytes, addr).await;
                            }
                            continue;
                        }

                        // only allow two players to connect
                        if players.len() >= 2 {
                            let reply = ServerMessage::Rejected { reason: "Server full".into() };
                            if let Ok(bytes) = to_allocvec(&reply) {
                                let _ = socket.send_to(&bytes, addr).await;
                            }
                            println!("Server full. Rejecting {addr}");
                            continue;
                        }

                        // assign the next avaialble player ID
                        let player_id = players.len() as u8 + 1;
                        players.insert(addr, player_id);

                        println!("Registered {addr} as Player {player_id}");
                        println!("Connected players: {}/2", players.len());

                        // tell new client which player it controls
                        let welcome = ServerMessage::Connected { player_id };
                        if let Ok(bytes) = to_allocvec(&welcome) {
                            let _ = socket.send_to(&bytes, addr).await;
                        }

                        // notify the other client that a player has joined
                        let broadcast = ServerMessage::PlayerJoined { player_id };
                        if let Ok(bytes) = to_allocvec(&broadcast) {
                            for (&client_addr, _) in players.iter() {
                                if client_addr != addr {
                                    let _ = socket.send_to(&bytes, client_addr).await;
                                }
                            }
                        }
                    }
                    ClientMessage::Heartbeat => {
                        if let Some(pid) = players.get(&addr) {
                            println!("Heartbeat from Player {pid} ({addr})");
                        }
                    }

                    ClientMessage::PlayerInput(input) => {

                        // accept input only from a registered player's connection
                        if let Some(&player_id) = players.get(&addr) {
                            let index = (player_id - 1) as usize;

                            // save the latest controls and hold jump until the next tick
                            pending_jumps[index] |= input.jump;
                            pending_attacks[index] |= input.attack;
                            latest_inputs[index] = input;
                            latest_inputs[index].jump = false;
                            latest_inputs[index].attack = false;

                            // record when this player last sent input
                            last_input_at[index] = Some(Instant::now());
                        } else {
                            println!("Ignoring input from unregistered client {addr}");
                        }
                    }
                }
            },
            _ = ticker.tick() => {
                // prepare each player's input for this simulation tick
                let mut frame_inputs = latest_inputs.clone();

                for index in 0..2 {
                    let recent = last_input_at[index]
                        .is_some_and(|time| time.elapsed() <= Duration::from_millis(250));

                    if recent {
                        frame_inputs[index].jump = pending_jumps[index];
                        frame_inputs[index].attack = pending_attacks[index];
                    } else {
                        // stop movement and actions when input stops arriving
                        frame_inputs[index].movement_x = 0.0;
                        frame_inputs[index].movement_y = 0.0;
                        frame_inputs[index].jump = false;
                        frame_inputs[index].crouch = false;
                        frame_inputs[index].attack = false;
                    }
                }

                // advance both players once at the fixed server rate
                game.step([&frame_inputs[0], &frame_inputs[1]]);
                pending_attacks = [false; 2];
                pending_jumps = [false; 2];

                // send the updated player states to every connected client
                if !players.is_empty() {
                    let update = ServerMessage::GameState {
                        players: game.snapshots(),
                    };

                    if let Ok(bytes) = to_allocvec(&update) {
                        for &client_addr in players.keys() {
                            let _ = socket.send_to(&bytes, client_addr).await;
                        }
                    }
                }
            },
        }
    }
}
