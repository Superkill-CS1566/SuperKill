use tokio::net::UdpSocket;
use std::io;
use std::collections::HashMap;
use std::net::SocketAddr;
use postcard::{from_bytes, to_allocvec};

// import the shared network messages and gameplay simulation
use superkill::{
    protocol::{ClientMessage, ServerMessage},
    simulation::MatchState,
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

                    // update this player using the shared simulation
                    game.players[(player_id - 1) as usize].step(&input);

                    // broadcast the updated state of both players to every client
                    let update = ServerMessage::GameState {
                        players: game.snapshots(),
                    };

                    if let Ok(bytes) = to_allocvec(&update) {
                        for &client_addr in players.keys() {
                            let _ = socket.send_to(&bytes, client_addr).await;
                        }
                    }
                } else {
                    println!("Ignoring input from unregistered client {addr}");
                }
            }
        }
    }
}
