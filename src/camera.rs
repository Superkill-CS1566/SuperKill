use bevy::prelude::*;

use crate::combat::{Dead, Priority};
use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState, UiCamera, LEVEL_LEN, WIN_W};
use crate::game::{screen_transition, GameState};
use crate::player::{Player, PlayerSide};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::InGame),
            (spawn_ui_camera, init_camera_zoom).chain(),
        )
        .add_systems(
            Update,
            camera_follow
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameState::Playing))
                .after(screen_transition),
        )
        .add_systems(OnExit(AppState::InGame), despawn_ui_camera);
    }
}

pub const ZOOM: f32 = 0.80; // 0.5 = 2× zoomed in
/// Width of the visible world at this zoom.
pub const SCREEN_W: f32 = WIN_W * ZOOM;

/// How far off-center the leader sits, so the level ahead of them is visible.
const LOOKAHEAD: f32 = 160.0;
/// How close to the view edge the leader may be pushed in midpoint mode.
const CAGE_MARGIN: f32 = 80.0;

pub fn init_camera_zoom(mut camera: Single<&mut Projection, With<UiCamera>>) {
    if let Projection::Orthographic(ref mut ortho) = **camera {
        ortho.scale = ZOOM;
    }
}

// three modes: locked (neutral), follow-with-lookahead (leader alone),
// midpoint (both alive)
pub fn camera_follow(
    priority: Res<Priority>,
    players: Query<(&Transform, &PlayerSide), (With<Player>, Without<Dead>)>,
    mut camera: Single<&mut Transform, (With<UiCamera>, Without<Player>)>,
) {
    let half_view = SCREEN_W / 2.0;
    let bound = LEVEL_LEN / 2.0 - half_view;

    let target = match priority.0 {
        // neutral — locked to the center of the current screen
        None => 0.0,

        Some(holder) => {
            let Some(holder_x) = players
                .iter()
                .find(|(_, s)| **s == holder)
                .map(|(t, _)| t.translation.x)
            else {
                return; // holder is down this frame; leave the camera where it is
            };

            let other_x = players
                .iter()
                .find(|(_, s)| **s != holder)
                .map(|(t, _)| t.translation.x);

            match other_x {
                // leader alone — offset them toward their back edge
                None => holder_x + LOOKAHEAD * holder.advance_dir(),

                // both alive — midpoint, but never let the leader leave frame
                Some(other_x) => ((holder_x + other_x) / 2.0).clamp(
                    holder_x - half_view + CAGE_MARGIN,
                    holder_x + half_view - CAGE_MARGIN,
                ),
            }
        }
    };

    camera.translation.x = target.clamp(-bound, bound);
}