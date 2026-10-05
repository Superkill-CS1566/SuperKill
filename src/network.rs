use bevy::prelude::*;
use tokio::net::UdpSocket;
use postcard::to_allocvec;
use crate::protocol::ClientMessage;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_network_client);
    }
}

fn start_network_client() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        runtime.block_on(async {
            println!("Tokio network runtime started");

            let socket = UdpSocket::bind("127.0.0.1:0").await
                .expect("Failed to bind client UDP socket");

            let server_address = "127.0.0.1:5000";

            let connect_msg = ClientMessage::Connect;
            let bytes = to_allocvec(&connect_msg).expect("Failed to serialize Connect");
            let bytes_sent = socket.send_to(&bytes, server_address).await
                .expect("Failed to send Connect message");

            println!("Bevy client sent {bytes_sent} bytes to {server_address}");

            let client_address = socket.local_addr()
                .expect("Failed to read client address");
            println!("Bevy client bound to {client_address}");
        });
    });
}
