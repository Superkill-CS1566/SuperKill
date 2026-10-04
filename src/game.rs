use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, UiCamera, AppState, LEVEL_LEN, WIN_W, WIN_H};
use crate::loading::{LoadingAssets, despawn_with};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Startup, load_level)
            .add_systems(OnEnter(AppState::InGame), 
                ((spawn_ui_camera, init_camera_zoom).chain(), setup_game, setup_level),
            )
            .add_systems(
                Update,
                (
                    player_movement
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .before(apply_gravity),
                    apply_gravity
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .before(move_and_collide),
                    move_and_collide
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .before(update_player_visual),
                    screen_transition
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .after(move_and_collide),
                    camera_follow
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Playing))
                        .after(screen_transition),
                    update_player_visual.run_if(in_state(AppState::InGame)),
                    toggle_pause.run_if(in_state(AppState::InGame)),
                    pause_menu_interaction
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Paused)),
                ),
            )
            .add_systems(OnExit(AppState::InGame), (despawn_game, despawn_with::<Background>, despawn_ui_camera));
    }
}

#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
}

// 玩家组件
#[derive(Component)]
struct Player {
    speed: f32,
    jump_force: f32,
    is_grounded: bool,
    is_crouching: bool,
    prev_crouching: bool,
    velocity: Vec2,
    width: f32,
    height: f32,
    crouch_height: f32,
}

#[derive(Component)]
struct Platform {
    width: f32,
    height: f32,
}

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
enum PauseButton {
    Resume,
    Quit,
}

#[derive(Component)]
struct Background;

#[derive(Resource)]
pub struct Backgrounds(Vec<Handle<Image>>);

#[derive(Resource)]
struct CurrentScreen(usize);


const GRAVITY: f32 = 1800.0;
// replaced world height/width withh WIN_H and WIN_W in common

// Camera and Background
const ZOOM: f32 = 0.80;      // 0.5 = 2× zoomed in
const BG_OFFSET_Y: f32 = 65.0;
const SCREEN_W: f32 = WIN_W * ZOOM; 
const SCREEN_INSET: f32 = 20.0;
const START_SCREEN: usize = 1;
const BG_PATHS: &[&str] = &[
    "landscape_planets_stars.png",
    "RainyNeonTokyoAlley.png",
    "lawn_forest_mountains.png",
];

// Default Platform
const GROUND_Y: f32 = -300.0;        // center of the ground platform
const GROUND_THICKNESS: f32 = 60.0;

// Player Constants
const PLAYER_HEIGHT: f32 = 60.0;
const PLAYER_SPAWN_X: f32 = 0.0;

const PLAYER_SPAWN_Y: f32 = GROUND_Y + GROUND_THICKNESS / 2.0 + PLAYER_HEIGHT / 2.0;

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

fn screen_transition(
    mut current: ResMut<CurrentScreen>,
    backgrounds: Res<Backgrounds>,
    mut player_q: Query<(&mut Player, &mut Transform)>,
    mut bg_q: Query<&mut Sprite, With<Background>>,
) {
    let Ok((mut player, mut transform)) = player_q.single_mut() else { return; };
    let Ok(mut sprite) = bg_q.single_mut() else { return; };

    // edge of the background image, not the edge of the view
    let edge = LEVEL_LEN / 2.0 - player.width / 2.0;
    let x = transform.translation.x;

    if x >= edge && current.0 + 1 < backgrounds.0.len() {
        current.0 += 1;
        transform.translation.x = -edge + SCREEN_INSET;
    } else if x <= -edge && current.0 > 0 {
        current.0 -= 1;
        transform.translation.x = edge - SCREEN_INSET;
    } else {
        return;
    }

    sprite.image = backgrounds.0[current.0].clone();
    player.velocity.x = 0.0;
}

// camera pans across the current background, stopping at its edges
fn camera_follow(
    player: Single<&Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<UiCamera>, Without<Player>)>,
) {
    let half_view = SCREEN_W / 2.0;
    camera.translation.x = player.translation.x.clamp(
        -LEVEL_LEN / 2.0 + half_view,
        LEVEL_LEN / 2.0 - half_view,
    );
}

