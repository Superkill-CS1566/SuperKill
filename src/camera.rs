use bevy::prelude::*;

use crate::combat::{Dead, Priority};
use crate::common::{AppState, LEVEL_LEN, UiCamera, WIN_W, despawn_ui_camera, spawn_ui_camera};
use crate::game::{GameState, screen_transition};
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
        app.init_resource::<CameraFollowState>();
    }
}

pub const ZOOM: f32 = 0.6; // 0.5 = 2× zoomed in
/// Width of the visible world at this zoom.
pub const SCREEN_W: f32 = WIN_W * ZOOM;

/// How far off-center the leader sits, so the level ahead of them is visible.
const LOOKAHEAD: f32 = 160.0;
/// How close to the view edge the leader may be pushed in midpoint mode.
const CAGE_MARGIN: f32 = 80.0;
const CAMERA_Y: f32 = -120.0;
const FOLLOW_SPEED: f32 = 9.0;

#[derive(Resource, Default)]
pub struct CameraFollowState {
    initialized: bool,
}

pub fn init_camera_zoom(
    mut camera: Single<(&mut Projection, &mut Transform), With<UiCamera>>,
    mut follow: ResMut<CameraFollowState>,
) {
    let (projection, transform) = &mut *camera;
    if let Projection::Orthographic(ortho) = &mut **projection {
        ortho.scale = ZOOM;
    }
    transform.translation.y = CAMERA_Y;
    follow.initialized = false;
}

fn smooth_camera_x(current: f32, target: f32, delta_secs: f32, snap: bool) -> f32 {
    let bound = LEVEL_LEN / 2.0 - SCREEN_W / 2.0;
    let target = target.clamp(-bound, bound);
    if snap {
        return target;
    }
    let blend = 1.0 - (-FOLLOW_SPEED * delta_secs.max(0.0)).exp();
    (current + (target - current) * blend).clamp(-bound, bound)
}

// three modes: locked (neutral), follow-with-lookahead (leader alone),
// midpoint (both alive)
pub fn camera_follow(
    time: Res<Time>,
    priority: Res<Priority>,
    current_screen: Res<crate::game::CurrentScreen>,
    mut follow: ResMut<CameraFollowState>,
    players: Query<(&Transform, &PlayerSide), (With<Player>, Without<Dead>)>,
    mut camera: Single<&mut Transform, (With<UiCamera>, Without<Player>)>,
) {
    let half_view = SCREEN_W / 2.0;

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

    // A screen transition teleports players; only ordinary following should ease.
    let snap = !follow.initialized || current_screen.is_changed();
    camera.translation.x = smooth_camera_x(camera.translation.x, target, time.delta_secs(), snap);
    camera.translation.y = CAMERA_Y;
    follow.initialized = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn death_target_change_eases_without_jumping_or_overshooting() {
        let start = 0.0;
        let target = 300.0;
        let first = smooth_camera_x(start, target, 1.0 / 60.0, false);
        assert!(first > start && first < target);
        assert!(first - start < 50.0);
        let mut x = first;
        for _ in 0..60 {
            let next = smooth_camera_x(x, target, 1.0 / 60.0, false);
            assert!(next >= x && next <= target);
            x = next;
        }
        assert!((x - target).abs() < 0.1);
    }

    #[test]
    fn smoothing_is_frame_rate_independent() {
        let follow_for_second = |fps: usize| {
            let mut x = -200.0;
            for _ in 0..fps {
                x = smooth_camera_x(x, 300.0, 1.0 / fps as f32, false);
            }
            x
        };
        assert!((follow_for_second(30) - follow_for_second(144)).abs() < 0.001);
    }

    #[test]
    fn screen_changes_snap_and_all_positions_stay_inside_level() {
        let bound = LEVEL_LEN / 2.0 - SCREEN_W / 2.0;
        assert_eq!(smooth_camera_x(-bound, 400.0, 0.0, true), 400.0);
        assert_eq!(smooth_camera_x(0.0, 9999.0, 1.0, true), bound);
        assert_eq!(smooth_camera_x(0.0, -9999.0, 1.0, true), -bound);
        assert!(smooth_camera_x(0.0, 9999.0, 10.0, false) <= bound);
        assert!(smooth_camera_x(0.0, -9999.0, 10.0, false) >= -bound);
    }

    #[test]
    fn zero_delta_does_not_move_the_camera() {
        assert_eq!(smooth_camera_x(100.0, -300.0, 0.0, false), 100.0);
    }
}
