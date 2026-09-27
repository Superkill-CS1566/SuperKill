use bevy::prelude::*;
use crate::common::{despawn_ui_camera, spawn_ui_camera, back_to_menu, AppState};

pub struct PlayPlugin;

impl Plugin for PlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Play), (spawn_ui_camera, setup_play))
            .add_systems(Update, back_to_menu.run_if(in_state(AppState::Play)))
            .add_systems(OnExit(AppState::Play), (despawn_play, despawn_ui_camera));
    }
}

#[derive(Component)]
struct PlayRoot;

fn setup_play(mut commands: Commands) {
    info!("Entered Play state");
    commands.spawn((
        Sprite::from_color(Color::srgb(0.2, 0.7, 0.3), Vec2::new(200., 200.)),
        Transform::default(),
        PlayRoot,
    ));
}

fn despawn_play(mut commands: Commands, q: Query<Entity, With<PlayRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}