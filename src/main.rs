mod camera;
mod combat;
mod common;
mod game;
mod loading;
mod main_menu;
mod network;
mod player;
mod protocol;

use bevy::{prelude::*, window::{EnabledButtons, PresentMode}};

use crate::{common::AppState, game::GamePlugin, loading::LoadingPlugin, main_menu::MainMenuPlugin};
use crate::network::NetworkPlugin;
mod main_menu;
mod player;
mod procedural_generation;
mod procedural_test;
mod settings;
mod test;
mod loading;
mod perlin_noise;
mod perlin_noise_test;
mod procedural_gen_menu;
mod recursive_generation;
mod recursive_test;

use bevy::{prelude::*, window::{EnabledButtons, PresentMode}};

use crate::{common::AppState::{self}, credits::CreditsPlugin, game::GamePlugin, loading::LoadingPlugin, main_menu::MainMenuPlugin, settings::SettingsPlugin, test::TestPlugin};
use crate::camera::CameraPlugin;
use crate::combat::CombatPlugin;
use crate::player::PlayerPlugin;
use crate::procedural_test::ProceduralTestPlugin;
use crate::perlin_noise_test::PerlinNoiseTestPlugin;
use crate::procedural_gen_menu::ProcGenPlugin;
use crate::recursive_test::RecursiveTestPlugin;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
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
        .add_plugins((MainMenuPlugin, GamePlugin, LoadingPlugin, NetworkPlugin))
        // changed this to just add the plugins
        .add_plugins((MainMenuPlugin, CreditsPlugin, SettingsPlugin, TestPlugin, LoadingPlugin))
        // in-game: level + fighters + camera + combat
        .add_plugins((GamePlugin, PlayerPlugin, CameraPlugin, CombatPlugin))
        .add_plugins(ProceduralTestPlugin)
        .add_plugins(PerlinNoiseTestPlugin)
        .add_plugins(ProcGenPlugin)
        .add_plugins(RecursiveTestPlugin)
        .run();
}