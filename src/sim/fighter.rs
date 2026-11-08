//! One fighter: everything their own input causes, for a single tick.
//!
//! `FighterState::step` touches only this fighter and the terrain — never the
//! other fighter, swords, or the arena. That boundary is what makes client-side
//! prediction possible: clone a fighter, call `step` once per unconfirmed
//! input, and you've reproduced exactly what the server will compute.

use serde::{Deserialize, Serialize};

use super::terrain::Terrain;
use super::{ticks, Input, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN, TICK_SECONDS};

// ---------------------------------------------------------------- body

pub const PLAYER_WIDTH: f32 = 30.0;
pub const PLAYER_HEIGHT: f32 = 95.0;
pub const CROUCH_HEIGHT: f32 = 35.0;
pub const PLAYER_SPEED: f32 = 320.0;
pub const JUMP_FORCE: f32 = 620.0;
pub const MAX_HEALTH: u8 = 2;

const GRAVITY: f32 = 1800.0;
const TERMINAL_VELOCITY: f32 = -1200.0;

/// Below this a fighter has fallen out of the level and goes back to their own
/// spawn point. Matches the old `-WIN_H / 2.0 - 100.0`.
const KILL_PLANE_Y: f32 = -460.0;

// how far from center each fighter starts
const SPAWN_OFFSET_X: f32 = 200.0;
pub const PLAYER_SPAWN_Y: f32 = GROUND_Y + GROUND_THICKNESS / 2.0 + PLAYER_HEIGHT / 2.0;

/// The block box, for drawing. Blocking itself only cares which way the
/// fighter faces, not this box.
pub const BLOCK_W: f32 = 25.0;
pub const BLOCK_H: f32 = 52.0;

// ---------------------------------------------------------------- timing
// Tuned in seconds, counted in ticks.

pub const HITSTUN_TICKS: u16 = ticks(0.25);

const BLOCK_TICKS: u16 = ticks(0.6);
/// Before a fighter's first block, at spawn.
const BLOCK_COOLDOWN_SPAWN_TICKS: u16 = ticks(1.5);
/// After each block ends. The old code hard-coded 2.0 here while using
/// BLOCK_COOLDOWN (1.5) at spawn — kept as it was.
const BLOCK_COOLDOWN_AFTER_TICKS: u16 = ticks(2.0);

/// A drop needs this much airtime first.
const DROP_MIN_AIR_TICKS: u16 = ticks(0.22);
/// Safety cap on a drop's hitbox; it normally ends on landing well before this.
const DROP_MAX_TICKS: u16 = ticks(1.5);
const DROP_LANDING_LAG_TICKS: u16 = ticks(0.35);
pub const DROP_SPEED: f32 = -1100.0;

/// The throw animation runs 6 frames of this length, and the sword leaves the
/// hand on the 5th (index 4).
pub const THROW_FRAME_SECONDS: f32 = 0.06;
const THROW_RELEASE_TICKS: u16 = ticks(THROW_FRAME_SECONDS * 4.0);
const THROW_END_TICKS: u16 = ticks(THROW_FRAME_SECONDS * 6.0);

// ---------------------------------------------------------------- sides

// which fighter this is — left starts on the left, right on the right
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerSide {
    Left,
    Right,
}

impl PlayerSide {
    /// Position in `MatchState::fighters`.
    pub fn index(self) -> usize {
        match self {
            PlayerSide::Left => 0,
            PlayerSide::Right => 1,
        }
    }

    pub fn from_player_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(PlayerSide::Left),
            2 => Some(PlayerSide::Right),
            _ => None,
        }
    }

    pub fn other(self) -> Self {
        match self {
            PlayerSide::Left => PlayerSide::Right,
            PlayerSide::Right => PlayerSide::Left,
        }
    }

    // left fighter wins by pushing right, right fighter by pushing left
    pub fn advance_dir(self) -> f32 {
        match self {
            PlayerSide::Left => 1.0,
            PlayerSide::Right => -1.0,
        }
    }

    pub fn spawn_x(self) -> f32 {
        match self {
            PlayerSide::Left => -SPAWN_OFFSET_X,
            PlayerSide::Right => SPAWN_OFFSET_X,
        }
    }

    // Label for win screen
    pub fn label(self) -> &'static str {
        match self {
            PlayerSide::Left => "Player 1",
            PlayerSide::Right => "Player 2",
        }
    }
}

