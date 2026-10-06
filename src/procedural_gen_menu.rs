use bevy::prelude::*;
use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};

pub struct ProcGenPlugin;

// Created plugin so main can connect to it
impl Plugin for ProcGenPlugin {
    fn build(&self, app: &mut App) {
        // main menu sys
        // Added spawn_ui_camera to this call since I moved it out from spawn_main_menu
        app.add_systems(OnEnter(AppState::ProcGenMenu), (spawn_ui_camera, spawn_main_menu))
        .add_systems(Update, button_interaction.run_if(in_state(AppState::ProcGenMenu)))
        .add_systems(OnExit(AppState::ProcGenMenu), (despawn_main_menu, despawn_ui_camera));
    }
}

#[derive(Component)]
struct ProcGenRoot;

#[derive(Component)]
struct DFSMazeTestButton;

#[derive(Component)]
struct NoiseTestButton;

#[derive(Component)]
struct RDMazeTestButton;

#[derive(Component)]
struct ExitButton;

// Removed camera spawning from fn, moved to separate fn in common.rs
fn spawn_main_menu(mut commands: Commands) {
    // if print 1 it is ok
    println!("1");

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Default,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        ProcGenRoot,
    ))
    .with_children(|parent| {
        parent.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });

        parent
            .spawn((
                Button,
                DFSMazeTestButton,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.45, 0.75)),
                BorderColor::all(Color::srgb(0.4, 0.7, 0.95)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("DFS Maze Test"),
                    TextFont {
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        
        parent
            .spawn((
                Button,
                RDMazeTestButton,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.45, 0.75)),
                BorderColor::all(Color::srgb(0.4, 0.7, 0.95)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("RD Maze Test"),
                    TextFont {
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        
        parent
            .spawn((
                Button,
                NoiseTestButton,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.45, 0.75)),
                BorderColor::all(Color::srgb(0.4, 0.7, 0.95)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("Noise Test"),
                    TextFont {
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

        parent.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });

        // 左下角按钮行
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
    mut dfs_maze_test_q: Query<&Interaction, (Changed<Interaction>, With<DFSMazeTestButton>)>,
    mut noise_test_q: Query<&Interaction, (Changed<Interaction>, With<NoiseTestButton>)>,
    mut rd_maze_test_q: Query<&Interaction, (Changed<Interaction>, With <RDMazeTestButton>)>,
    mut exit_q: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for interaction in &mut dfs_maze_test_q {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::DFSMazeTesting);
        }
    }
    for interaction in &mut noise_test_q {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::PNoiseTesting);
        }
    }
    for interaction in &mut rd_maze_test_q {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::RDMazeTesting);
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
    query_root: Query<Entity, With<ProcGenRoot>>,
) {
    for entity in &query_root {
        commands.entity(entity).despawn();
    }
}
