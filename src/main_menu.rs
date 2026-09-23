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

#[derive(Component)]
struct CreditsButton;

#[derive(Component)]
struct SettingsButton;

#[derive(Component)]
struct ExitButton;

// Removed camera spawning from fn, moved to separate fn in common.rs
fn spawn_main_menu(mut commands: Commands) {
    // if print 1 it is ok
    println!("1");

    commands.spawn((
        // root Node
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::End,
            justify_content: JustifyContent::Start,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        MainMenuRoot,
    ))
    .with_children(|parent| {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                margin: UiRect::all(Val::Px(40.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Button,
                    CreditsButton,
                    menu_button_node(),
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.95)),
                    BorderColor::all(Color::srgb(0.5, 0.7, 1.0)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Credits"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                row.spawn((
                    Button,
                    SettingsButton,
                    menu_button_node(),
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                    BorderColor::all(Color::srgb(0.5, 0.5, 0.55)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                row.spawn((
                    Button,
                    ExitButton,
                    menu_button_node(),
                    BackgroundColor(Color::srgb(0.7, 0.2, 0.2)),
                    BorderColor::all(Color::srgb(0.9, 0.4, 0.4)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Exit"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            });
    });
}

// 统一的按钮尺寸
fn menu_button_node() -> Node {
    Node {
        width: Val::Px(220.0),
        height: Val::Px(80.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border: UiRect::all(Val::Px(2.0)),
        ..default()
    }
}

// handle interact with bottons
fn button_interaction(
    mut credits_q: Query<&Interaction, (Changed<Interaction>, With<CreditsButton>)>,
    mut settings_q: Query<&Interaction, (Changed<Interaction>, With<SettingsButton>)>,
    mut exit_q: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for interaction in &mut credits_q {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::Credits);
        }
    }
    for interaction in &mut settings_q {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::Settings);
        }
    }
    for interaction in &mut exit_q {
        if let Interaction::Pressed = *interaction {
            exit.write(AppExit::Success);
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