// ---------------------------------------------------------------- attacks

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackType {
    Standing,
    /// The drop attack.
    Jumping,
    Crouching,
}

/// Everything that differs between attacks, in one place. This replaces the
/// constants that were spread through combat.rs.
pub struct AttackSpec {
    pub damage: u8,
    /// How long the hitbox is out. For the drop, a safety cap — it ends on landing.
    pub active_ticks: u16,
    /// Time from the start of this attack until another can begin.
    pub cooldown_ticks: u16,
    /// Hitbox (width, height) holding the sword, and empty-handed.
    pub armed_size: (f32, f32),
    pub unarmed_size: (f32, f32),
    /// Vertical position of the hitbox relative to the fighter's center.
    pub y_offset: f32,
}

impl AttackType {
    pub fn spec(self) -> AttackSpec {
        match self {
            AttackType::Standing => AttackSpec {
                damage: 1,
                active_ticks: ticks(0.18),
                cooldown_ticks: ticks(0.30),
                armed_size: (55.0, 20.0),
                unarmed_size: (28.0, 20.0),
                y_offset: 15.0,
            },
            AttackType::Jumping => AttackSpec {
                damage: 2,
                active_ticks: DROP_MAX_TICKS,
                cooldown_ticks: ticks(0.50),
                armed_size: (60.0, 50.0),
                unarmed_size: (36.0, 30.0),
                y_offset: -25.0,
            },
            AttackType::Crouching => AttackSpec {
                damage: 1,
                active_ticks: ticks(0.10),
                cooldown_ticks: ticks(0.18),
                armed_size: (70.0, 16.0),
                unarmed_size: (35.0, 16.0),
                y_offset: -8.0,
            },
        }
    }
}

/// A rectangle by center and size, used for hitboxes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn overlaps(&self, other: &Rect) -> bool {
        (self.x - other.x).abs() <= (self.width + other.width) / 2.0
            && (self.y - other.y).abs() <= (self.height + other.height) / 2.0
    }
}

// ---------------------------------------------------------------- state

/// Present while a fighter is down. Records where the killer stood at the
/// moment of the kill, so respawn can tell how far the leader has pushed since.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Death {
    pub killer_x: f32,
    pub ticks_down: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FighterState {
    pub side: PlayerSide,

    /// Center of the collision box — where the old `Transform` was.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// -1.0 facing left, 1.0 facing right.
    pub facing: f32,
    pub grounded: bool,
    pub crouching: bool,

    pub health: u8,
    pub has_weapon: bool,

    pub attack: Option<AttackType>,
    /// Ticks the current attack's hitbox has left.
    pub attack_ticks: u16,
    /// Whether the fighter held the sword when the attack started — decides
    /// the hitbox size for the whole swing.
    pub attack_armed: bool,
    /// An attack lands at most once.
    pub attack_has_hit: bool,
    pub attack_cooldown: u16,

    pub blocking: bool,
    pub block_ticks: u16,
    pub block_cooldown: u16,

    pub hitstun: u16,
    /// Ticks since leaving the ground. A drop needs enough of it.
    pub air_ticks: u16,
    /// Mid drop attack — being pulled straight down.
    pub dropping: bool,
    /// The beat after a drop lands, when the fighter can't act.
    pub landing_lag: u16,

    /// Ticks since the throw started, while one is in progress.
    pub throw_ticks: Option<u16>,
    /// True for exactly the tick the sword leaves the hand. The swords code
    /// reads it to launch the sword, since `step` can't touch anything but
    /// this fighter.
    pub threw_this_tick: bool,

    pub death: Option<Death>,
}

