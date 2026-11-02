use bevy::prelude::*;

use crate::common::AppState;

#[derive(Clone, Copy, PartialEq, Eq, Resource)]
pub enum LevelType {
    Flat,
    Noise,
    Maze,
}

// Tracks which generation type the player selected
#[derive(Resource)]
pub struct SelectedLevel(pub LevelType);

// Identifies the buttons in the level selection menu
#[derive(Component)]
struct LevelSelectButton(LevelType);

// Identifies menu entities so we can clean them up
#[derive(Component)]
struct LevelSelectUI;

pub struct LevelSelectPlugin;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedLevel>()
            .add_systems(OnEnter(AppState::LevelSelect), setup_level_select)
            .add_systems(
                Update,
                select_level.run_if(in_state(AppState::LevelSelect)),
            )
            .add_systems(
                OnExit(AppState::LevelSelect),
                cleanup_level_select,
            );
    }
}

impl Default for SelectedLevel {
    fn default() -> Self {
        Self(LevelType::Flat)
    }
}

fn setup_level_select(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        LevelSelectUI,
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("SELECT LEVEL TYPE"),
            TextFont {
                font_size: FontSize::Px(40.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));

        spawn_level_button(parent, "Flat Terrain", LevelType::Flat);
        spawn_level_button(parent, "Noise Terrain", LevelType::Noise);
        spawn_level_button(parent, "Maze Generation", LevelType::Maze);
    });
}

fn spawn_level_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    level_type: LevelType,
) {
    parent.spawn((
        Button,
        Node {
            width: Val::Px(300.0),
            height: Val::Px(65.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.28)),
        LevelSelectButton(level_type),
    ))
    .with_children(|button| {
        button.spawn((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn select_level(
    mut interaction_query: Query<
        (&Interaction, &LevelSelectButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut selected_level: ResMut<SelectedLevel>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected_level.0 = button.0;
                next_state.set(AppState::Loading);
            }

            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.35, 0.35, 0.45));
            }

            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.2, 0.28));
            }
        }
    }
}

fn cleanup_level_select(
    mut commands: Commands,
    query: Query<Entity, With<LevelSelectUI>>,
    cameras: Query<Entity, With<Camera2d>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}