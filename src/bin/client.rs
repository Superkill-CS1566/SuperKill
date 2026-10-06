use tokio::net::UdpSocket;
use std::io;
use postcard::{from_bytes, to_allocvec};

#[path = "../protocol.rs"]
mod protocol;
use protocol::{ClientMessage, ServerMessage};

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    let client_address = socket.local_addr()?;

    let server_address = "127.0.0.1:5000";

    let connect_msg = ClientMessage::Connect;
    let bytes = to_allocvec(&connect_msg).expect("Failed to serialize");
    let bytes_sent = socket.send_to(&bytes, server_address).await?;

    println!("Client {client_address} sent {bytes_sent} bytes to {server_address}");

    let mut buf = [0; 1024];
    let (len, _) = socket.recv_from(&mut buf).await?;

    let reply: ServerMessage = from_bytes(&buf[..len]).expect("Failed to deserialize");
    println!("Received from server: {reply:?}");

    Ok(())
}
