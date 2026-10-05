use bevy::prelude::*;
use tokio::net::UdpSocket;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_network_client);
    }
}

fn start_network_client() {

    std::thread::spawn(|| {
        
        // Starts tokio runtime
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Failed to create Tokio runtime");
        
        // Runs asynchronous block from background thread
        runtime.block_on(async {

            println!("Tokio network runtime started");

            // Bind UDP socket
            let socket = UdpSocket::bind("127.0.0.1:0").await.expect("Failed to bind client UDP socket");

            // Connect to server
            let server_address = "127.0.0.1:5000";

            let bytes_sent = socket.send_to(b"Connect", server_address).await.expect("Failed to send Connect message");
            println!("Bevy client sent {bytes_sent} bytes to {server_address}");

            let client_address = socket.local_addr().expect("Failed to read client address");

            println!("Bevy client bound to {client_address}");
        });
    });
}