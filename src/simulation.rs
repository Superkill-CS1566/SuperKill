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
const JUMP_ATTACK_MIN_HEIGHT: f32 = 60.0;
const MAX_HEALTH: u8 = 2;
const HITSTUN_TICKS: u16 = 15;

// melee attack depending on player's stance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    Standing,
    Jumping,
    Crouching,
}

// specifications of attack
struct AttackSpec {
    damage: u8,
    active_ticks: u16,
    cooldown_ticks: u16,
    width: f32,
    height: f32,
    y_offset: f32,
}

// provide each attack's values, durations are measured in server ticks
impl AttackType {
    fn spec(self) -> AttackSpec {
        match self {
            AttackType::Standing => AttackSpec {
                damage: 1, active_ticks: 11, cooldown_ticks: 18,
                width: 55.0, height: 20.0, y_offset: 15.0,
            },
            AttackType::Jumping => AttackSpec {
                damage: 2, active_ticks: 18, cooldown_ticks: 30,
                width: 60.0, height: 50.0, y_offset: -25.0,
            },
            AttackType::Crouching => AttackSpec {
                damage: 1, active_ticks: 6, cooldown_ticks: 11,
                width: 70.0, height: 16.0, y_offset: -8.0,
            },
        }
    }
}

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
    pub health: u8,
    pub current_attack: Option<AttackType>,
    attack_ticks: u16,
    attack_cooldown_ticks: u16,
    attack_has_hit: bool,
    hitstun_ticks: u16,
}

// gameplay state of one player
impl PlayerState {

    // count down combat timers and end attacks
    fn advance_timers(&mut self) {
        self.attack_cooldown_ticks = self.attack_cooldown_ticks.saturating_sub(1);
        self.hitstun_ticks = self.hitstun_ticks.saturating_sub(1);
        self.attack_ticks = self.attack_ticks.saturating_sub(1);

        if self.attack_ticks == 0 {
            self.current_attack = None;
            self.attack_has_hit = false;
        }
    }

    // start an attack if the plyaer is able to act
    fn try_start_attack(&mut self, input: &PlayerInput) {
        if !input.attack
            || self.health == 0
            || self.hitstun_ticks > 0
            || self.attack_cooldown_ticks > 0
            || self.current_attack.is_some()
        {
            return;
        }

        let attack = if self.is_crouching {
            AttackType::Crouching
        } else if !self.grounded
            && self.position_y - (GROUND_TOP + STANDING_HEIGHT / 2.0)
                > JUMP_ATTACK_MIN_HEIGHT
        {
            AttackType::Jumping
        } else if self.grounded {
            AttackType::Standing
        } else {
            return;
        };

        let spec = attack.spec();
        self.current_attack = Some(attack);
        self.attack_ticks = spec.active_ticks;
        self.attack_cooldown_ticks = spec.cooldown_ticks;
        self.attack_has_hit = false;
    }

    // apply damage, interrupt the attack, and start hitstun if still alive
    fn take_hit(&mut self, damage: u8) {
        self.health = self.health.saturating_sub(damage);
        self.velocity_x = 0.0;
        self.current_attack = None;
        self.attack_ticks = 0;
        self.attack_has_hit = false;

        if self.health == 0 {
            self.velocity_y = 0.0;
            self.hitstun_ticks = 0;
        } else {
            self.hitstun_ticks = HITSTUN_TICKS;
        }
    }

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
            health: MAX_HEALTH,
            current_attack: None,
            attack_ticks: 0,
            attack_cooldown_ticks: 0,
            attack_has_hit: false,
            hitstun_ticks: 0,
        }
    }

    // advance this player by one fixed tick using their latest input
    pub fn step(&mut self, input: &PlayerInput) {

        if self.health == 0 {
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
            return;
        }

        let direction = if self.hitstun_ticks > 0 {
            0.0
        } else if input.movement_x.is_finite() {
            input.movement_x.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        if direction != 0.0 {
            self.facing = direction.signum();
        }

        if self.grounded {

            self.is_crouching = if self.current_attack == Some(AttackType::Crouching) {
                true
            } else {
                input.crouch && self.hitstun_ticks == 0 && self.current_attack.is_none()
            };

            let height = if self.is_crouching {
                CROUCH_HEIGHT
            } else {
                STANDING_HEIGHT
            };

            self.position_y = GROUND_TOP + height / 2.0;
        }

        let attack_speed = match self.current_attack {
            Some(AttackType::Jumping) => 0.7,
            Some(AttackType::Crouching) => 0.4,
            _ => 1.0,
        };
        let crouch_speed = if self.is_crouching { 0.5 } else { 1.0 };
        self.velocity_x = direction * PLAYER_SPEED * attack_speed * crouch_speed;
        self.position_x += self.velocity_x * TICK_SECONDS;
        self.position_x = self.position_x.clamp(
            -LEVEL_HALF_WIDTH + PLAYER_WIDTH / 2.0,
            LEVEL_HALF_WIDTH - PLAYER_WIDTH / 2.0,
        );

        if input.jump && self.grounded && !self.is_crouching && self.hitstun_ticks == 0 && self.current_attack.is_none() {
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
            health: self.health,
            is_attacking: self.current_attack.is_some(),
            is_dead: self.health == 0,
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

    // move both players, start attacks, then resolve both possible hits
    pub fn step(&mut self, inputs: [&PlayerInput; 2]) {
        for player in &mut self.players {
            player.advance_timers();
        }

        self.players[0].step(inputs[0]);
        self.players[1].step(inputs[1]);

        self.players[0].try_start_attack(inputs[0]);
        self.players[1].try_start_attack(inputs[1]);

        // Check both hits before applying damage, so simultaneous hits trade.
        let left_hit = self.attack_hits(0, 1);
        let right_hit = self.attack_hits(1, 0);

        if let Some(damage) = left_hit {
            self.players[0].attack_has_hit = true;
            self.players[1].take_hit(damage);
        }
        if let Some(damage) = right_hit {
            self.players[1].attack_has_hit = true;
            self.players[0].take_hit(damage);
        }
    }

    // return this attack's damage if its hitbox reaches target
    fn attack_hits(&self, attacker_index: usize, target_index: usize) -> Option<u8> {
        let attacker = &self.players[attacker_index];
        let target = &self.players[target_index];

        if attacker.health == 0 || target.health == 0 || attacker.attack_has_hit {
            return None;
        }

        let attack = attacker.current_attack?;

        // standing swing passes over a crouching player
        if attack == AttackType::Standing && target.is_crouching {
            return None;
        }

        // position the attack hitbox in front of the attacker
        let spec = attack.spec();
        let hitbox_x = attacker.position_x + attacker.facing * (PLAYER_WIDTH + spec.width) / 2.0;
        let hitbox_y = attacker.position_y + spec.y_offset;
        let target_height = if target.is_crouching {
            CROUCH_HEIGHT
        } else {
            STANDING_HEIGHT
        };

        // check whether the attack and player hitboxes overlap on both axes
        let overlaps_x = (hitbox_x - target.position_x).abs()
            <= (spec.width + PLAYER_WIDTH) / 2.0;
        let overlaps_y = (hitbox_y - target.position_y).abs()
            <= (spec.height + target_height) / 2.0;

        if overlaps_x && overlaps_y {
            Some(spec.damage)
        } else {
            None
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