impl FighterState {
    pub fn new(side: PlayerSide) -> Self {
        Self {
            side,
            x: side.spawn_x(),
            y: PLAYER_SPAWN_Y,
            vx: 0.0,
            vy: 0.0,
            facing: side.advance_dir(),
            grounded: false,
            crouching: false,
            health: MAX_HEALTH,
            has_weapon: true,
            attack: None,
            attack_ticks: 0,
            attack_armed: true,
            attack_has_hit: false,
            attack_cooldown: 0,
            blocking: false,
            block_ticks: 0,
            block_cooldown: BLOCK_COOLDOWN_SPAWN_TICKS,
            hitstun: 0,
            air_ticks: 0,
            dropping: false,
            landing_lag: 0,
            throw_ticks: None,
            threw_this_tick: false,
            death: None,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.death.is_some()
    }

    /// Can't act right now — recoiling from a hit, or stuck landing a drop.
    pub fn is_stunned(&self) -> bool {
        self.hitstun > 0 || self.landing_lag > 0
    }

    /// Height of the collision box right now.
    pub fn height(&self) -> f32 {
        if self.crouching {
            CROUCH_HEIGHT
        } else {
            PLAYER_HEIGHT
        }
    }

    /// The collision box.
    pub fn body(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            width: PLAYER_WIDTH,
            height: self.height(),
        }
    }

    /// The active attack's hitbox, computed from this fighter's state rather
    /// than stored as an entity — so it can never outlive the attack.
    pub fn attack_hitbox(&self) -> Option<Rect> {
        let attack = self.attack?;
        let spec = attack.spec();
        let (width, height) = if self.attack_armed {
            spec.armed_size
        } else {
            spec.unarmed_size
        };

        // a drop strikes whatever is directly underneath; every other attack
        // sits in front of the fighter
        let x = if attack == AttackType::Jumping {
            self.x
        } else {
            self.x + self.facing * (PLAYER_WIDTH + width) / 2.0
        };

        Some(Rect {
            x,
            y: self.y + spec.y_offset,
            width,
            height,
        })
    }

    /// Where the block box sits, while blocking. For drawing only.
    pub fn block_box(&self) -> Option<Rect> {
        if !self.blocking {
            return None;
        }
        Some(Rect {
            x: self.x + self.facing * (PLAYER_WIDTH + BLOCK_W) / 2.0,
            y: self.y,
            width: BLOCK_W,
            height: BLOCK_H,
        })
    }

    /// Which frame of the throw animation to show, while throwing.
    pub fn throw_frame(&self) -> Option<usize> {
        let elapsed = self.throw_ticks?;
        Some((elapsed as f32 * TICK_SECONDS / THROW_FRAME_SECONDS) as usize)
    }

    /// Bring a downed fighter back at `x`, armed and at full health. Resets the
    /// same things the old `check_respawn` did.
    pub fn respawn_at(&mut self, x: f32) {
        self.x = x;
        self.y = PLAYER_SPAWN_Y;
        self.vx = 0.0;
        self.vy = 0.0;
        self.has_weapon = true;
        self.grounded = false;
        self.health = MAX_HEALTH;
        self.attack_cooldown = 0;
        self.blocking = false;
        self.hitstun = 0;
        self.attack = None;
        // clear any drop that was interrupted by dying
        self.air_ticks = 0;
        self.dropping = false;
        self.landing_lag = 0;
        self.death = None;
    }

    // ------------------------------------------------------------ the tick

    /// Advance this fighter one tick from their own input. Dead fighters don't
    /// move — bringing them back is the arena's job.
    pub fn step(&mut self, input: &Input, terrain: &Terrain) {
        self.threw_this_tick = false;
        if self.is_dead() {
            return;
        }

        self.tick_timers();

        let stunned = self.is_stunned();
        let attacking = self.attack.is_some();

        // ---- crouching
        let want_crouch = input.crouch && !stunned && !attacking;
        if self.grounded {
            // the box shrinks from the top, so move the center to keep the
            // feet where they were
            let half_delta = (PLAYER_HEIGHT - CROUCH_HEIGHT) / 2.0;
            if self.crouching && !want_crouch {
                self.y += half_delta;
            } else if !self.crouching && want_crouch {
                self.y -= half_delta;
            }
            self.crouching = want_crouch;
        }

        // ---- walking
        let mut move_dir = 0.0;
        if !stunned && !self.blocking {
            if input.left {
                move_dir -= 1.0;
            }
            if input.right {
                move_dir += 1.0;
            }
        }
        if move_dir != 0.0 {
            self.facing = move_dir;
        }

        let attack_mult = if self.attack == Some(AttackType::Crouching) {
            0.4
        } else {
            1.0
        };
        // 下蹲时移动速度减半
        let crouch_mult = if self.crouching { 0.5 } else { 1.0 };
        // a drop keeps the horizontal speed it started with until it lands
        if !self.dropping {
            self.vx = move_dir * PLAYER_SPEED * attack_mult * crouch_mult;
        }

        // ---- jumping
        if input.jump && self.grounded && !self.crouching && !stunned && !attacking {
            self.vy = JUMP_FORCE;
            self.grounded = false;
        }

        // ---- starting actions
        self.try_start_attack(input, stunned);
        self.try_start_block(input, stunned);
        self.try_start_throw(input);

        // ---- gravity
        if self.grounded {
            self.air_ticks = 0;
        } else {
            self.air_ticks = self.air_ticks.saturating_add(1);
            if self.dropping {
                // a drop pulls straight down instead of arcing
                self.vy = DROP_SPEED;
            } else {
                // 终端速度限制
                self.vy = (self.vy - GRAVITY * TICK_SECONDS).max(TERMINAL_VELOCITY);
            }
        }

        // ---- moving
        self.move_and_collide(terrain);

        // a drop ends the moment the fighter touches down
        if self.dropping && self.grounded {
            self.dropping = false;
            self.landing_lag = DROP_LANDING_LAG_TICKS;
        }

        self.advance_throw();
    }

