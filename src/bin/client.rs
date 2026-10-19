use tokio::net::UdpSocket;
use std::io;
use postcard::{from_bytes, to_allocvec};

#[path = "../protocol.rs"]
mod protocol;
use protocol::{ClientMessage, PlayerInput, ServerMessage};

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

    // Test input to check if PlayerInput Works
    let input = PlayerInput {
        movement_x: 1.0, 
        movement_y: 0.0, 
        jump: false,
        crouch: false,
    };

    let input_message = ClientMessage::PlayerInput(input);

    let input_bytes = 
        to_allocvec(&input_message).expect("Failed to serialize player input");


    socket.send_to(&input_bytes, server_address).await?;
    println!("Sent test player input");

    Ok(())
}
