//! The game's rules, in plain Rust — no Bevy anywhere in here.
//!
//! Everything that decides what *happens* lives in this module: movement,
//! combat, swords, the arena. Bevy only draws the result. Local play, the
//! server, and client-side prediction all run this same code, which is what
//! keeps them from drifting apart.

pub mod arena;
pub mod combat;
pub mod fighter;
pub mod terrain;
pub mod weapons;

use serde::{Deserialize, Serialize};

pub use arena::{ARENA_WIDTH, SCREEN_COUNT, START_SCREEN};
pub use combat::Kill;
pub use fighter::{AttackType, Death, FighterState, PlayerSide, Rect};
pub use terrain::{Block, Terrain, TerrainKind, DEFAULT_SEED};
pub use weapons::{GroundWeapon, ThrownSword};

/// The fixed timestep every rule advances by. The server and the client both
/// step at exactly this rate, so the same inputs always give the same result.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;

/// Converts seconds to whole ticks, rounding to the nearest. Lets the tuning
/// stay in seconds while the rules count ticks.
pub const fn ticks(seconds: f32) -> u16 {
    (seconds / TICK_SECONDS + 0.5) as u16
}

// Level geometry. common.rs re-exports these, so there is exactly one copy.
pub const LEVEL_LEN: f32 = 1920.0;
pub const GROUND_Y: f32 = -300.0; // center of the ground platform
pub const GROUND_THICKNESS: f32 = 60.0;

/// What one fighter's controls are doing this tick.
///
/// The one-shot buttons — jump, attack, block, throw — must be true for
/// exactly one tick per press. Whoever fills this in (the keyboard on the
/// client, packets on the server) is responsible for latching them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub crouch: bool,
    pub jump: bool,
    pub attack: bool,
    pub block: bool,
    pub throw: bool,
}

/// Everything about a match that changes while it's played.
///
/// The terrain isn't in here: it never changes mid-match, so it's passed to
/// `step` by reference instead. That keeps cloning a `MatchState` cheap, which
/// matters once rewind starts keeping a history of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchState {
    pub tick: u32,
    /// Indexed by `PlayerSide::index()` — the left fighter first.
    pub fighters: [FighterState; 2],
    pub swords: Vec<ThrownSword>,
    pub ground_weapons: Vec<GroundWeapon>,

    /// Who owns the camera and may advance screens. None means neutral.
    pub priority: Option<PlayerSide>,
    pub current_screen: usize,
    /// True for the one tick a screen transition happens. It's the respawn
    /// checkpoint, and it tells the camera to cut rather than pan.
    pub screen_changed_this_tick: bool,
    /// Center of the play area: where the camera heads, and what the crush
    /// edge and respawns measure from.
    pub arena_center: f32,
    pub winner: Option<PlayerSide>,

    /// Gives swords and ground weapons stable ids, so the view — and later the
    /// network — can tell which one is which from tick to tick.
    next_object_id: u32,
}

impl MatchState {
    pub fn new() -> Self {
        Self {
            tick: 0,
            fighters: [
                FighterState::new(PlayerSide::Left),
                FighterState::new(PlayerSide::Right),
            ],
            swords: Vec::new(),
            ground_weapons: Vec::new(),
            priority: None,
            current_screen: START_SCREEN,
            screen_changed_this_tick: false,
            arena_center: 0.0,
            winner: None,
            next_object_id: 0,
        }
    }

    pub fn fighter(&self, side: PlayerSide) -> &FighterState {
        &self.fighters[side.index()]
    }

    pub fn is_over(&self) -> bool {
        self.winner.is_some()
    }

    /// Advance the whole match by one tick. `inputs` is indexed like
    /// `fighters`: left fighter first.
    pub fn step(&mut self, inputs: [Input; 2], terrain: &Terrain) {
        if self.is_over() {
            return;
        }
        self.tick += 1;
        self.screen_changed_this_tick = false;

        // 1. each fighter, from their own input alone
        for index in 0..2 {
            self.fighters[index].step(&inputs[index], terrain);
        }

        // 2. everything between fighters, collecting kills as they happen
        let mut kills = Vec::new();
        combat::resolve_melee(&mut self.fighters, &mut kills);
        weapons::step(self, &inputs, terrain, &mut kills);

        // 3. deaths, priority, screens, the crush edge, and respawns
        arena::step(self, &kills);
    }

