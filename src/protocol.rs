use serde::{Deserialize, Serialize};

// stores one player's current state sent by the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSnapshot {
    pub player_id: u8, 
    pub position_x: f32, 
    pub position_y: f32,
    pub is_crouching: bool,
}

// stores movement controls sent by client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInput {
    pub movement_x: f32,
    pub movement_y: f32, 
    pub jump: bool,
    pub crouch: bool,
}

// messages that a client sends to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Connect,
    Heartbeat,
    PlayerInput(PlayerInput),
}

// messages that the server sends to clients
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Connected { player_id: u8 },
    Rejected { reason: String },
    PlayerJoined { player_id: u8 },

    // contains the latest state of every connected player
    GameState {
        players: Vec<PlayerSnapshot>
    }
}
