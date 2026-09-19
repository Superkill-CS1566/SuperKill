mod common;
mod credits;
mod main_menu;

use bevy::{prelude::*, window::PresentMode};

use crate::{common::AppState, credits::CreditsPlugin, main_menu::MainMenuPlugin};


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "End Credits for SuperKill".into(),
                resolution: (1280, 720).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        // changed this to just add the plugins
        .add_plugins((MainMenuPlugin, CreditsPlugin))
        .run();
}