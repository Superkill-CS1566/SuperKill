use tokio::net::UdpSocket;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    let client_address = socket.local_addr()?;

    let server_address = "127.0.0.1:5000";
    let message = b"Connect";

    let bytes_sent = socket.send_to(message, server_address).await?;

    println!("Client {client_address} sent to {bytes_sent} bytes to {server_address}");

    Ok(())
}