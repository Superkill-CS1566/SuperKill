use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Connect,
    Heartbeat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Connected { player_id: u8 },
    Rejected { reason: String },
    PlayerJoined { player_id: u8 },
}
