use bevy::prelude::*;

use crate::combat::Dead;
use crate::common::{AppState, Platform, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN, WIN_H};
use crate::game::GameState;
use crate::loading::despawn_with;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_players)
            .add_systems(
                Update,
                (player_movement, apply_gravity, move_and_collide)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                update_player_visual.run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), despawn_with::<Player>);
    }
}

// which fighter this is — left starts on the left, right on the right
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerSide {
    Left,
    Right,
}

// 每个玩家自己的按键绑定
#[derive(Component)]
pub struct Controls {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub crouch: KeyCode,
    pub attack: KeyCode,
}

// 玩家组件
#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub jump_force: f32,
    pub is_grounded: bool,
    pub is_crouching: bool,
    pub prev_crouching: bool,
    pub velocity: Vec2,
    pub facing: f32,    // -1.0 = facing left 1.0 = facing right
    pub width: f32,
    pub height: f32,
    pub crouch_height: f32,
}

const GRAVITY: f32 = 1800.0;
const TERMINAL_VELOCITY: f32 = -1200.0;

// Player Constants
const PLAYER_WIDTH: f32 = 40.0;
const PLAYER_HEIGHT: f32 = 60.0;
const CROUCH_HEIGHT: f32 = 35.0;
const PLAYER_SPEED: f32 = 320.0;
const JUMP_FORCE: f32 = 620.0;

// how far from center each fighter starts
const SPAWN_OFFSET_X: f32 = 200.0;
pub const PLAYER_SPAWN_Y: f32 = GROUND_Y + GROUND_THICKNESS / 2.0 + PLAYER_HEIGHT / 2.0;

impl PlayerSide {
    pub fn spawn_x(self) -> f32 {
        match self {
            PlayerSide::Left => -SPAWN_OFFSET_X,
            PlayerSide::Right => SPAWN_OFFSET_X,
        }
    }

    pub fn other(self) -> Self {
        match self {
            PlayerSide::Left => PlayerSide::Right,
            PlayerSide::Right => PlayerSide::Left,
        }
    }

    // left fighter wins by pushing right, right fighter by pushing left
    pub fn advance_dir(self) -> f32 {
        match self {
            PlayerSide::Left => 1.0,
            PlayerSide::Right => -1.0,
        }
    }

    fn color(self) -> Color {
        match self {
            PlayerSide::Left => Color::srgb(0.2, 0.75, 0.35),
            PlayerSide::Right => Color::srgb(0.35, 0.2, 0.75),
        }
    }

    fn controls(self) -> Controls {
        match self {
            PlayerSide::Left => Controls {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                jump: KeyCode::KeyW,
                crouch: KeyCode::KeyS,
                attack: KeyCode::KeyG,
            },
            PlayerSide::Right => Controls {
                left: KeyCode::ArrowLeft,
                right: KeyCode::ArrowRight,
                jump: KeyCode::ArrowUp,
                crouch: KeyCode::ArrowDown,
                attack: KeyCode::KeyH,
            },
        }
    }
}

fn spawn_players(mut commands: Commands) {
    for side in [PlayerSide::Left, PlayerSide::Right] {
        spawn_player(&mut commands, side);
    }
}

fn spawn_player(commands: &mut Commands, side: PlayerSide) {
    commands.spawn((
        Sprite {
            color: side.color(),
            custom_size: Some(Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT)),
            ..default()
        },
        Transform {
            translation: Vec3::new(side.spawn_x(), PLAYER_SPAWN_Y, 1.0),
            ..default()
        },
        GlobalTransform::default(),
        Player {
            speed: PLAYER_SPEED,
            jump_force: JUMP_FORCE,
            is_grounded: false,
            is_crouching: false,
            prev_crouching: false,
            velocity: Vec2::ZERO,
            facing: side.advance_dir(),
            width: PLAYER_WIDTH,
            height: PLAYER_HEIGHT,
            crouch_height: CROUCH_HEIGHT,
        },
        side,
        side.controls(),
    ));
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Player, &mut Transform, &Controls), Without<Dead>>,
) {
    for (mut player, mut transform, controls) in &mut query {
        let now_crouching = keyboard.pressed(controls.crouch);

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
        if keyboard.pressed(controls.left) {
            move_dir -= 1.0;
        }
        if keyboard.pressed(controls.right) {
            move_dir += 1.0;
        }

        // keep facing the last direction walked
        if move_dir != 0.0 {
            player.facing = move_dir;
        }

        // 下蹲时移动速度减半
        let speed = if player.is_crouching {
            player.speed * 0.5
        } else {
            player.speed
        };
        player.velocity.x = move_dir * speed;

        if keyboard.just_pressed(controls.jump) && player.is_grounded && !player.is_crouching {
            player.velocity.y = player.jump_force;
            player.is_grounded = false;
        }
    }
}

fn apply_gravity(time: Res<Time>, mut query: Query<&mut Player, Without<Dead>>) {
    for mut player in &mut query {
        if !player.is_grounded {
            player.velocity.y -= GRAVITY * time.delta().as_secs_f32();
            // 终端速度限制
            player.velocity.y = player.velocity.y.max(TERMINAL_VELOCITY);
        }
    }
}

pub fn move_and_collide(
    time: Res<Time>,
    mut player_query: Query<(&mut Player, &mut Transform, &PlayerSide), Without<Dead>>,
    platform_query: Query<(&Platform, &Transform), Without<Player>>,
) {
    let delta = time.delta().as_secs_f32();

    for (mut player, mut transform, side) in &mut player_query {
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
            let overlaps_y = (new_y - half_h) <= (py + ph) && (new_y + half_h) >= (py - ph);

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

        // Respawn Logic — each fighter returns to their own side
        if new_y < -WIN_H / 2.0 - 100.0 {
            new_y = PLAYER_SPAWN_Y;
            transform.translation.x = side.spawn_x();
            player.velocity = Vec2::ZERO;
        }
        transform.translation.y = new_y;
    }
}

fn update_player_visual(mut query: Query<(&Player, &mut Sprite), Without<Dead>>) {
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