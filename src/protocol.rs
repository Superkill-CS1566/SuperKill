use serde::{Deserialize, Serialize};

// Define player input
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInput {
    pub movement_x: f32,
    pub movement_y: f32, 
    pub jump: bool,
    pub block: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Connect,
    Heartbeat,
    PlayerInput(PlayerInput),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Connected { player_id: u8 },
    Rejected { reason: String },
    PlayerJoined { player_id: u8 },
}