    fn tick_timers(&mut self) {
        self.attack_cooldown = self.attack_cooldown.saturating_sub(1);
        self.hitstun = self.hitstun.saturating_sub(1);
        self.landing_lag = self.landing_lag.saturating_sub(1);
        self.block_cooldown = self.block_cooldown.saturating_sub(1);

        // An attack runs out — or, for a drop, ends once its fighter has
        // landed. Ending it here at the start of the next tick, rather than the
        // instant they land, gives the drop one hit check at its landing spot.
        if let Some(attack) = self.attack {
            let drop_over = attack == AttackType::Jumping && !self.dropping;
            self.attack_ticks = self.attack_ticks.saturating_sub(1);
            if drop_over || self.attack_ticks == 0 {
                self.attack = None;
            }
        }

        // a block runs out, then can't be raised again for a while
        if self.blocking {
            self.block_ticks = self.block_ticks.saturating_sub(1);
            if self.block_ticks == 0 {
                self.blocking = false;
                self.block_cooldown = BLOCK_COOLDOWN_AFTER_TICKS;
            }
        }
    }

    fn try_start_attack(&mut self, input: &Input, stunned: bool) {
        if !input.attack
            || self.attack_cooldown > 0
            || stunned
            || self.blocking
            || self.attack.is_some()
        {
            return;
        }

        // airtime rather than height above the ground, so it works on hills
        let attack = if !self.grounded && self.air_ticks >= DROP_MIN_AIR_TICKS {
            AttackType::Jumping
        } else if self.crouching {
            AttackType::Crouching
        } else if self.grounded {
            AttackType::Standing
        } else {
            return; // airborne, but not long enough for a drop yet
        };

        let spec = attack.spec();
        self.attack = Some(attack);
        self.attack_ticks = spec.active_ticks;
        self.attack_cooldown = spec.cooldown_ticks;
        self.attack_armed = self.has_weapon;
        self.attack_has_hit = false;

        if attack == AttackType::Jumping {
            // start the plunge now rather than next tick
            self.dropping = true;
            self.vy = DROP_SPEED;
        }
    }

    fn try_start_block(&mut self, input: &Input, stunned: bool) {
        if !input.block
            || self.blocking
            || self.attack.is_some()
            || self.block_cooldown > 0
            || stunned
        {
            return;
        }
        self.blocking = true;
        self.block_ticks = BLOCK_TICKS;
    }

    // One button does both: throw if armed, pick up if not. Picking up involves
    // a weapon on the ground, so it's handled by the swords code instead.
    fn try_start_throw(&mut self, input: &Input) {
        if !input.throw || !self.has_weapon || self.crouching || self.throw_ticks.is_some() {
            return;
        }
        self.throw_ticks = Some(0);
    }

    // The sword leaves the hand partway through the wind-up.
    fn advance_throw(&mut self) {
        let Some(elapsed) = self.throw_ticks else {
            return;
        };
        let elapsed = elapsed + 1;

        if elapsed == THROW_RELEASE_TICKS {
            self.has_weapon = false;
            self.threw_this_tick = true;
        }
        self.throw_ticks = if elapsed >= THROW_END_TICKS {
            None
        } else {
            Some(elapsed)
        };
    }

