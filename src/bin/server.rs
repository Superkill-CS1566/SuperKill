use tokio::net::UdpSocket;
use std::io;
use std::collections::HashMap;
use std::net::SocketAddr;
use postcard::{from_bytes, to_allocvec};

#[path = "../protocol.rs"]
mod protocol;
use protocol::{ClientMessage, ServerMessage};

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:5000").await?;
    let mut buf = [0; 1024];
    let mut players: HashMap<SocketAddr, u8> = HashMap::new();

    println!("Server listening on 0.0.0.0:5000");

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;

        let msg: ClientMessage = match from_bytes(&buf[..len]) {
            Ok(m) => m,
            Err(e) => {
                println!("Failed to parse message from {addr}: {e}");
                continue;
            }
        };

        match msg {
            ClientMessage::Connect => {
                if players.contains_key(&addr) {
                    let reply = ServerMessage::Rejected { reason: "Already connected".into() };
                    if let Ok(bytes) = to_allocvec(&reply) {
                        let _ = socket.send_to(&bytes, addr).await;
                    }
                    continue;
                }

                if players.len() >= 2 {
                    let reply = ServerMessage::Rejected { reason: "Server full".into() };
                    if let Ok(bytes) = to_allocvec(&reply) {
                        let _ = socket.send_to(&bytes, addr).await;
                    }
                    println!("Server full. Rejecting {addr}");
                    continue;
                }

                let player_id = players.len() as u8 + 1;
                players.insert(addr, player_id);

                println!("Registered {addr} as Player {player_id}");
                println!("Connected players: {}/2", players.len());

                let welcome = ServerMessage::Connected { player_id };
                if let Ok(bytes) = to_allocvec(&welcome) {
                    let _ = socket.send_to(&bytes, addr).await;
                }

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
        }
    }
}
