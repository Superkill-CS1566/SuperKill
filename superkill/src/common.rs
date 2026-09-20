use bevy::prelude::*;

// Since the stuff in this file is shared between main.rs, main_menu.rs, and credits.rs, I made them have `pub`.
#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,  //主菜单
    Credits,  //鸣谢
    Settings, //设置
    // can create a separate game state for actual game
}

// camera
#[derive(Component)]
pub struct UiCamera;

// Made spawning in camera a function since both screens spawned their own identical camera.
pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::default(),
        GlobalTransform::default(),
        UiCamera,
    ));
}

// Same with despawning cameras
pub fn despawn_ui_camera(mut commands: Commands, cameras: Query<Entity, With<UiCamera>>) {
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}