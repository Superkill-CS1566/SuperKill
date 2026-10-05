use tokio::net::UdpSocket;
use std::io;
use std::collections::HashMap;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> io::Result<()> {

    let socket = UdpSocket::bind("0.0.0.0:5000").await?;
    let mut buf = [0; 1024];
    
    // Player storage
    let mut players: HashMap<SocketAddr, u8> = HashMap::new();

    loop {
        let (len, addr) = socket.recv_from (&mut buf).await?;

        if &buf[..len] != b"Connect" {
            println!("Ignoring unknown message from {addr}");
            continue;
        }

        // Prevent one address from registering twice
        if players.contains_key(&addr) {
            println!("{addr} is already registered");
            continue;
        }

        // Prevent more than two players from registering
        if players.len() >= 2 {
            println!("Server is full. Rejecting {addr}");
            continue;
        }

        // Assign new player ID and store
        let player_id = players.len() as u8 +1;
        players.insert(addr, player_id);

        println!("Registered {addr} as Player {player_id}");
        println!("Connected players: {}/2", players.len());
    }
}