    // Moved unchanged from move_and_collide in player.rs.
    fn move_and_collide(&mut self, terrain: &Terrain) {
        let half_w = PLAYER_WIDTH / 2.0;
        let half_h = self.height() / 2.0;

        // ---- horizontal: step, then push out of anything walked into
        let mut new_x = self.x + self.vx * TICK_SECONDS;
        new_x = new_x.clamp(-LEVEL_LEN / 2.0 + half_w, LEVEL_LEN / 2.0 - half_w);

        for block in &terrain.blocks {
            let overlaps_x =
                (new_x - half_w) <= block.right() && (new_x + half_w) >= block.left();
            let overlaps_y =
                (self.y - half_h) < block.top() && (self.y + half_h) > block.bottom();

            if overlaps_x && overlaps_y {
                if self.x < block.x {
                    new_x = block.left() - half_w; // 从左侧撞，推到平台左边
                } else {
                    new_x = block.right() + half_w; // 从右侧撞，推到平台右边
                }
                self.vx = 0.0;
            }
        }
        self.x = new_x;

        // ---- vertical: step, then land on or bump against blocks
        let mut new_y = self.y + self.vy * TICK_SECONDS;
        self.grounded = false;

        for block in &terrain.blocks {
            let overlaps_x =
                (self.x - half_w) <= block.right() && (self.x + half_w) >= block.left();
            let overlaps_y =
                (new_y - half_h) <= block.top() && (new_y + half_h) >= block.bottom();

            if overlaps_x && overlaps_y {
                if self.vy <= 0.0 && self.y >= block.y {
                    new_y = block.top() + half_h;
                    self.vy = 0.0;
                    self.grounded = true;
                } else if self.vy > 0.0 && self.y < block.y {
                    new_y = block.bottom() - half_h;
                    self.vy = 0.0;
                }
            }
        }

        // Respawn Logic — fell out of the level, back to this fighter's own side
        if new_y < KILL_PLANE_Y {
            new_y = PLAYER_SPAWN_Y;
            self.x = self.side.spawn_x();
            self.vx = 0.0;
            self.vy = 0.0;
        }
        self.y = new_y;
    }
}

