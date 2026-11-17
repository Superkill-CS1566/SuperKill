//! The match around the fighters: who leads, where the play area is, the crush
//! edge, respawning, screen transitions, and the win.

use crate::sim::ScrollMode;

use super::combat::Kill;
use super::fighter::{Death, PLAYER_WIDTH, PlayerSide};
use super::{LEVEL_LEN, MatchState, TICK_SECONDS, ticks};

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

pub const SCREEN_COUNT: usize = 5;
/// Every match starts on the middle screen.
pub const START_SCREEN: usize = 2;
const SCREEN_INSET: f32 = 20.0;

// Autoscrolling constants
const SCROLL_SPEED: f32 = 90.0; // units per second
const SCROLL_PER_TICK: f32 = SCROLL_SPEED * TICK_SECONDS;
/// Deaths a fighter can take before they're eliminated. MatchState::new
/// starts both fighters with this many.
pub const STARTING_LIVES: u8 = 3;
/// How far ahead of the survivor the finish line goes once the other fighter
/// is eliminated.
const FINISH_DISTANCE: f32 = 1500.0;
/// Respawns come in this far above the flat-ground height and land on
/// whatever hill is there. Keep it above the tallest hill.
const RESPAWN_DROP: f32 = 200.0;

/// Everything after the fighters have moved and struck: deaths, screens, the
/// play area, the crush edge, and respawns — in that order.
pub fn step(state: &mut MatchState, kills: &[Kill]) {
    apply_kills(state, kills);
    update_scroll_heading(state);

    screen_transition(state);
    if state.winner.is_some() {
        return;
    }

    update_arena_center(state);

    let crushed = cage_and_crush(state);
    apply_kills(state, &crushed);
    update_scroll_heading(state);

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

        // autoscroll: each death costs a life
        if state.scroll_mode == ScrollMode::Autoscroll && state.scroll_winner.is_none() {
            let lives = &mut state.lives[kill.victim.index()];
            *lives = lives.saturating_sub(1);
        }
    }

    check_elimination(state);

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

    // once someone is eliminated the survivor keeps priority for good
    if let Some(winner) = state.scroll_winner {
        state.priority = Some(winner);
    }
}

/// Autoscroll: a fighter out of lives ends the scroll, and a finish line goes
/// up ahead of the survivor.
fn check_elimination(state: &mut MatchState) {
    if state.scroll_mode != ScrollMode::Autoscroll || state.scroll_winner.is_some() {
        return;
    }
    let survivor = match [state.lives[0] == 0, state.lives[1] == 0] {
        [true, true] => {
            state.lives = [1, 1]; // simultaneous: sudden death
            return;
        }
        [true, false] => PlayerSide::Right,
        [false, true] => PlayerSide::Left,
        [false, false] => return,
    };
    state.scroll_winner = Some(survivor);
    state.finish_x = Some(state.fighters[survivor.index()].x + survivor.advance_dir() * FINISH_DISTANCE);
}

