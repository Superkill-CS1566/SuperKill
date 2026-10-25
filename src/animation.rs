use bevy::prelude::*;

use crate::common::AppState;
use crate::game::GameState;
use crate::loading::LoadingAssets;
use crate::player::Player;
use crate::weapon::{Throwing, THROW_FRAME_TIME};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_player_sheets).add_systems(
            Update,
            (pick_anim_state, advance_anim)
                .chain()
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------- assets

pub struct Sheet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource)]
pub struct PlayerSheets {
    idle: Sheet,
    run: Sheet,
    crouch: Sheet,
    jump: Sheet,
    throw: Sheet,
}

/// Cell size shared by every player sheet. Must divide each PNG's width
/// exactly or frames drift sideways during playback.
const PLAYER_CELL_W: u32 = 100;
const PLAYER_CELL_H: u32 = 100;

/// How large the character is drawn, in world units. Keep the same ratio as
/// the cell above. The collision box stays 40x60 regardless.
pub const PLAYER_SPRITE_W: f32 = 100.0;
pub const PLAYER_SPRITE_H: f32 = 100.0;

fn load_player_sheets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut loading_assets: ResMut<LoadingAssets>,
) {
    let mut sheet = |path: &str, frames: u32| -> Sheet {
        let image: Handle<Image> = asset_server.load(path.to_string());
        loading_assets.0.push(image.clone().untyped());
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(PLAYER_CELL_W, PLAYER_CELL_H),
            frames,
            1,
            None,
            None,
        ));
        Sheet { image, layout }
    };

    commands.insert_resource(PlayerSheets {
        idle: sheet("sprites/player/player_standing.png", 4),
        run: sheet("sprites/player/player_running.png", 8),
        crouch: sheet("sprites/player/player_crouching.png", 4),
        jump: sheet("sprites/player/player_jumping.png", 6),
        throw: sheet("sprites/player/player_throwing.png", 6),
    });
}

// ---------------------------------------------------------------- states

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnimState {
    Idle,
    Run,
    Crouch,
    /// The crouch frames played backward on release.
    CrouchExit,
    /// Launch frames, holding the last one while airborne.
    Jump,
    /// The two landing frames at the end of the jump sheet.
    Land,
    Throw,
}

enum LoopMode {
    /// Cycles forever.
    Repeat,
    /// Plays to the last frame and stays there.
    HoldLast,
    /// Plays through once, then hands control back.
    Once,
    /// Plays from last frame to first, then hands control back.
    OnceReverse,
}

impl AnimState {
    /// Inclusive frame range within this state's sheet.
    fn range(self) -> (usize, usize) {
        match self {
            AnimState::Idle => (0, 3),
            AnimState::Run => (0, 7),
            AnimState::Crouch | AnimState::CrouchExit => (0, 3),
            AnimState::Jump => (0, 3),
            AnimState::Land => (4, 5),
            AnimState::Throw => (0, 5),
        }
    }

    fn frame_time(self) -> f32 {
        match self {
            AnimState::Idle => 0.16,
            AnimState::Run => 0.08,
            AnimState::Crouch => 0.16,
            AnimState::CrouchExit => 0.04,
            AnimState::Jump => 0.07,
            AnimState::Land => 0.07,
            // must match weapon.rs, or the sword leaves on the wrong frame
            AnimState::Throw => THROW_FRAME_TIME,
        }
    }

    fn loop_mode(self) -> LoopMode {
        match self {
            AnimState::Idle | AnimState::Run | AnimState::Crouch => LoopMode::Repeat,
            AnimState::Jump => LoopMode::HoldLast,
            AnimState::CrouchExit => LoopMode::OnceReverse,
            AnimState::Land | AnimState::Throw => LoopMode::Once,
        }
    }

    /// One-shots keep control until they finish.
    fn locks(self) -> bool {
        matches!(
            self,
            AnimState::CrouchExit | AnimState::Land | AnimState::Throw
        )
    }

    fn sheet(self, sheets: &PlayerSheets) -> &Sheet {
        match self {
            AnimState::Idle => &sheets.idle,
            AnimState::Run => &sheets.run,
            AnimState::Crouch | AnimState::CrouchExit => &sheets.crouch,
            AnimState::Jump | AnimState::Land => &sheets.jump,
            AnimState::Throw => &sheets.throw,
        }
    }
}