// ---------------------------------------------------------------- tests
// Run with `cargo test`. These exercise the rules without launching the game.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::terrain::Block;
    use crate::sim::DEFAULT_SEED;

    fn run(fighter: &mut FighterState, input: Input, terrain: &Terrain, ticks: usize) {
        for _ in 0..ticks {
            fighter.step(&input, terrain);
        }
    }

    fn landed(side: PlayerSide, terrain: &Terrain) -> FighterState {
        let mut fighter = FighterState::new(side);
        run(&mut fighter, Input::default(), terrain, 10);
        fighter
    }

    #[test]
    fn settles_onto_flat_ground() {
        let terrain = Terrain::flat();
        let fighter = landed(PlayerSide::Left, &terrain);

        let feet = fighter.y - PLAYER_HEIGHT / 2.0;
        let floor_top = GROUND_Y + GROUND_THICKNESS / 2.0;
        assert!(fighter.grounded);
        assert!((feet - floor_top).abs() < 0.01);
    }

    #[test]
    fn walks_at_player_speed() {
        let terrain = Terrain::flat();
        let mut fighter = landed(PlayerSide::Left, &terrain);
        let start = fighter.x;

        let right = Input { right: true, ..Input::default() };
        run(&mut fighter, right, &terrain, 60); // one second

        assert!((fighter.x - start - PLAYER_SPEED).abs() < 1.0);
        assert_eq!(fighter.facing, 1.0);
    }

    #[test]
    fn jump_comes_back_down() {
        let terrain = Terrain::flat();
        let mut fighter = landed(PlayerSide::Left, &terrain);

        fighter.step(&Input { jump: true, ..Input::default() }, &terrain);
        assert!(!fighter.grounded);

        run(&mut fighter, Input::default(), &terrain, 120);
        assert!(fighter.grounded);
    }

    #[test]
    fn crouching_keeps_the_feet_in_place() {
        let terrain = Terrain::flat();
        let mut fighter = landed(PlayerSide::Left, &terrain);
        let feet_standing = fighter.y - fighter.height() / 2.0;

        fighter.step(&Input { crouch: true, ..Input::default() }, &terrain);
        let feet_crouching = fighter.y - fighter.height() / 2.0;

        assert!(fighter.crouching);
        assert!((feet_standing - feet_crouching).abs() < 0.01);
    }

    #[test]
    fn sword_leaves_the_hand_mid_throw() {
        let terrain = Terrain::flat();
        let mut fighter = landed(PlayerSide::Left, &terrain);

        fighter.step(&Input { throw: true, ..Input::default() }, &terrain);
        let mut released_on = None;
        for tick in 1..=30 {
            fighter.step(&Input::default(), &terrain);
            if fighter.threw_this_tick {
                released_on = Some(tick);
            }
        }

        assert!(!fighter.has_weapon);
        assert!(released_on.is_some());
        assert!(fighter.throw_ticks.is_none()); // the wind-up finished
    }

    #[test]
    fn drop_attack_ends_on_landing() {
        let terrain = Terrain::flat();
        let mut fighter = landed(PlayerSide::Left, &terrain);

        fighter.step(&Input { jump: true, ..Input::default() }, &terrain);
        run(&mut fighter, Input::default(), &terrain, 15); // enough airtime
        fighter.step(&Input { attack: true, ..Input::default() }, &terrain);
        assert!(fighter.dropping);
        assert_eq!(fighter.attack, Some(AttackType::Jumping));

        run(&mut fighter, Input::default(), &terrain, 60);
        assert!(fighter.grounded);
        assert!(!fighter.dropping);
        assert_eq!(fighter.attack, None);
    }

    /// Ported from the old platform_test program: each of three steps must be
    /// reachable with a running jump from either side, landing fully on top.
    #[test]
    fn every_test_platform_is_reachable_from_both_sides() {
        let ground_top = GROUND_Y + GROUND_THICKNESS / 2.0;
        // (center x, width, height above the ground)
        let platforms = [(-360.0, 160.0, 50.0), (0.0, 200.0, 65.0), (360.0, 180.0, 80.0)];

        let mut blocks = vec![Block {
            x: 0.0,
            y: GROUND_Y,
            width: LEVEL_LEN,
            height: GROUND_THICKNESS,
        }];
        for &(x, width, height) in &platforms {
            blocks.push(Block {
                x,
                y: ground_top + height / 2.0,
                width,
                height,
            });
        }
        let terrain = Terrain { blocks };

        for &(x, width, height) in &platforms {
            for direction in [1.0, -1.0] {
                // standing on the ground, 100 units short of the near edge
                let mut fighter = FighterState::new(PlayerSide::Left);
                fighter.x = x - direction * (width / 2.0 + 100.0);
                fighter.grounded = true;

                let running = Input {
                    left: direction < 0.0,
                    right: direction > 0.0,
                    ..Input::default()
                };
                let running_jump = Input { jump: true, ..running };

                // two seconds to land, as the old test allowed
                let mut landed = false;
                for tick in 0..120 {
                    let input = if tick == 0 { &running_jump } else { &running };
                    fighter.step(input, &terrain);

                    let feet = fighter.y - fighter.height() / 2.0;
                    let fully_on_top =
                        (fighter.x - x).abs() + PLAYER_WIDTH / 2.0 < width / 2.0 - 1.0;
                    if fighter.grounded && fully_on_top && (feet - (ground_top + height)).abs() < 0.5
                    {
                        landed = true;
                        break;
                    }
                }
                assert!(
                    landed,
                    "the {height}-tall platform at x = {x} isn't reachable moving {direction}"
                );
            }
        }
    }

    /// The property client-side prediction depends on: the same starting state
    /// and the same inputs always produce the same result.
    #[test]
    fn replaying_the_same_inputs_gives_the_same_state() {
        let terrain = Terrain::noise(DEFAULT_SEED);
        let right = Input { right: true, ..Input::default() };
        let jump = Input { jump: true, right: true, ..Input::default() };
        let inputs = [right, right, jump, right, right, right, right, right];

        let mut original = FighterState::new(PlayerSide::Left);
        let mut replay = original.clone();
        for input in &inputs {
            original.step(input, &terrain);
        }
        for input in &inputs {
            replay.step(input, &terrain);
        }

        assert_eq!(original, replay);
    }
}