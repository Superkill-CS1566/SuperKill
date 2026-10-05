mod common;
mod credits;
mod game;
mod main_menu;
mod procedural_generation;
mod procedural_test;
mod settings;
mod test;
mod loading;
mod network;

use bevy::{prelude::*, window::{EnabledButtons, PresentMode}};

use crate::{common::AppState, credits::CreditsPlugin, game::GamePlugin, main_menu::MainMenuPlugin, settings::SettingsPlugin, test::TestPlugin, loading::LoadingPlugin};
use crate::procedural_test::ProceduralTestPlugin;
use crate::network::NetworkPlugin;


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
        // changed this to just add the plugins
        .add_plugins((MainMenuPlugin, CreditsPlugin, SettingsPlugin, GamePlugin, TestPlugin, LoadingPlugin))
        .add_plugins(ProceduralTestPlugin)
        .add_plugins(NetworkPlugin)
        .run();
}
