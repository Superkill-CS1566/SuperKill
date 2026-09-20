mod common;
mod credits;
mod main_menu;
mod settings;

use bevy::{prelude::*, window::{EnabledButtons, PresentMode}};

use crate::{common::AppState, credits::CreditsPlugin, main_menu::MainMenuPlugin, settings::SettingsPlugin};


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
        .add_plugins((MainMenuPlugin, CreditsPlugin, SettingsPlugin))
        .run();
}