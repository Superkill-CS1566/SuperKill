use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};
use crate::game::Winner;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameOver),
            (spawn_ui_camera, setup_game_over),
        )
        .add_systems(
            Update,
            menu_button_interaction.run_if(in_state(AppState::GameOver)),
        )
        .add_systems(
            OnExit(AppState::GameOver),
            (despawn_game_over, despawn_ui_camera),
        );
    }
}

#[derive(Component)]
struct GameOverRoot;

#[derive(Component)]
enum GameOverButton {
    BackToMenu,
}

fn setup_game_over(mut commands: Commands, winner: Res<Winner>) {
    let headline = match winner.0 {
        Some(side) => format!("{} Wins", side.label()),
        None => "Draw".to_string(),
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(32.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            GameOverRoot,
        ))
        .with_children(|parent| {
                        parent.spawn((
                Text::new(headline),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent
                .spawn((
                    Button,
                    GameOverButton::BackToMenu,
                    Node {
                        width: Val::Px(260.0),
                        height: Val::Px(70.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.95)),
                    BorderColor::all(Color::srgb(0.5, 0.7, 1.0)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Back to Menu"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn menu_button_interaction(
    mut query: Query<(&Interaction, &GameOverButton), (Changed<Interaction>, With<Button>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut query {
        if let Interaction::Pressed = *interaction {
            match button {
                GameOverButton::BackToMenu => {
                    next_app_state.set(AppState::MainMenu);
                }
            }
        }
    }
}

fn despawn_game_over(mut commands: Commands, query: Query<Entity, With<GameOverRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}