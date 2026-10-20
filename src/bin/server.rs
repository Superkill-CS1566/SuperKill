use tokio::net::UdpSocket;
use std::io;
use std::collections::HashMap;
use std::net::SocketAddr;
use postcard::{from_bytes, to_allocvec};

#[path = "../protocol.rs"]
mod protocol;
use protocol::{
    ClientMessage, PlayerSnapshot, ServerMessage,
};

// values used to calculate player movement on server
const GROUND_LEVEL: f32 = -240.0;
const PLAYER_SPEED: f32 = 320.0;
const JUMP_FORCE: f32 = 620.0;
const GRAVITY: f32 = 1800.0;
const TIME_STEP: f32 = 1.0/60.0;

#[tokio::main]
async fn main() -> io::Result<()> {

    // create the UDP socket that listens for client messages
    let socket = UdpSocket::bind("0.0.0.0:5000").await?;
    let mut buf = [0; 1024];
    
    // maps each network address to its assigned player ID
    let mut players: HashMap<SocketAddr, u8> = HashMap::new();
    // stores latest state of each player
    let mut player_states: HashMap<u8, PlayerSnapshot> = 
        HashMap::new();
    // stores each players vertical speed
    let mut vertical_velocities: HashMap<u8, f32> = 
        HashMap::new();

    println!("Server listening on 0.0.0.0:5000");

    loop {

        // wait for next message from client
        let (len, addr) = socket.recv_from(&mut buf).await?;

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

                let starting_x = if player_id == 1 {
                    -200.0
                } else {
                    200.0
                };

                // create the player's initial state
                player_states.insert(
                    player_id,
                    PlayerSnapshot {
                        player_id,
                        position_x: starting_x,
                        position_y: GROUND_LEVEL,
                        is_crouching: false,
                    },
                );
                
                vertical_velocities.insert(player_id, 0.0);

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

                // only accept movement from registered clients
                if let Some(&player_id) = players.get(&addr) {

                    if let Some(state) = player_states.get_mut(&player_id) {

                        // update crouching and horizontal movement
                        let is_grounded = state.position_y <= GROUND_LEVEL;
                        state.is_crouching = input.crouch && is_grounded; 

                        let movement_speed = if state.is_crouching {
                            PLAYER_SPEED * 0.5
                        } else {
                            PLAYER_SPEED
                        };

                        state.position_x += 
                            input.movement_x * movement_speed * TIME_STEP;

                        // update jumping, gravity, and ground collision
                        let vertical_velocity = vertical_velocities
                            .entry(player_id)
                            .or_insert(0.0);

                        if input.jump && state.position_y <= GROUND_LEVEL && !state.is_crouching{
                            *vertical_velocity = JUMP_FORCE;
                        }

                        if state.position_y > GROUND_LEVEL
                            || *vertical_velocity > 0.0
                        {
                            *vertical_velocity -= GRAVITY * TIME_STEP;
                            state.position_y += *vertical_velocity * TIME_STEP;

                            if state.position_y <= GROUND_LEVEL {
                                state.position_y = GROUND_LEVEL;
                                *vertical_velocity = 0.0;
                            }
                        }

                        println!(
                            "Player {player_id} position: ({}, {})",
                            state.position_x,
                            state.position_y
                        );
                    }

                    // colelct latest player states into one game update
                    let snapshots: Vec<PlayerSnapshot> =
                        player_states.values().cloned().collect();

                    let update = ServerMessage::GameState {
                        players: snapshots,
                    };

                    // send update game state to every connected client
                    if let Ok(bytes) = to_allocvec(&update) {
                        for &client_addr in players.keys() {
                            let _ = socket
                                .send_to(&bytes, client_addr)
                                .await;
                        }
                    }
                } else {
                    println!("Ignoring input from unregistered client {addr}");
                }
            }
        }
    }
}
