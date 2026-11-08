//! The match around the fighters: who leads, where the play area is, the crush
//! edge, respawning, screen transitions, and the win.

use super::combat::Kill;
use super::fighter::{Death, PlayerSide, PLAYER_WIDTH};
use super::{ticks, MatchState, LEVEL_LEN};

/// Width of the play area the camera shows. camera.rs sets its zoom from this,
/// so the crush edge and the edge of the screen always line up. (This was
/// `SCREEN_W` — 1280 at a 0.6 zoom.)
pub const ARENA_WIDTH: f32 = 768.0;

/// How far off-center the leader sits, so the level ahead of them is visible.
const LOOKAHEAD: f32 = 160.0;
/// How close to the edge of the play area the leader may be pushed, with both alive.
const CAGE_MARGIN: f32 = 80.0;

const RESPAWN_ADVANCE: f32 = 220.0;
const RESPAWN_INSET: f32 = 60.0;
const RESPAWN_MAX_WAIT_TICKS: u16 = ticks(4.0);

pub const SCREEN_COUNT: usize = 3;
/// Every match starts on the middle screen.
pub const START_SCREEN: usize = 1;
const SCREEN_INSET: f32 = 20.0;

/// Everything after the fighters have moved and struck: deaths, screens, the
/// play area, the crush edge, and respawns — in that order.
pub fn step(state: &mut MatchState, kills: &[Kill]) {
    apply_kills(state, kills);

    screen_transition(state);
    if state.winner.is_some() {
        return;
    }

    update_arena_center(state);

    let crushed = cage_and_crush(state);
    apply_kills(state, &crushed);

    respawn(state);
}

/// Put each victim down, then settle priority: a kill hands it to whoever is
/// still standing, and a trade — nobody standing — leaves the match neutral.
pub fn apply_kills(state: &mut MatchState, kills: &[Kill]) {
    if kills.is_empty() {
        return;
    }

    for kill in kills {
        let victim = &mut state.fighters[kill.victim.index()];
        if victim.is_dead() {
            continue; // already down — e.g. a sword and a swing on the same tick
        }
        victim.death = Some(Death {
            killer_x: kill.killer_x,
            ticks_down: 0,
        });
        victim.attack = None;
        // otherwise a throw in progress would finish after respawning
        victim.throw_ticks = None;
    }

    let mut standing = Vec::new();
    for fighter in &state.fighters {
        if !fighter.is_dead() {
            standing.push(fighter.side);
        }
    }
    state.priority = match standing.len() {
        0 => None,
        1 => Some(standing[0]),
        _ => state.priority,
    };
}

// only the fighter holding priority may cross, and only toward their own goal
fn screen_transition(state: &mut MatchState) {
    // neutral — nobody advances
    let Some(holder) = state.priority else {
        return;
    };
    let leader = &state.fighters[holder.index()];
    if leader.is_dead() {
        return;
    }

    let dir = holder.advance_dir();
    // edge of the background image, not the edge of the view
    let edge = LEVEL_LEN / 2.0;
    if leader.x * dir < edge - PLAYER_WIDTH / 2.0 - 1.0 {
        return;
    }

    let next = if dir > 0.0 {
        if state.current_screen + 1 >= SCREEN_COUNT {
            // outermost screen to the right - P1 wins
            state.winner = Some(holder);
            return;
        }
        state.current_screen + 1
    } else {
        if state.current_screen == 0 {
            // outermost screen to the left - P2 wins
            state.winner = Some(holder);
            return;
        }
        state.current_screen - 1
    };
    state.current_screen = next;
    state.screen_changed_this_tick = true;

    // despawn weapons left behind in old screen
    state.swords.clear();
    state.ground_weapons.clear();

    // leader enters from behind; the other fighter appears ahead of them
    for fighter in &mut state.fighters {
        if fighter.is_dead() {
            continue;
        }
        let sign = if fighter.side == holder { -1.0 } else { 1.0 };
        fighter.x = sign * (edge - SCREEN_INSET) * dir;
        fighter.vx = 0.0;
    }
}