// only the fighter holding priority may cross, and only toward their own goal
fn screen_transition(state: &mut MatchState) {
    // the autoscroll level is endless: no screens, just the finish line
    if state.scroll_mode == ScrollMode::Autoscroll {
        autoscroll_finish(state);
        return;
    }

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

/// The survivor of an autoscroll match wins by reaching the finish line.
fn autoscroll_finish(state: &mut MatchState) {
    let (Some(winner), Some(finish_x)) = (state.scroll_winner, state.finish_x) else {
        return;
    };
    let fighter = &state.fighters[winner.index()];
    if !fighter.is_dead() && (fighter.x - finish_x) * winner.advance_dir() >= 0.0 {
        state.winner = Some(winner);
    }
}

/// Autoscroll only: the drift direction. Priority decides it (toward the
/// non-holder's goal), but it's remembered, so the scroll never stops while
/// someone is dead or priority is neutral.
fn update_scroll_heading(state: &mut MatchState) {
    if state.scroll_mode != ScrollMode::Autoscroll {
        return;
    }
    if let Some(holder) = state.priority {
        state.scroll_heading = holder.other().advance_dir();
    }
}

/// The direction the play area is drifting. `None` outside autoscroll, and
/// once someone is eliminated — then the scroll stops and the usual leader
/// rules take over.
fn scroll_dir(state: &MatchState) -> Option<f32> {
    (state.scroll_mode == ScrollMode::Autoscroll && state.scroll_winner.is_none())
        .then_some(state.scroll_heading)
}

// The three camera modes, now as gameplay: locked to the screen center when
// neutral, ahead of the leader when they're alone, between the two when both
// are up. In autoscroll the area just keeps drifting. The camera smooths
// toward this for display; the crush edge and respawns use it exactly.
fn update_arena_center(state: &mut MatchState) {
    let half_view = ARENA_WIDTH / 2.0;
    let bound = LEVEL_LEN / 2.0 - half_view;

    // autoscroll: the area moves on its own, whoever is alive or leading, and
    // the level has no end to stop at
    if let Some(dir) = scroll_dir(state) {
        state.arena_center += SCROLL_PER_TICK * dir;
        return;
    }

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

    // the autoscroll level is endless, so the survivor can run past LEVEL_LEN
    state.arena_center = if state.scroll_mode == ScrollMode::Autoscroll {
        target
    } else {
        target.clamp(-bound, bound)
    };
}

// Keeps everyone inside the play area. For the trailing fighter the back edge
// is lethal: if the leader outruns them, the edge catches them and they die.
// (Autoscroll has its own version while it's scrolling: the edge is lethal
// for anyone it passes.)
fn cage_and_crush(state: &mut MatchState) -> Vec<Kill> {
    if let Some(dir) = scroll_dir(state) {
        return autoscroll_crush(state, dir);
    }

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

/// Autoscroll: whoever the trailing edge passes is crushed, leader or not.
/// The other fighter is credited with the kill, so priority flips and the
/// scroll turns around.
fn autoscroll_crush(state: &mut MatchState, dir: f32) -> Vec<Kill> {
    let half_view = ARENA_WIDTH / 2.0;
    let half_w = PLAYER_WIDTH / 2.0;
    let min = state.arena_center - half_view + half_w;
    let max = state.arena_center + half_view - half_w;
    let trailing = if dir > 0.0 { min } else { max };
    let xs = [state.fighters[0].x, state.fighters[1].x];

    let mut crushed = Vec::new();
    for fighter in &mut state.fighters {
        if fighter.is_dead() {
            continue;
        }
        // fell behind the edge that's sweeping past
        if (fighter.x - trailing) * dir < 0.0 {
            let killer = fighter.side.other();
            crushed.push(Kill {
                victim: fighter.side,
                killer,
                killer_x: xs[killer.index()],
            });
            continue;
        }
        // can't run past the leading edge
        fighter.x = if dir > 0.0 {
            fighter.x.min(max)
        } else {
            fighter.x.max(min)
        };
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

                if state.scroll_mode == ScrollMode::Autoscroll {
                    // no screens to cross, and the leader is being pushed back
                    // rather than advancing: the timer decides
                    if !waited_long_enough {
                        continue;
                    }
                } else if !state.screen_changed_this_tick
                    && !waited_long_enough
                    && advanced < RESPAWN_ADVANCE
                {
                    // positions reset on a screen change, so the distance check
                    // is only meaningful within one screen — and a new screen is
                    // a checkpoint that brings anyone down back
                    continue;
                }
                // Free: the edge the leader is pushing toward. Autoscroll while
                // scrolling: that edge is the lethal one, so appear on the side
                // the area is moving toward instead.
                let spawn_dir = scroll_dir(state).unwrap_or(dir);
                state.arena_center + (half_view - RESPAWN_INSET) * spawn_dir
            }

            // neutral, or the leader is down too — everyone back once the
            // wait is up
            None => {
                if !waited_long_enough {
                    continue;
                }
                let own_mark = state.fighters[index].side.spawn_x();
                match scroll_dir(state) {
                    // their own mark could be behind the edge and kill them
                    // at once, so come in on the leading side
                    Some(dir) => state.arena_center + (half_view - RESPAWN_INSET) * dir,
                    // endless level: marks are relative to where the area is
                    None if state.scroll_mode == ScrollMode::Autoscroll => {
                        state.arena_center + own_mark
                    }
                    None => own_mark,
                }
            }
        };

        state.fighters[index].respawn_at(respawn_x);
        if state.scroll_mode == ScrollMode::Autoscroll {
            // drop in from above: respawn_at uses the flat-ground height
            state.fighters[index].y += RESPAWN_DROP;
        }
    }
}
