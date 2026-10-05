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
        .run();
}