// The three camera modes, now as gameplay: locked to the screen center when
// neutral, ahead of the leader when they're alone, between the two when both
// are up. The camera smooths toward this for display; the crush edge and
// respawns use it exactly.
fn update_arena_center(state: &mut MatchState) {
    let half_view = ARENA_WIDTH / 2.0;

    let target = match state.priority {
        None => 0.0,
        Some(holder) => {
            let leader = &state.fighters[holder.index()];
            if leader.is_dead() {
                return; // leave the play area where it is
            }
            let other = &state.fighters[holder.other().index()];

            if other.is_dead() {
                // leader alone — offset them toward their back edge
                leader.x + LOOKAHEAD * holder.advance_dir()
            } else {
                // both alive — midpoint, but never let the leader leave the area
                ((leader.x + other.x) / 2.0).clamp(
                    leader.x - half_view + CAGE_MARGIN,
                    leader.x + half_view - CAGE_MARGIN,
                )
            }
        }
    };

    let bound = LEVEL_LEN / 2.0 - half_view;
    state.arena_center = target.clamp(-bound, bound);
}

// Keeps everyone inside the play area. For the trailing fighter the back edge
// is lethal: if the leader outruns them, the edge catches them and they die.
fn cage_and_crush(state: &mut MatchState) -> Vec<Kill> {
    let half_view = ARENA_WIDTH / 2.0;
    let half_w = PLAYER_WIDTH / 2.0;
    let min = state.arena_center - half_view + half_w;
    let max = state.arena_center + half_view - half_w;

    // where the leader stands right now, recorded on any crush death
    let mut leader: Option<(PlayerSide, f32)> = None;
    if let Some(holder) = state.priority {
        let fighter = &state.fighters[holder.index()];
        if !fighter.is_dead() {
            leader = Some((holder, fighter.x));
        }
    }

    let mut crushed = Vec::new();
    for fighter in &mut state.fighters {
        if fighter.is_dead() {
            continue;
        }

        if let Some((holder, holder_x)) = leader {
            if fighter.side != holder {
                let dir = holder.advance_dir();
                let trailing = if dir > 0.0 { min } else { max };
                // behind the leader's back edge — crushed out of frame
                if (fighter.x - trailing) * dir < 0.0 {
                    crushed.push(Kill {
                        victim: fighter.side,
                        killer: holder,
                        killer_x: holder_x,
                    });
                    continue;
                }
            }
        }

        fighter.x = fighter.x.clamp(min, max);
    }
    crushed
}

fn respawn(state: &mut MatchState) {
    let half_view = ARENA_WIDTH / 2.0;

    // a leader who's standing, if there is one
    let mut leader: Option<(PlayerSide, f32)> = None;
    if let Some(holder) = state.priority {
        let fighter = &state.fighters[holder.index()];
        if !fighter.is_dead() {
            leader = Some((holder, fighter.x));
        }
    }

    for index in 0..2 {
        let Some(mut death) = state.fighters[index].death else {
            continue;
        };
        death.ticks_down = death.ticks_down.saturating_add(1);
        state.fighters[index].death = Some(death);
        let waited_long_enough = death.ticks_down >= RESPAWN_MAX_WAIT_TICKS;

        let respawn_x = match leader {
            Some((holder, holder_x)) => {
                let dir = holder.advance_dir();
                let advanced = (holder_x - death.killer_x) * dir;

                // positions reset on a screen change, so the distance check is
                // only meaningful within one screen — and a new screen is a
                // checkpoint that brings anyone down back
                if !state.screen_changed_this_tick
                    && !waited_long_enough
                    && advanced < RESPAWN_ADVANCE
                {
                    continue;
                }
                // appear at the edge the leader is pushing toward
                state.arena_center + (half_view - RESPAWN_INSET) * dir
            }

            // neutral, or the leader is down too — everyone back to their
            // own mark once the wait is up
            None => {
                if !waited_long_enough {
                    continue;
                }
                state.fighters[index].side.spawn_x()
            }
        };

        state.fighters[index].respawn_at(respawn_x);
    }
}