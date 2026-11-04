//! The game server. It runs the one copy of the match that counts.
//!
//! Each client sends only its inputs. The server steps the simulation at the
//! fixed tick rate — the same code local play runs — and sends every client the
//! resulting state. There's no Bevy here: the rules come straight from the
//! library's simulation.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use postcard::{from_bytes, to_allocvec};
use tokio::net::UdpSocket;
use tokio::time::interval;

use superkill::protocol::{ClientMessage, ServerMessage, ONLINE_LEVEL, SERVER_PORT};
use superkill::sim::{Input, MatchState, PlayerSide, Terrain, DEFAULT_SEED, TICK_SECONDS};

/// A client silent this long has their fighter stand still, so a stalled
/// connection doesn't leave them running in one direction.
const INPUT_TIMEOUT: Duration = Duration::from_millis(250);
/// A client silent this long has left, and their slot opens up.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How long the final state stays up after a win before a new match starts.
const RESTART_DELAY_TICKS: u32 = 180;

struct Client {
    side: PlayerSide,
    last_heard: Instant,
    /// Number of the newest input already used, so repeats and late packets
    /// are ignored.
    last_seq: Option<u32>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", SERVER_PORT)).await?;
    println!("Server listening on port {SERVER_PORT}");

    let terrain = Terrain::generate(ONLINE_LEVEL, DEFAULT_SEED);
    let mut game = MatchState::new();
    // each fighter's input for the next tick, built up from their packets
    let mut pending = [Input::default(); 2];
    let mut clients: HashMap<SocketAddr, Client> = HashMap::new();
    let mut server_tick: u32 = 0;
    let mut ticks_since_win: u32 = 0;

    let mut ticker = interval(Duration::from_secs_f32(TICK_SECONDS));
    let mut buffer = [0u8; 2048];

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let now = Instant::now();

                // anyone silent too long has left — free their slot
                let mut left = Vec::new();
                clients.retain(|address, client| {
                    let gone = now.duration_since(client.last_heard) > DISCONNECT_TIMEOUT;
                    if gone {
                        println!("{} ({address}) timed out", client.side.label());
                        left.push(client.side);
                    }
                    !gone
                });
                for side in left {
                    broadcast(&socket, &clients, &ServerMessage::PlayerLeft { side }).await;
                    // the match can't go on one-sided; start over once they're replaced
                    game = MatchState::new();
                    pending = [Input::default(); 2];
                    ticks_since_win = 0;
                }

                // a stalled connection stands still instead of holding its last keys
                for client in clients.values() {
                    if now.duration_since(client.last_heard) > INPUT_TIMEOUT {
                        pending[client.side.index()] = Input::default();
                    }
                }

                // the match only runs with both players here
                if clients.len() == 2 {
                    if let Some(winner) = game.winner {
                        if ticks_since_win == 0 {
                            println!("{} wins", winner.label());
                        }
                        ticks_since_win += 1;
                        if ticks_since_win >= RESTART_DELAY_TICKS {
                            println!("Starting a new match");
                            game = MatchState::new();
                            ticks_since_win = 0;
                        }
                    } else {
                        game.step(pending, &terrain);
                        clear_presses(&mut pending);
                    }
                }

                server_tick += 1;
                let snapshot = ServerMessage::Snapshot {
                    server_tick,
                    players: clients.len() as u8,
                    state: game.clone(),
                };
                broadcast(&socket, &clients, &snapshot).await;
            }

            result = socket.recv_from(&mut buffer) => {
                let (length, address) = match result {
                    Ok(received) => received,
                    Err(error) => {
                        // On Windows, sending to a client that has quit makes
                        // the next receive fail with "connection reset". That
                        // isn't fatal — keep serving everyone else.
                        if error.kind() != io::ErrorKind::ConnectionReset {
                            println!("Receive error: {error}");
                        }
                        continue;
                    }
                };

                let message = match from_bytes::<ClientMessage>(&buffer[..length]) {
                    Ok(message) => message,
                    Err(error) => {
                        println!("Unreadable message from {address}: {error}");
                        continue;
                    }
                };

                match message {
                    ClientMessage::Connect => {
                        // already joined — the reply may have been lost, so answer again
                        if let Some(client) = clients.get(&address) {
                            send(&socket, address, &ServerMessage::Connected { side: client.side }).await;
                            continue;
                        }

                        let Some(side) = free_side(&clients) else {
                            let reply = ServerMessage::Rejected { reason: "Server full".into() };
                            send(&socket, address, &reply).await;
                            continue;
                        };

                        clients.insert(address, Client {
                            side,
                            last_heard: Instant::now(),
                            last_seq: None,
                        });
                        println!("{} joined from {address} ({}/2)", side.label(), clients.len());
                        send(&socket, address, &ServerMessage::Connected { side }).await;
                        broadcast(&socket, &clients, &ServerMessage::PlayerJoined { side }).await;

                        // a new pairing starts a fresh match
                        if clients.len() == 2 {
                            println!("Both players here — starting the match");
                            game = MatchState::new();
                            pending = [Input::default(); 2];
                            ticks_since_win = 0;
                        }
                    }

                    ClientMessage::Input { seq, inputs } => {
                        let Some(client) = clients.get_mut(&address) else {
                            // not joined — most likely the server restarted
                            // under them. Saying so makes them rejoin.
                            let reply = ServerMessage::Rejected { reason: "Not connected".into() };
                            send(&socket, address, &reply).await;
                            continue;
                        };
                        client.last_heard = Instant::now();

                        // The packet holds the client's newest few inputs, oldest
                        // first, ending with number `seq`. Use only the ones not
                        // seen yet: a press in a lost packet still arrives in the
                        // next one, and no press is ever applied twice.
                        let count = inputs.len() as u32;
                        for (offset, input) in inputs.iter().enumerate() {
                            let input_seq = seq.wrapping_sub(count - 1 - offset as u32);
                            if client.last_seq.is_some_and(|last| input_seq <= last) {
                                continue;
                            }
                            merge(&mut pending[client.side.index()], input);
                            client.last_seq = Some(input_seq);
                        }
                    }
                }
            }
        }
    }
}

/// The first fighter nobody is controlling yet.
fn free_side(clients: &HashMap<SocketAddr, Client>) -> Option<PlayerSide> {
    [PlayerSide::Left, PlayerSide::Right]
        .into_iter()
        .find(|side| clients.values().all(|client| client.side != *side))
}

// Held keys follow the newest input; presses wait for a tick to use them.
fn merge(into: &mut Input, newer: &Input) {
    into.left = newer.left;
    into.right = newer.right;
    into.crouch = newer.crouch;
    into.jump |= newer.jump;
    into.attack |= newer.attack;
    into.block |= newer.block;
    into.throw |= newer.throw;
}

fn clear_presses(pending: &mut [Input; 2]) {
    for input in pending {
        input.jump = false;
        input.attack = false;
        input.block = false;
        input.throw = false;
    }
}

async fn send(socket: &UdpSocket, address: SocketAddr, message: &ServerMessage) {
    if let Ok(bytes) = to_allocvec(message) {
        let _ = socket.send_to(&bytes, address).await;
    }
}

async fn broadcast(socket: &UdpSocket, clients: &HashMap<SocketAddr, Client>, message: &ServerMessage) {
    let Ok(bytes) = to_allocvec(message) else {
        return;
    };
    for address in clients.keys() {
        let _ = socket.send_to(&bytes, *address).await;
    }
}