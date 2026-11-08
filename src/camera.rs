use bevy::prelude::*;

use crate::common::{
    AppState, LEVEL_LEN, BackgroundCamera, UiCamera, WIN_W, despawn_background_camera, despawn_ui_camera,
    spawn_background_camera, spawn_ui_camera,
};
use crate::game::{GameState, Match};
use crate::sim::ARENA_WIDTH;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {

        app.init_resource::<CameraFollowState>();

        app.add_systems(
            OnEnter(AppState::InGame),
            (spawn_background_camera, spawn_ui_camera, init_camera_zoom).chain(),
        )
        .add_systems(
            Update,
            (camera_follow, background_camera_follow)
                .chain()
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameState::Playing))
                .run_if(resource_exists::<Match>),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (despawn_background_camera, despawn_ui_camera),
        );
    }
}

/// Width of the visible world. It's exactly the simulation's play area, so
/// the crush edge is always the edge of the screen.
pub const SCREEN_W: f32 = ARENA_WIDTH;
/// Follows from the width — 0.6 at the current numbers (0.5 = 2× zoomed in).
pub const ZOOM: f32 = SCREEN_W / WIN_W;

const CAMERA_Y: f32 = -120.0;
const FOLLOW_SPEED: f32 = 9.0;

#[derive(Resource, Default)]
pub struct CameraFollowState {
    initialized: bool,
    /// The screen the camera last followed — a change means cut, not pan.
    screen: usize,
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

// The simulation decides where the play area is — locked to the screen center
// when neutral, ahead of a lone leader, between two live fighters. The camera
// just eases toward it.
pub fn camera_follow(
    time: Res<Time>,
    game: Res<Match>,
    mut follow: ResMut<CameraFollowState>,
    mut camera: Single<&mut Transform, With<UiCamera>>,
) {
    let target = game.state.arena_center;

    // A screen transition teleports players; only ordinary following should ease.
    let screen = game.state.current_screen;
    let snap = !follow.initialized || screen != follow.screen;

    camera.translation.x = smooth_camera_x(camera.translation.x, target, time.delta_secs(), snap);
    camera.translation.y = CAMERA_Y;
    follow.initialized = true;
    follow.screen = screen;
}

const PARALLAX_FACTOR: f32 = 0.4;

pub fn background_camera_follow(
    gameplay_camera: Single<&Transform, (With<UiCamera>, Without<BackgroundCamera>)>,
    mut background_camera: Single<&mut Transform, (With<BackgroundCamera>, Without<UiCamera>)>,
) {
    background_camera.translation.x = gameplay_camera.translation.x * PARALLAX_FACTOR;

    background_camera.translation.y = CAMERA_Y * PARALLAX_FACTOR;
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