    /// Debug: put a fighter down as if they'd been killed. Local play only —
    /// the 1 and 2 keys.
    pub fn debug_kill(&mut self, side: PlayerSide) {
        let killer_x = self.fighters[side.other().index()].x;
        let kill = Kill {
            victim: side,
            killer: side.other(),
            killer_x,
        };
        arena::apply_kills(self, &[kill]);
    }

    pub(crate) fn next_id(&mut self) -> u32 {
        let id = self.next_object_id;
        self.next_object_id += 1;
        id
    }
}

impl Default for MatchState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------- tests
// Whole-match behavior. Run with `cargo test`.

#[cfg(test)]
mod tests {
    use super::fighter::MAX_HEALTH;
    use super::*;

    fn idle(state: &mut MatchState, terrain: &Terrain, ticks: usize) {
        for _ in 0..ticks {
            state.step([Input::default(); 2], terrain);
        }
    }

    /// Two fighters settled on a flat floor, `gap` apart and facing each other.
    fn duel(gap: f32) -> (MatchState, Terrain) {
        let terrain = Terrain::flat();
        let mut state = MatchState::new();
        idle(&mut state, &terrain, 10);

        state.fighters[0].x = -gap / 2.0;
        state.fighters[0].facing = 1.0;
        state.fighters[1].x = gap / 2.0;
        state.fighters[1].facing = -1.0;
        (state, terrain)
    }

    fn left_attacks(state: &mut MatchState, terrain: &Terrain) {
        let mut inputs = [Input::default(); 2];
        inputs[0].attack = true;
        state.step(inputs, terrain);
    }

    #[test]
    fn a_swing_in_range_lands() {
        let (mut state, terrain) = duel(60.0);
        left_attacks(&mut state, &terrain);

        assert_eq!(state.fighters[1].health, MAX_HEALTH - 1);
        assert!(state.fighters[1].hitstun > 0);
    }

    #[test]
    fn a_block_stops_a_swing_from_the_front() {
        let (mut state, terrain) = duel(60.0);
        state.fighters[1].blocking = true;
        state.fighters[1].block_ticks = 30;

        left_attacks(&mut state, &terrain);

        assert_eq!(state.fighters[1].health, MAX_HEALTH);
        assert!(state.fighters[0].attack_has_hit); // the swing is spent
    }

    #[test]
    fn a_block_does_nothing_against_a_swing_from_behind() {
        let (mut state, terrain) = duel(60.0);
        state.fighters[1].facing = 1.0; // turned away from the attacker
        state.fighters[1].blocking = true;
        state.fighters[1].block_ticks = 30;

        left_attacks(&mut state, &terrain);

        assert_eq!(state.fighters[1].health, MAX_HEALTH - 1);
    }

    #[test]
    fn a_kill_hands_priority_to_the_survivor() {
        let (mut state, terrain) = duel(60.0);
        state.fighters[1].health = 1;

        left_attacks(&mut state, &terrain);

        assert!(state.fighters[1].is_dead());
        assert_eq!(state.priority, Some(PlayerSide::Left));
    }

    #[test]
    fn a_trade_leaves_nobody_in_charge() {
        let (mut state, terrain) = duel(60.0);
        state.fighters[0].health = 1;
        state.fighters[1].health = 1;

        let mut inputs = [Input::default(); 2];
        inputs[0].attack = true;
        inputs[1].attack = true;
        state.step(inputs, &terrain);

        assert!(state.fighters[0].is_dead());
        assert!(state.fighters[1].is_dead());
        assert_eq!(state.priority, None);
    }

    #[test]
    fn a_thrown_sword_kills_then_drops_to_the_ground() {
        let (mut state, terrain) = duel(300.0);

        let mut inputs = [Input::default(); 2];
        inputs[0].throw = true;
        state.step(inputs, &terrain);
        idle(&mut state, &terrain, 60);

        assert!(state.fighters[1].is_dead());
        assert_eq!(state.priority, Some(PlayerSide::Left));
        assert!(state.swords.is_empty());
        assert_eq!(state.ground_weapons.len(), 1);
    }

    #[test]
    fn both_down_respawn_on_their_marks_after_the_wait() {
        let (mut state, terrain) = duel(60.0);
        state.debug_kill(PlayerSide::Left);
        state.debug_kill(PlayerSide::Right);
        assert_eq!(state.priority, None);

        idle(&mut state, &terrain, 241);

        assert!(!state.fighters[0].is_dead());
        assert!(!state.fighters[1].is_dead());
        assert_eq!(state.fighters[0].x, PlayerSide::Left.spawn_x());
        assert_eq!(state.fighters[1].x, PlayerSide::Right.spawn_x());
    }
}