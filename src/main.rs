// The game binary. All of the modules now live in the library (src/lib.rs),
// so this file only builds the app from them.

use bevy::{prelude::*, window::{EnabledButtons, PresentMode}};

use superkill::{common::AppState, credits::CreditsPlugin, game::GamePlugin, loading::LoadingPlugin, main_menu::MainMenuPlugin, settings::SettingsPlugin, test::TestPlugin};
use superkill::camera::CameraPlugin;
use superkill::combat::CombatPlugin;
use superkill::network::NetworkPlugin;
use superkill::player::PlayerPlugin;
use superkill::procedural_test::ProceduralTestPlugin;
use superkill::perlin_noise_test::PerlinNoiseTestPlugin;
use superkill::procedural_gen_menu::ProcGenPlugin;
use superkill::recursive_test::RecursiveTestPlugin;
use superkill::game_over::GameOverPlugin;
use superkill::weapon::WeaponPlugin;
use superkill::animation::AnimationPlugin;
use superkill::level_select::LevelSelectPlugin;
use superkill::health_visuals::HealthVisualsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
            primary_window: Some(Window {
                title: "SuperKill".into(),
                resolution: (1280, 720).into(),
                present_mode: PresentMode::AutoVsync,
                enabled_buttons: EnabledButtons {
                    maximize: false,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        // changed this to just add the plugins
        .add_plugins((MainMenuPlugin, CreditsPlugin, SettingsPlugin, TestPlugin, LoadingPlugin, NetworkPlugin))
        // in-game: level + fighters + camera + combat
        .add_plugins((GamePlugin, PlayerPlugin, CameraPlugin, CombatPlugin, GameOverPlugin, WeaponPlugin, AnimationPlugin, HealthVisualsPlugin))
        .add_plugins(ProceduralTestPlugin)
        .add_plugins(PerlinNoiseTestPlugin)
        .add_plugins(ProcGenPlugin)
        .add_plugins(RecursiveTestPlugin)
        .add_plugins(LevelSelectPlugin)
        .run();
}