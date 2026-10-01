use tokio::net::UdpSocket;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:5000").await?;
    let mut buf = [0; 1024];

    loop {
        let (len, addr) = socket.recv_from (&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);
        println!("Data: {}", String::from_utf8_lossy(&buf[..len]));
    }
}