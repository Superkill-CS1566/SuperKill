use bevy::prelude::*;
pub const LEVEL_LEN: f32 = 1920.;
pub const WIN_W: f32 = 1280.;
pub const WIN_H: f32 = 720.;

// Default Platform — shared by game.rs (spawning) and player.rs (collision)
pub const GROUND_Y: f32 = -300.0;        // center of the ground platform
pub const GROUND_THICKNESS: f32 = 60.0;

// Since the stuff in this file is shared between main.rs, main_menu.rs, and credits.rs, I made them have `pub`.
#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,  //主菜单
    Credits,  //鸣谢
    Settings, //设置
    InGame,   //游戏
    Loading, 
    Testing,
    ProcGenMenu,
    DFSMazeTesting,
    PNoiseTesting,
    RDMazeTesting
}

// camera
#[derive(Component)]
pub struct UiCamera;

// any solid surface the fighters stand on or bump into
#[derive(Component)]
pub struct Platform {
    pub width: f32,
    pub height: f32,
}

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