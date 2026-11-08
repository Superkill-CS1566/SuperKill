//! Thrown swords and swords lying on the ground.

use serde::{Deserialize, Serialize};

use super::combat::Kill;
use super::fighter::{PlayerSide, Rect, PLAYER_WIDTH};
use super::terrain::Terrain;
use super::{ticks, Input, MatchState, LEVEL_LEN, TICK_SECONDS};

// The part that actually kills. Set these to the blade inside the sprite,
// not the whole cell — most of a 100x100 cell is empty air.
pub const SWORD_HITBOX_W: f32 = 60.0;
pub const SWORD_HITBOX_H: f32 = 16.0;
// Nudge the hitbox off the sprite's center if the blade isn't centered in
// the cell. Positive x is toward the direction of travel.
pub const SWORD_HITBOX_OFFSET_X: f32 = 0.0;
pub const SWORD_HITBOX_OFFSET_Y: f32 = 0.0;

const SWORD_SPEED: f32 = 700.0;
/// How far a throw carries before the sword drops.
const MAX_THROW_DISTANCE: f32 = 1400.0;

/// Four impact frames play where the sword struck, then it drops.
pub const SWORD_IMPACT_FRAME_SECONDS: f32 = 0.05;
const SWORD_IMPACT_TICKS: u16 = ticks(SWORD_IMPACT_FRAME_SECONDS * 4.0);

pub const GROUND_WEAPON_W: f32 = 50.0;
pub const GROUND_WEAPON_H: f32 = 12.0;
/// A fighter this close to a grounded weapon can pick it up.
pub const PICKUP_RADIUS: f32 = 55.0;

/// A sword in flight, or playing its impact frames before it drops.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThrownSword {
    /// Stable between ticks, so the view knows which sprite is which.
    pub id: u32,
    /// Which fighter threw it — they can't be hit by their own sword.
    pub owner: PlayerSide,
    pub x: f32,
    pub y: f32,
    /// -1.0 or 1.0, the direction of travel.
    pub direction: f32,
    /// How much further it can travel before dropping.
    pub remaining: f32,
    /// None while flying; ticks since it struck something once it has.
    pub impact_ticks: Option<u16>,
}

impl ThrownSword {
    pub fn is_flying(&self) -> bool {
        self.impact_ticks.is_none()
    }

    // the collider sits inside the sprite, mirrored with the flip
    pub fn hitbox(&self) -> Rect {
        Rect {
            x: self.x + self.direction * SWORD_HITBOX_OFFSET_X,
            y: self.y + SWORD_HITBOX_OFFSET_Y,
            width: SWORD_HITBOX_W,
            height: SWORD_HITBOX_H,
        }
    }

    /// Which impact frame to show (1 through 4), once it has struck something.
    pub fn impact_frame(&self) -> Option<usize> {
        let elapsed = self.impact_ticks?;
        let frame = 1 + (elapsed as f32 * TICK_SECONDS / SWORD_IMPACT_FRAME_SECONDS) as usize;
        Some(frame.min(4))
    }
}

/// A weapon lying on the ground, waiting to be picked up. Harmless.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundWeapon {
    pub id: u32,
    /// Center of the sprite, resting on the surface.
    pub x: f32,
    pub y: f32,
}

/// Move every sword, strike whatever they hit, launch swords released this
/// tick, and let empty-handed fighters pick one up.
pub fn step(state: &mut MatchState, inputs: &[Input; 2], terrain: &Terrain, kills: &mut Vec<Kill>) {
    // swords already out first; any thrown this tick start moving next tick
    let swords = std::mem::take(&mut state.swords);
    let mut still_out = Vec::new();
    for mut sword in swords {
        if advance_sword(&mut sword, state, terrain, kills) {
            still_out.push(sword);
        }
    }
    state.swords = still_out;

    launch_thrown_swords(state);
    pick_up(state, inputs);
}

