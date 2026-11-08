use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;

use std::time::Instant;
use crate::common::{AppState, BACKGROUND_LAYER};
use crate::level_select::{LevelType, SelectedLevel};
use crate::loading::{LoadingAssets, despawn_with};
use crate::network::OnlineSession;
use crate::player::PendingInputs;
use crate::protocol::ONLINE_LEVEL;
use crate::sim::{
    MatchState, PlayerSide, SCREEN_COUNT, START_SCREEN, TICK_SECONDS, Terrain, TerrainKind,
    ThrownSword, DEFAULT_SEED,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .insert_resource(Winner(None))
            // the fixed clock ticks at exactly the simulation's rate
            .insert_resource(Time::<Fixed>::from_seconds(TICK_SECONDS as f64))
            .add_systems(Startup, load_level)
            .add_systems(OnEnter(AppState::InGame), (setup_game, setup_level))
            // online, the server runs the match instead — see network.rs
            .add_systems(
                FixedUpdate,
                step_match
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>)
                    .run_if(not(resource_exists::<OnlineSession>)),
            )
            .add_systems(
                Update,
                (
                    (update_background, check_winner)
                        .run_if(in_state(AppState::InGame))
                        .run_if(resource_exists::<Match>),
                    toggle_pause.run_if(in_state(AppState::InGame)),
                    pause_menu_interaction
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Paused)),
                ),
            )
            .add_systems(
                Update,
                time_presses
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>)
            )
            .add_systems(
                OnExit(AppState::InGame),
                (despawn_game, despawn_with::<Background>, remove_match),
            );
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

/// The background image, and which screen it's currently showing.
#[derive(Component)]
pub struct Background {
    screen: usize,
}

#[derive(Resource)]
pub struct Backgrounds(Vec<Handle<Image>>);

#[derive(Resource)]
pub struct Winner(pub Option<PlayerSide>);

/// Marks the sprites that draw the terrain.
#[derive(Component)]
struct TerrainSprite;

// ---------------------------------------------------------------- the match

/// The match being played.
///
/// `previous` is the state one tick earlier. The simulation only moves 60
/// times a second, but the screen may refresh faster, so sprites are drawn
/// partway between the two to keep movement smooth at any frame rate.
#[derive(Resource)]
pub struct Match {
    pub state: MatchState,
    pub previous: MatchState,
    pub terrain: Terrain,
}

/// A move bigger than this between two ticks is a teleport — a respawn or a
/// screen change — and is drawn as a jump instead of sliding across the screen.
const SNAP_DISTANCE: f32 = 100.0;

impl Match {
    pub fn new(terrain: Terrain) -> Self {
        let state = MatchState::new();
        Self {
            previous: state.clone(),
            state,
            terrain,
        }
    }

    /// Where to draw a fighter this frame: `alpha` of the way from the
    /// previous tick's position to the current one.
    pub fn drawn_position(&self, side: PlayerSide, alpha: f32) -> Vec2 {
        let before = self.previous.fighter(side);
        let now = self.state.fighter(side);
        blend(Vec2::new(before.x, before.y), Vec2::new(now.x, now.y), alpha)
    }

    /// The same for a thrown sword. One thrown this tick is drawn where it is.
    pub fn drawn_sword_position(&self, sword: &ThrownSword, alpha: f32) -> Vec2 {
        let now = Vec2::new(sword.x, sword.y);
        match self.previous.swords.iter().find(|s| s.id == sword.id) {
            Some(before) => blend(Vec2::new(before.x, before.y), now, alpha),
            None => now,
        }
    }
}

fn blend(before: Vec2, now: Vec2, alpha: f32) -> Vec2 {
    if before.distance(now) > SNAP_DISTANCE {
        now
    } else {
        before.lerp(now, alpha)
    }
}

// One simulation tick. This runs on the fixed clock, so the match advances at
// exactly the simulation's rate whatever the frame rate is.
fn step_match(
    mut game: ResMut<Match>,
    mut pending: ResMut<PendingInputs>,
    game_state: Res<State<GameState>>,
) {
    let Match {
        state,
        previous,
        terrain,
    } = &mut *game;
    *previous = state.clone();

    // paused: leave the match alone. previous now equals it, so the fighters
    // hold still instead of hovering between two ticks
    if *game_state.get() != GameState::Playing {
        return;
    }
    state.step(pending.take(), terrain);
}

