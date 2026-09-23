use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), (spawn_ui_camera, spawn_settings))
            .add_systems(
                Update,
                (
                    settings_buttons.run_if(in_state(AppState::Settings)),
                    lock_1280x720_when_fullscreen,
                ),
            )
            .add_systems(OnExit(AppState::Settings), (despawn_ui_camera, despawn_settings));
    }
}

#[derive(Component)]
struct SettingsRoot;

#[derive(Component)]
enum SettingsEntry {
    Fullscreen,
    Windowed,
    Back,
}

fn spawn_settings(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.18)),
            SettingsRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Settings"),
                TextFont {
                    font_size: FontSize::Px(44.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent
                .spawn(row_node())
                .with_children(|row| {
                    row.spawn((
                        Text::new("Display Mode"),
                        TextFont {
                            font_size: FontSize::Px(26.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.85, 0.85)),
                    ));

                    row.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|btns| {
                        btns
                            .spawn((
                                Button,
                                SettingsEntry::Fullscreen,
                                entry_button_node(),
                                BackgroundColor(Color::srgb(0.2, 0.55, 0.4)),
                            ))
                            .with_children(|b| {
                                b.spawn((
                                    Text::new("Fullscreen"),
                                    TextFont {
                                        font_size: FontSize::Px(22.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        btns
                            .spawn((
                                Button,
                                SettingsEntry::Windowed,
                                entry_button_node(),
                                BackgroundColor(Color::srgb(0.2, 0.4, 0.8)),
                            ))
                            .with_children(|b| {
                                b.spawn((
                                    Text::new("Windowed"),
                                    TextFont {
                                        font_size: FontSize::Px(22.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    });
                });

            parent
                .spawn((
                    Button,
                    SettingsEntry::Back,
                    Node {
                        width: Val::Px(160.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.3, 0.3)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Back"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn row_node() -> Node {
    Node {
        width: Val::Px(460.0),
        height: Val::Px(56.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    }
}

fn entry_button_node() -> Node {
    Node {
        width: Val::Px(140.0),
        height: Val::Px(44.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn settings_buttons(
    mut q: Query<(&Interaction, &SettingsEntry), (Changed<Interaction>, With<Button>)>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (inter, entry) in &mut q {
        if *inter != Interaction::Pressed {
            continue;
        }
        match entry {
            SettingsEntry::Fullscreen => {
                if let Ok(mut w) = window.single_mut() {
                    w.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
                }
            }
            SettingsEntry::Windowed => {
                if let Ok(mut w) = window.single_mut() {
                    w.mode = WindowMode::Windowed;
                    w.resolution.set_scale_factor(1.0);
                    w.resolution.set(1280.0, 720.0);
                }
            }
            SettingsEntry::Back => {
                next_state.set(AppState::MainMenu);
            }
        }
    }
}

fn lock_1280x720_when_fullscreen(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut w) = window.single_mut() {
        if matches!(w.mode, WindowMode::BorderlessFullscreen(_)) {
            let target_sf = w.physical_width() as f32 / 1280.0;
            if (w.resolution.scale_factor() - target_sf).abs() > 0.01 {
                w.resolution.set_scale_factor(target_sf);
            }
        }
    }
}

fn despawn_settings(mut commands: Commands, q: Query<Entity, With<SettingsRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
