use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};

pub struct MainMenuPlugin;

// Created plugin so main can connect to it
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        // main menu sys
        // Added spawn_ui_camera to this call since I moved it out from spawn_main_menu
        app.add_systems(OnEnter(AppState::MainMenu), (spawn_ui_camera, spawn_main_menu))
        .add_systems(Update, button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), (despawn_main_menu, despawn_ui_camera));
    }
}

#[derive(Component)]
struct MainMenuRoot;

// Removed camera spawning from fn, moved to separate fn in common.rs
fn spawn_main_menu(mut commands: Commands) {
    // if print 1 it is ok
    println!("1");

    commands.spawn((
        // root Node
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        MainMenuRoot,
    ))
    .with_children(|parent| {
        parent.spawn((
            // button position
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(90.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.5, 0.95)),
            BorderColor::all(Color::srgb(0.5, 0.7, 1.0)),
        ))
        .with_children(|parent| {
            // button word
            parent.spawn((
                Text::new("Credits"),
                TextFont {
                    font_size: FontSize::Px(36.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

// handle interact with bottons
fn button_interaction(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &mut interaction_query {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::Credits);
        }
    }
}

// exit main menu and delete camera
// Removed camera despawning from fn
fn despawn_main_menu(
    mut commands: Commands,
    query_root: Query<Entity, With<MainMenuRoot>>,
) {
    for entity in &query_root {
        commands.entity(entity).despawn();
    }
}