// The match is won by leaving the outermost screen.
fn check_winner(
    game: Res<Match>,
    mut winner: ResMut<Winner>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    if let Some(side) = game.state.winner {
        winner.0 = Some(side);
        next_app_state.set(AppState::GameOver);
    }
}

fn remove_match(mut commands: Commands) {
    commands.remove_resource::<Match>();
}

// ---------------------------------------------------------------- level

// Background
const BG_OFFSET_Y: f32 = 65.0;

const BG_PATHS: &[&str] = &[
    "backgrounds/street.png",
    "backgrounds/hills.png", // start screen
    "white_grid_inverted.png",
];
// one background per screen — this fails to compile if the two drift apart
const _: () = assert!(BG_PATHS.len() == SCREEN_COUNT);

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
    commands.spawn((
        Sprite {
            image: backgrounds.0[START_SCREEN].clone(),
            ..default()
        },
        Transform::from_xyz(0., BG_OFFSET_Y, -1.),
        RenderLayers::layer(BACKGROUND_LAYER),
        Background {
            screen: START_SCREEN,
        },
    ));
}

// swap the background when the match moves to another screen
fn update_background(
    game: Res<Match>,
    backgrounds: Res<Backgrounds>,
    mut background: Query<(&mut Sprite, &mut Background)>,
) {
    let screen = game.state.current_screen;
    for (mut sprite, mut shown) in &mut background {
        if shown.screen == screen {
            continue;
        }
        shown.screen = screen;
        sprite.image = backgrounds.0[screen].clone();
    }
}

fn setup_game(
    mut commands: Commands,
    selected_level: Res<SelectedLevel>,
    online: Option<Res<OnlineSession>>,
    mut winner: ResMut<Winner>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    next_game_state.set(GameState::Playing);
    winner.0 = None;

    // online, everyone must collide with the same ground the server does
    let kind = if online.is_some() {
        ONLINE_LEVEL
    } else {
        match selected_level.0 {
            LevelType::Flat => TerrainKind::Flat,
            LevelType::Noise => TerrainKind::Noise,
            LevelType::Maze => TerrainKind::Maze,
        }
    };
    let terrain = Terrain::generate(kind, DEFAULT_SEED);

    // draw exactly the blocks the simulation collides with
    for block in &terrain.blocks {
        commands.spawn((
            Sprite {
                color: Color::srgb(0.25, 0.25, 0.3),
                custom_size: Some(Vec2::new(block.width, block.height)),
                ..default()
            },
            Transform::from_xyz(block.x, block.y, 0.0),
            TerrainSprite,
        ));
    }

    commands.insert_resource(Match::new(terrain));
}

// ---------------------------------------------------------------- pause

fn despawn_tree(commands: &mut Commands, entity: Entity, children: &Query<&Children>) {
    if let Ok(kids) = children.get(entity) {
        for kid in kids.iter() {
            despawn_tree(commands, kid, children);
        }
    }
    commands.entity(entity).despawn();
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
    query: Query<Entity, Or<(With<TerrainSprite>, With<PauseMenuRoot>)>>,
    children: Query<&Children>,
) {
    for entity in &query {
        despawn_tree(&mut commands, entity, &children);
    }
}

// Debug: run with --latency-test, only checks for D inputs, letting the
// fighter stop in between. Prints each press-to-screen time.
fn time_presses(
    keyboard: Res<ButtonInput<KeyCode>>,
    game: Res<Match>,
    mut started: Local<Option<Instant>>,
    mut total: Local<f64>,
    mut count: Local<u32>,
) {
    let was_still = game.previous.fighters.iter().all(|f| f.vx == 0.0);
    let moving = game.state.fighters.iter().any(|f| f.vx != 0.0);

    // start: D pressed while everyone was standing still
    if keyboard.just_pressed(KeyCode::KeyD) && was_still && started.is_none() {
        *started = Some(Instant::now());
    }

    // stop: the first frame a fighter is shown moving
    if let Some(start) = *started {
        if moving {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            *total += ms;
            *count += 1;
            println!("press {}: {ms:.1} ms (average {:.1} ms)", *count, *total / *count as f64);
            *started = None;
        }
    }
}