use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(OnEnter(AppState::InGame), (spawn_ui_camera, setup_game))
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
                    update_player_visual.run_if(in_state(AppState::InGame)),
                    toggle_pause.run_if(in_state(AppState::InGame)),
                    pause_menu_interaction
                        .run_if(in_state(AppState::InGame))
                        .run_if(in_state(GameState::Paused)),
                ),
            )
            .add_systems(OnExit(AppState::InGame), (despawn_game, despawn_ui_camera));
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

const GRAVITY: f32 = 1800.0;
const WORLD_WIDTH: f32 = 1280.0;
const WORLD_HEIGHT: f32 = 720.0;

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

    spawn_platform(&mut commands, 0.0, -320.0, 1280.0, 40.0, Color::srgb(0.25, 0.25, 0.3));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.75, 0.35),
            custom_size: Some(Vec2::new(40.0, 60.0)),
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, -270.0, 1.0),
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

        new_x = new_x.clamp(-WORLD_WIDTH / 2.0 + half_w, WORLD_WIDTH / 2.0 - half_w);

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
        if new_y < -WORLD_HEIGHT / 2.0 - 100.0 {
            new_y = -270.0;
            transform.translation.x = 0.0;
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
