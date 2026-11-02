use crate::protocol::{PlayerInput, PlayerSnapshot};

// shared timing and flat-level boundaries for simulation
pub const TICK_SECONDS: f32 = 1.0 / 60.0;
pub const LEVEL_HALF_WIDTH: f32 = 960.0;
pub const GROUND_TOP: f32 = -270.0;

const PLAYER_WIDTH: f32 = 30.0;
const STANDING_HEIGHT: f32 = 95.0;
const CROUCH_HEIGHT: f32 = 35.0;
const PLAYER_SPEED: f32 = 320.0;
const JUMP_FORCE: f32 = 620.0;
const GRAVITY: f32 = 1800.0;
const TERMINAL_VELOCITY: f32 = -1200.0;

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub player_id: u8,
    pub position_x: f32,
    pub position_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub facing: f32,
    pub grounded: bool,
    pub is_crouching: bool,
}

// gameplay state of one player
impl PlayerState {

    // place player at their starting position on the flat ground
    pub fn new(player_id: u8) -> Self {
        assert!(player_id == 1 || player_id == 2);

        Self {
            player_id,
            position_x: if player_id == 1 { -200.0 } else { 200.0 },
            position_y: GROUND_TOP + STANDING_HEIGHT / 2.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            facing: if player_id == 1 { 1.0 } else { -1.0 },
            grounded: true,
            is_crouching: false,
        }
    }

    // advance this player by one fixed tick using their latest input
    pub fn step(&mut self, input: &PlayerInput) {
        let direction = if input.movement_x.is_finite() {
            input.movement_x.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        if direction != 0.0 {
            self.facing = direction.signum();
        }

        if self.grounded {
            self.is_crouching = input.crouch;
            let height = if self.is_crouching {
                CROUCH_HEIGHT
            } else {
                STANDING_HEIGHT
            };
            self.position_y = GROUND_TOP + height / 2.0;
        }

        let speed = if self.is_crouching {
            PLAYER_SPEED * 0.5
        } else {
            PLAYER_SPEED
        };
        self.velocity_x = direction * speed;
        self.position_x += self.velocity_x * TICK_SECONDS;
        self.position_x = self.position_x.clamp(
            -LEVEL_HALF_WIDTH + PLAYER_WIDTH / 2.0,
            LEVEL_HALF_WIDTH - PLAYER_WIDTH / 2.0,
        );

        if input.jump && self.grounded && !self.is_crouching {
            self.velocity_y = JUMP_FORCE;
            self.grounded = false;
        }

        if !self.grounded {
            self.velocity_y =
                (self.velocity_y - GRAVITY * TICK_SECONDS).max(TERMINAL_VELOCITY);
            self.position_y += self.velocity_y * TICK_SECONDS;

            let floor_position = GROUND_TOP + STANDING_HEIGHT / 2.0;
            if self.position_y <= floor_position {
                self.position_y = floor_position;
                self.velocity_y = 0.0;
                self.grounded = true;
            }
        }
    }

    // convert the player's state into a message that server can send
    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            player_id: self.player_id,
            position_x: self.position_x,
            position_y: self.position_y,
            velocity_x: self.velocity_x,
            velocity_y: self.velocity_y,
            is_grounded: self.grounded,
            facing: self.facing,
            is_crouching: self.is_crouching,
        }
    }
}

#[derive(Debug, Clone)]
// hold the authoriative gameplay state for both players
pub struct MatchState {
    pub players: [PlayerState; 2],
}

impl MatchState {
    pub fn new() -> Self {
        Self {
            players: [PlayerState::new(1), PlayerState::new(2)],
        }
    }

    // advance both players by one fixed tick
    pub fn step(&mut self, inputs: [&PlayerInput; 2]) {
        for (player, input) in self.players.iter_mut().zip(inputs) {
            player.step(input);
        }
    }

    // collect both player states for a server update
    pub fn snapshots(&self) -> Vec<PlayerSnapshot> {
        self.players.iter().map(PlayerState::snapshot).collect()
    }
}

impl Default for MatchState {
    fn default() -> Self {
        Self::new()
    }
}