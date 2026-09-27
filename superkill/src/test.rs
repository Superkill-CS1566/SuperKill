use bevy::prelude::*;
use crate::common::{despawn_ui_camera, spawn_ui_camera, back_to_menu, AppState};

pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Test), (spawn_ui_camera, setup_test))
            .add_systems(Update, back_to_menu.run_if(in_state(AppState::Test)))
            .add_systems(OnExit(AppState::Test), (despawn_test, despawn_ui_camera));
    }
}

#[derive(Component)]
struct TestRoot;

fn setup_test(mut commands: Commands) {
    info!("Entered Test state");
    commands.spawn((
        Sprite::from_color(Color::srgb(0.2, 0.7, 0.3), Vec2::new(200., 200.)),
        Transform::default(),
        TestRoot,
    ));
}

fn despawn_test(mut commands: Commands, q: Query<Entity, With<TestRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}