fn init_camera_zoom(mut camera: Single<&mut Projection, With<UiCamera>>) {
    if let Projection::Orthographic(ref mut ortho) = **camera {
        ortho.scale = ZOOM;
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

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.75, 0.35),
            custom_size: Some(Vec2::new(40.0, PLAYER_HEIGHT)),
            ..default()
        },
        Transform {
            translation: Vec3::new(PLAYER_SPAWN_X, PLAYER_SPAWN_Y, 1.0),
            ..default()
        },
        GlobalTransform::default(),
        Player {
            speed: 320.0,
            jump_force: 620.0,
            is_grounded: false,
            is_crouching: false,
            prev_crouching: false,
            velocity: Vec2::ZERO,
            width: 40.0,
            height: 60.0,
            crouch_height: 35.0,
        },
    ));
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

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Player, &mut Transform)>,
) {
    for (mut player, mut transform) in &mut query {
        let now_crouching =
            keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);

        if player.is_grounded {
            let half_delta = (player.height - player.crouch_height) / 2.0;
            if player.prev_crouching && !now_crouching {

                transform.translation.y += half_delta;
            } else if !player.prev_crouching && now_crouching {

                transform.translation.y -= half_delta;
            }
        }
        player.prev_crouching = now_crouching;
        player.is_crouching = now_crouching;


        let mut move_dir = 0.0;
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            move_dir -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            move_dir += 1.0;
        }

        // 下蹲时移动速度减半
        let speed = if player.is_crouching {
            player.speed * 0.5
        } else {
            player.speed
        };
        player.velocity.x = move_dir * speed;

        if (keyboard.just_pressed(KeyCode::KeyW)
            || keyboard.just_pressed(KeyCode::ArrowUp)
            || keyboard.just_pressed(KeyCode::Space))
            && player.is_grounded
            && !player.is_crouching
        {
            player.velocity.y = player.jump_force;
            player.is_grounded = false;
        }
    }
}

fn apply_gravity(time: Res<Time>, mut query: Query<&mut Player>) {
    for mut player in &mut query {
        if !player.is_grounded {
            player.velocity.y -= GRAVITY * time.delta().as_secs_f32();
            // 终端速度限制
            player.velocity.y = player.velocity.y.max(-1200.0);
        }
    }
}

fn move_and_collide(
    time: Res<Time>,
    mut player_query: Query<(&mut Player, &mut Transform)>,
    platform_query: Query<(&Platform, &Transform), Without<Player>>,
) {
    let delta = time.delta().as_secs_f32();

    for (mut player, mut transform) in &mut player_query {
        let player_h = if player.is_crouching {
            player.crouch_height
        } else {
            player.height
        };
        let half_w = player.width / 2.0;
        let half_h = player_h / 2.0;

        let mut new_x = transform.translation.x + player.velocity.x * delta;

        new_x = new_x.clamp(-LEVEL_LEN / 2.0 + half_w, LEVEL_LEN / 2.0 - half_w);

        for (platform, p_transform) in &platform_query {
            let px = p_transform.translation.x;
            let py = p_transform.translation.y;
            let pw = platform.width / 2.0;
            let ph = platform.height / 2.0;

            let overlaps_x = (new_x - half_w) <= (px + pw) && (new_x + half_w) >= (px - pw);
            let overlaps_y = (transform.translation.y - half_h) < (py + ph)
                && (transform.translation.y + half_h) > (py - ph);

            if overlaps_x && overlaps_y {
                if transform.translation.x < px {
                    new_x = px - pw - half_w; // 从左侧撞，推到平台左边
                } else {
                    new_x = px + pw + half_w; // 从右侧撞，推到平台右边
                }
                player.velocity.x = 0.0;
            }
        }
        transform.translation.x = new_x;

        let mut new_y = transform.translation.y + player.velocity.y * delta;
        player.is_grounded = false;

        for (platform, p_transform) in &platform_query {
            let px = p_transform.translation.x;
            let py = p_transform.translation.y;
            let pw = platform.width / 2.0;
            let ph = platform.height / 2.0;

            let overlaps_x = (transform.translation.x - half_w) <= (px + pw)
                && (transform.translation.x + half_w) >= (px - pw);
            let overlaps_y =
                (new_y - half_h) <= (py + ph) && (new_y + half_h) >= (py - ph);

            if overlaps_x && overlaps_y {
                if player.velocity.y <= 0.0 && transform.translation.y >= py {
                    new_y = py + ph + half_h;
                    player.velocity.y = 0.0;
                    player.is_grounded = true;
                } else if player.velocity.y > 0.0 && transform.translation.y < py {
                    new_y = py - ph - half_h;
                    player.velocity.y = 0.0;
                }
            }
        }

        // Respawn Logic
        if new_y < -WIN_H / 2.0 - 100.0 {
            new_y = PLAYER_SPAWN_Y;
            transform.translation.x = PLAYER_SPAWN_X;
            player.velocity = Vec2::ZERO;
        }
        transform.translation.y = new_y;
    }
}

fn update_player_visual(mut query: Query<(&Player, &mut Sprite)>) {
    for (player, mut sprite) in &mut query {
        let target_h = if player.is_crouching {
            player.crouch_height
        } else {
            player.height
        };
        if let Some(size) = &mut sprite.custom_size {
            size.y = target_h;
        }
    }
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
    query: Query<Entity, Or<(With<Player>, With<Platform>, With<PauseMenuRoot>)>>,
    children: Query<&Children>,
) {
    for entity in &query {
        despawn_tree(&mut commands, entity, &children);
    }
}