/// Returns false once the sword has dropped and should be removed.
fn advance_sword(
    sword: &mut ThrownSword,
    state: &mut MatchState,
    terrain: &Terrain,
    kills: &mut Vec<Kill>,
) -> bool {
    // Plays its impact frames where it struck, then leaves a weapon behind.
    if let Some(elapsed) = sword.impact_ticks {
        let elapsed = elapsed + 1;
        if elapsed >= SWORD_IMPACT_TICKS {
            drop_weapon(state, terrain, sword.x, sword.y);
            return false;
        }
        sword.impact_ticks = Some(elapsed);
        return true;
    }

    // Swords fly straight.
    let step = SWORD_SPEED * TICK_SECONDS;
    sword.x += sword.direction * step;
    sword.remaining -= step;

    let half_w = SWORD_HITBOX_W / 2.0;
    let hitbox = sword.hitbox();

    // did it fly into the side of a hill? If it's touching more than one
    // block, use the face nearest the thrower so it doesn't end up inside
    let mut wall_face: Option<f32> = None;
    for block in &terrain.blocks {
        let block_rect = Rect {
            x: block.x,
            y: block.y,
            width: block.width,
            height: block.height,
        };
        if !hitbox.overlaps(&block_rect) {
            continue;
        }
        // the side of this block facing the incoming sword
        let face = block.x - sword.direction * block.width / 2.0;
        match wall_face {
            Some(nearest) if nearest * sword.direction <= face * sword.direction => {}
            _ => wall_face = Some(face),
        }
    }

    if let Some(face) = wall_face {
        // back it out to the wall so the impact frames play against it
        sword.x = face - sword.direction * (half_w + SWORD_HITBOX_OFFSET_X);
        sword.impact_ticks = Some(0);
        return true;
    }

    // keep it inside the level even on a throw from near the edge
    let limit = LEVEL_LEN / 2.0 - half_w;
    let hit_level_edge = sword.x.abs() >= limit;
    if hit_level_edge {
        sword.x = sword.x.clamp(-limit, limit);
    }

    // running out of distance or reaching the level edge just drops it
    if sword.remaining <= 0.0 || hit_level_edge {
        drop_weapon(state, terrain, sword.x, sword.y);
        return false;
    }

    strike_fighter(sword, state, kills);
    true
}

// A thrown sword kills outright, unless it's blocked.
fn strike_fighter(sword: &mut ThrownSword, state: &MatchState, kills: &mut Vec<Kill>) {
    let hitbox = sword.hitbox();

    for target in &state.fighters {
        if target.side == sword.owner || target.is_dead() {
            continue;
        }
        if !hitbox.overlaps(&target.body()) {
            continue;
        }

        // the sword arrives from the opposite side to the way it's travelling,
        // so it's coming at the defender's front if it moves against their facing
        let from_front = sword.direction * target.facing < 0.0;
        if !(target.blocking && from_front) {
            kills.push(Kill {
                victim: target.side,
                killer: sword.owner,
                killer_x: hitbox.x,
            });
        }

        // blocked or not, it stops here and plays its impact frames
        sword.impact_ticks = Some(0);
        return;
    }
}

// A sword leaves the hand on the release frame of the throw.
fn launch_thrown_swords(state: &mut MatchState) {
    for index in 0..2 {
        let thrower = &state.fighters[index];
        if !thrower.threw_this_tick {
            continue;
        }

        let owner = thrower.side;
        let x = thrower.x + thrower.facing * (PLAYER_WIDTH + SWORD_HITBOX_W) / 2.0;
        let y = thrower.y;
        let direction = thrower.facing;

        let id = state.next_id();
        state.swords.push(ThrownSword {
            id,
            owner,
            x,
            y,
            direction,
            remaining: MAX_THROW_DISTANCE,
            impact_ticks: None,
        });
    }
}

// Empty-handed, the throw button grabs the nearest weapon in reach instead.
fn pick_up(state: &mut MatchState, inputs: &[Input; 2]) {
    for index in 0..2 {
        let fighter = &state.fighters[index];
        if !inputs[index].throw
            || fighter.is_dead()
            || fighter.has_weapon
            || fighter.crouching
            || fighter.throw_ticks.is_some()
        {
            continue;
        }

        let (fighter_x, fighter_y) = (fighter.x, fighter.y);
        let mut nearest: Option<(usize, f32)> = None;
        for (weapon_index, weapon) in state.ground_weapons.iter().enumerate() {
            let dx = weapon.x - fighter_x;
            let dy = weapon.y - fighter_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > PICKUP_RADIUS {
                continue;
            }
            match nearest {
                Some((_, best)) if best <= distance => {}
                _ => nearest = Some((weapon_index, distance)),
            }
        }

        if let Some((weapon_index, _)) = nearest {
            state.ground_weapons.remove(weapon_index);
            state.fighters[index].has_weapon = true;
        }
    }
}

fn drop_weapon(state: &mut MatchState, terrain: &Terrain, x: f32, y: f32) {
    // resting on the surface below, not sunk into it
    let surface = terrain.surface_below(x, y);
    let id = state.next_id();
    state.ground_weapons.push(GroundWeapon {
        id,
        x,
        y: surface + GROUND_WEAPON_H / 2.0,
    });
}