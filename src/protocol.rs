//! What the game and the server say to each other over UDP.
//!
//! Both sides encode these with postcard, which writes no field names — so
//! both programs must be built from the same version of this file.

use serde::{Deserialize, Serialize};

use crate::sim::{Input, MatchState, PlayerSide, TerrainKind};

/// The port the server listens on.
pub const SERVER_PORT: u16 = 5000;

/// The level online matches are played on. The server and every client
/// generate it from this and DEFAULT_SEED, so it never has to be sent.
pub const ONLINE_LEVEL: TerrainKind = TerrainKind::Noise;

/// How many recent inputs each input packet repeats, so a press still arrives
/// if a packet or two goes missing.
pub const INPUT_HISTORY: usize = 3;

// messages that a client sends to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// Asking to join. Repeated until the server answers.
    Connect,
    /// This client's most recent inputs, oldest first. The last one is input
    /// number `seq`, and the ones before it count back from there.
    Input { seq: u32, inputs: Vec<Input> },
}

// messages that the server sends to clients
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    /// You're in, controlling this fighter.
    Connected { side: PlayerSide },
    Rejected { reason: String },
    PlayerJoined { side: PlayerSide },
    PlayerLeft { side: PlayerSide },
    /// The whole match after one server tick, sent to everyone every tick.
    /// `server_tick` only ever counts up — even across matches — so a client
    /// can ignore any snapshot older than one it already has.
    Snapshot {
        server_tick: u32,
        /// How many players are connected, so a client can show that it's
        /// waiting for an opponent.
        players: u8,
        state: MatchState,
    },
}