#[derive(Component)]
pub struct PlayerAnim {
    state: AnimState,
    frame: usize,
    timer: Timer,
    finished: bool,
    /// Movement animation to pick back up after a one-shot, so a throw
    /// doesn't restart the run cycle from frame 0.
    resume: Option<(AnimState, usize)>,
}

impl PlayerAnim {
    pub fn new() -> Self {
        Self {
            state: AnimState::Idle,
            frame: 0,
            timer: Timer::from_seconds(AnimState::Idle.frame_time(), TimerMode::Repeating),
            finished: false,
            resume: None,
        }
    }

    pub fn starting_sheet(sheets: &PlayerSheets) -> (Handle<Image>, TextureAtlas) {
        let sheet = AnimState::Idle.sheet(sheets);
        (
            sheet.image.clone(),
            TextureAtlas {
                layout: sheet.layout.clone(),
                index: 0,
            },
        )
    }
}

// ---------------------------------------------------------------- systems

// Decides which animation should be playing, and swaps sheets on a change.
fn pick_anim_state(
    sheets: Res<PlayerSheets>,
    mut players: Query<(&Player, &mut PlayerAnim, &mut Sprite, Has<Throwing>)>,
) {
    for (player, mut anim, mut sprite, throwing) in &mut players {
        let previous = anim.state;

        // a one-shot holds the screen until it ends — except that leaving the
        // ground cancels a landing
        if previous.locks() && !anim.finished {
            let jump_cancels_land = previous == AnimState::Land && !player.is_grounded;
            if !jump_cancels_land {
                continue;
            }
        }

        let next = if throwing {
            AnimState::Throw
        } else if !player.is_grounded {
            AnimState::Jump
        } else if player.is_crouching {
            AnimState::Crouch
        } else if previous == AnimState::Jump {
            AnimState::Land
        } else if previous == AnimState::Crouch {
            AnimState::CrouchExit
        } else if player.velocity.x != 0.0 {
            AnimState::Run
        } else {
            AnimState::Idle
        };

        if next == previous {
            continue;
        }

        // remember where the movement cycle was, so the throw can resume it
        if next == AnimState::Throw && matches!(previous, AnimState::Idle | AnimState::Run) {
            anim.resume = Some((previous, anim.frame));
        }

        let resumed = match anim.resume {
            Some((state, frame)) if state == next => Some(frame),
            _ => None,
        };
        if resumed.is_some() {
            anim.resume = None;
        }

        let (first, last) = next.range();
        anim.frame = match resumed {
            Some(frame) => frame,
            None => match next.loop_mode() {
                LoopMode::OnceReverse => last,
                _ => first,
            },
        };
        anim.state = next;
        anim.finished = false;
        anim.timer = Timer::from_seconds(next.frame_time(), TimerMode::Repeating);

        let sheet = next.sheet(&sheets);
        sprite.image = sheet.image.clone();
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: anim.frame,
        });
    }
}

// Steps the current animation and writes the atlas index.
fn advance_anim(time: Res<Time>, mut players: Query<(&Player, &mut PlayerAnim, &mut Sprite)>) {
    for (player, mut anim, mut sprite) in &mut players {
        // sheets are drawn facing right
        sprite.flip_x = player.facing < 0.0;

        if anim.finished {
            continue;
        }

        anim.timer.tick(time.delta());
        if !anim.timer.just_finished() {
            continue;
        }

        let (first, last) = anim.state.range();
        match anim.state.loop_mode() {
            LoopMode::Repeat => {
                anim.frame = if anim.frame >= last { first } else { anim.frame + 1 };
            }
            LoopMode::HoldLast | LoopMode::Once => {
                if anim.frame >= last {
                    anim.finished = true;
                } else {
                    anim.frame += 1;
                }
            }
            LoopMode::OnceReverse => {
                if anim.frame <= first {
                    anim.finished = true;
                } else {
                    anim.frame -= 1;
                }
            }
        }

        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = anim.frame;
        }
    }
}