use bevy::prelude::*;

use crate::common::{AppState, Platform, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN};
use crate::combat::{Dead, Priority};
use crate::loading::{LoadingAssets, despawn_with};
use crate::player::{move_and_collide, Player, PlayerSide};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Startup, load_level)
            .add_systems(OnEnter(AppState::InGame), (setup_game, setup_level))
            .add_systems(
                Update,
                (
                    screen_transition
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .after(move_and_collide),
                    toggle_pause.run_if(in_state(AppState::InGame)),
                    pause_menu_interaction
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Paused)),
                ),
            )
            .add_systems(OnExit(AppState::InGame), (despawn_game, despawn_with::<Background>));
    }
}

#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
}

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
enum PauseButton {
    Resume,
    Quit,
}

#[derive(Component)]
pub struct Background;

#[derive(Resource)]
pub struct Backgrounds(Vec<Handle<Image>>);

#[derive(Resource)]
pub struct CurrentScreen(usize);


// replaced world height/width withh WIN_H and WIN_W in common
// GRAVITY and the player constants now live in player.rs
// ZOOM / SCREEN_W and the camera systems now live in camera.rs
// GROUND_Y / GROUND_THICKNESS / Platform now live in common.rs

// Background
const BG_OFFSET_Y: f32 = 65.0;
const SCREEN_INSET: f32 = 20.0;
const START_SCREEN: usize = 1;
const BG_PATHS: &[&str] = &[
    "white_grid_inverted.png",
    "white_grid.png",   // start screen
    "white_grid_inverted.png",
];

// add stuff for loading in foreground later for now just background 
// might put it in separate file since its more a part of procedural gen
fn load_level(
    mut commands: Commands, 
    asset_server: Res<AssetServer>, 
    mut loading_assets: ResMut<LoadingAssets>,
) {
    let mut handles = Vec::new();
    for path in BG_PATHS {
        let handle: Handle<Image> = asset_server.load(*path);
        loading_assets.0.push(handle.clone().untyped());
        handles.push(handle);
    }
    commands.insert_resource(Backgrounds(handles));
}

fn setup_level(mut commands: Commands, backgrounds: Res<Backgrounds>) {
    commands.insert_resource(CurrentScreen(START_SCREEN));
    commands.spawn((
        Sprite::from_image(backgrounds.0[START_SCREEN].clone()),
        Transform::from_xyz(0., BG_OFFSET_Y, -1.),
        Background,
    ));
}

// only the fighter holding priority may cross, and only toward their own goal
pub fn screen_transition(
    mut current: ResMut<CurrentScreen>,
    backgrounds: Res<Backgrounds>,
    priority: Res<Priority>,
    mut players: Query<(&mut Player, &mut Transform, &PlayerSide), Without<Dead>>,
    mut bg_q: Query<&mut Sprite, With<Background>>,
) {
    // neutral — nobody advances
    let Some(holder) = priority.0 else { return; };

    let dir = holder.advance_dir();
    // edge of the background image, not the edge of the view
    let edge = LEVEL_LEN / 2.0;

    let at_edge = players.iter().any(|(p, t, s)| {
        *s == holder && t.translation.x * dir >= edge - p.width / 2.0 - 1.0
    });
    if !at_edge {
        return;
    }

    let next = if dir > 0.0 {
        if current.0 + 1 >= backgrounds.0.len() {
            return; // outermost screen, no further
        }
        current.0 + 1
    } else {
        if current.0 == 0 {
            return;
        }
        current.0 - 1
    };
    current.0 = next;

    // leader enters from behind; the other fighter appears ahead of them
    for (mut player, mut transform, side) in &mut players {
        let sign = if *side == holder { -1.0 } else { 1.0 };
        transform.translation.x = sign * (edge - SCREEN_INSET) * dir;
        player.velocity.x = 0.0;
    }

    if let Ok(mut sprite) = bg_q.single_mut() {
        sprite.image = backgrounds.0[current.0].clone();
    }
}


fn despawn_tree(commands: &mut Commands, entity: Entity, children: &Query<&Children>) {
    if let Ok(kids) = children.get(entity) {
        for kid in kids.iter() {
            despawn_tree(commands, kid, children);
        }
    }
    commands.entity(entity).despawn();
}

fn setup_game(mut commands: Commands, mut next_game_state: ResMut<NextState<GameState>>) {

    next_game_state.set(GameState::Playing);

    spawn_platform(&mut commands, 0.0, GROUND_Y, LEVEL_LEN, GROUND_THICKNESS, Color::srgb(0.25, 0.25, 0.3));
}

fn spawn_platform(
    commands: &mut Commands,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform {
            translation: Vec3::new(x, y, 0.0),
            ..default()
        },
        GlobalTransform::default(),
        Platform { width, height },
    ));
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    game_state: Res<State<GameState>>,
    mut commands: Commands,
    pause_menu_query: Query<Entity, With<PauseMenuRoot>>,
    children: Query<&Children>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match game_state.get() {
            GameState::Playing => {
                next_game_state.set(GameState::Paused);
                spawn_pause_menu(&mut commands);
            }
            GameState::Paused => {
                next_game_state.set(GameState::Playing);
                for entity in &pause_menu_query {
                    despawn_tree(&mut commands, entity, &children);
                }
            }
        }
    }
}

fn spawn_pause_menu(commands: &mut Commands) {
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            PauseMenuRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Paused"),
                TextFont {
                    font_size: FontSize::Px(48.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent
                .spawn((
                    Button,
                    PauseButton::Resume,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(60.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.3)),
                    BorderColor::all(Color::srgb(0.4, 0.8, 0.5)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Resume"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((
                    Button,
                    PauseButton::Quit,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(60.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.7, 0.2, 0.2)),
                    BorderColor::all(Color::srgb(0.9, 0.4, 0.4)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Quit to Menu"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn pause_menu_interaction(
    mut query: Query<(&Interaction, &PauseButton), (Changed<Interaction>, With<Button>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    pause_menu_query: Query<Entity, With<PauseMenuRoot>>,
    children: Query<&Children>,
) {
    for (interaction, button) in &mut query {
        if let Interaction::Pressed = *interaction {
            match button {
                PauseButton::Resume => {
                    next_game_state.set(GameState::Playing);
                    for entity in &pause_menu_query {
                        despawn_tree(&mut commands, entity, &children);
                    }
                }
                PauseButton::Quit => {
                    next_game_state.set(GameState::Playing); // 重置，下次进游戏是 Playing
                    next_app_state.set(AppState::MainMenu);
                }
            }
        }
    }
}

fn despawn_game(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Platform>, With<PauseMenuRoot>)>>,
    children: Query<&Children>,
) {
    for entity in &query {
        despawn_tree(&mut commands, entity, &children);
    }
}