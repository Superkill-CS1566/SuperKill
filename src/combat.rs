use bevy::prelude::*;

use crate::camera::{camera_follow, SCREEN_W};
use crate::common::{AppState, UiCamera};
use crate::loading::despawn_with;
use crate::game::{CurrentScreen, GameState};
use crate::player::{Controls, Player, PlayerSide, PLAYER_SPAWN_Y};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Priority(None))
            .add_systems(OnEnter(AppState::InGame), reset_priority)
            .add_systems(
                Update,
                (debug_kill, check_respawn)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (spawn_attacks, update_hitboxes, hitbox_hits, update_attack_state)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                cage_and_crush
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing))
                    .after(camera_follow),
            )
            .add_systems(OnExit(AppState::InGame), despawn_with::<Hitbox>);
    }
}

#[derive(Resource)]
pub struct Priority(pub Option<PlayerSide>);

#[derive(Component)]
pub struct Dead {
    killer_x_at_death: f32,
    timer: Timer,
}

impl Dead {
    fn new(killer_x: f32) -> Self {
        Self {
            killer_x_at_death: killer_x,
            timer: Timer::from_seconds(RESPAWN_MAX_WAIT, TimerMode::Once),
        }
    }
}

const RESPAWN_ADVANCE: f32 = 220.0;
const RESPAWN_INSET: f32 = 60.0;
const RESPAWN_MAX_WAIT: f32 = 4.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttackType {
    Standing,
    Jumping,
    Crouching,
}

#[derive(Component)]
pub struct Hitbox {
    owner: PlayerSide,
    width: f32,
    height: f32,
    life: Timer,
    attack_type: AttackType,
    damage: u8,
    has_hit: bool,
}

const ATTACK_COLOR: Color = Color::srgb(0.9, 0.15, 0.15);

const STAND_DAMAGE: u8 = 1;
const STAND_DURATION: f32 = 0.18;
const STAND_COOLDOWN: f32 = 0.30;
const STAND_W: f32 = 55.0;
const STAND_H: f32 = 20.0;
const STAND_Y_OFFSET: f32 = 15.0;

const JUMP_DAMAGE: u8 = 2;
const JUMP_DURATION: f32 = 0.30;
const JUMP_COOLDOWN: f32 = 0.50;
const JUMP_W: f32 = 60.0;
const JUMP_H: f32 = 50.0;
const JUMP_Y_OFFSET: f32 = -25.0;
const JUMP_MIN_HEIGHT: f32 = 60.0;

const CROUCH_DAMAGE: u8 = 1;
const CROUCH_DURATION: f32 = 0.10;
const CROUCH_COOLDOWN: f32 = 0.18;
const CROUCH_W: f32 = 70.0;
const CROUCH_H: f32 = 16.0;
const CROUCH_Y_OFFSET: f32 = -8.0;

const HITSTUN_DURATION: f32 = 0.25;

fn reset_priority(mut priority: ResMut<Priority>) {
    priority.0 = None;
}

// placeholder for real combat — 1 kills the left fighter, 2 kills the right
fn debug_kill(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut priority: ResMut<Priority>,
    players: Query<(Entity, &Transform, &PlayerSide, Option<&Dead>), With<Player>>,
) {
    let target = if keyboard.just_pressed(KeyCode::Digit1) {
        PlayerSide::Left
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        PlayerSide::Right
    } else {
        return;
    };

    let mut victim = None;
    let mut killer_x = 0.0;
    let mut killer_alive = false;

    for (entity, transform, side, dead) in &players {
        if *side == target {
            if dead.is_some() {
                return; // already down
            }
            victim = Some(entity);
        } else {
            killer_alive = dead.is_none();
            killer_x = transform.translation.x;
        }
    }

    let Some(victim) = victim else {
        return;
    };

    commands
        .entity(victim)
        .insert((Dead::new(killer_x), Visibility::Hidden));

    // a kill hands priority to the survivor; a double-down resets to neutral
    priority.0 = if killer_alive {
        Some(target.other())
    } else {
        None
    };
}

fn check_respawn(
    mut commands: Commands,
    time: Res<Time>,
    priority: Res<Priority>,
    current_screen: Res<CurrentScreen>,
    camera: Single<&Transform, With<UiCamera>>,
    mut dead: Query<
        (Entity, &mut Player, &mut Transform, &PlayerSide, &mut Dead),
        Without<UiCamera>,
    >,
    alive: Query<(&Transform, &PlayerSide), (With<Player>, Without<Dead>)>,
) {
    let half_view = SCREEN_W / 2.0;
    // a new screen is a checkpoint — anyone down comes back on arrival
    let screen_changed = current_screen.is_changed();

    for (entity, mut player, mut transform, side, mut dead_info) in &mut dead {
        dead_info.timer.tick(time.delta());
        let waited_long_enough = dead_info.timer.is_finished();

        let respawn_x = match priority.0 {
            // neutral — both went down, everyone back to their starting marks
            None => side.spawn_x(),
            Some(holder) => {
                // where the fighter holding priority is standing
                let mut holder_x = None;
                for (transform, side) in &alive {
                    if *side == holder {
                        holder_x = Some(transform.translation.x);
                    }
                }
                let Some(holder_x) = holder_x else {
                    continue;
                };

                let dir = holder.advance_dir();
                let advanced = (holder_x - dead_info.killer_x_at_death) * dir;

                // positions reset on a screen change, so the distance check is
                // only meaningful within one screen
                if !screen_changed && !waited_long_enough && advanced < RESPAWN_ADVANCE {
                    continue;
                }
                // appear at the edge the leader is pushing toward
                camera.translation.x + (half_view - RESPAWN_INSET) * dir
            }
        };

        transform.translation.x = respawn_x;
        transform.translation.y = PLAYER_SPAWN_Y;
        player.velocity = Vec2::ZERO;
        player.is_grounded = false;
        player.health = player.max_health;
        player.attack_cooldown = Timer::from_seconds(0.0, TimerMode::Once);
        player.hitstun = Timer::from_seconds(0.0, TimerMode::Once);
        player.current_attack = None;

        commands
            .entity(entity)
            .remove::<Dead>()
            .insert(Visibility::Inherited);
    }
}

// Keeps everyone inside the view. For the trailing fighter the back edge is
// lethal: if the leader outruns them, the camera pushes them off and they die.
fn cage_and_crush(
    mut commands: Commands,
    priority: Res<Priority>,
    camera: Single<&Transform, With<UiCamera>>,
    mut players: Query<
        (Entity, &Player, &mut Transform, &PlayerSide),
        (Without<Dead>, Without<UiCamera>),
    >,
) {
    let half_view = SCREEN_W / 2.0;
    let cam_x = camera.translation.x;

    // where the leader stands right now, recorded on any crush death
    let holder = priority.0;
    let mut holder_x = None;
    if let Some(h) = holder {
        for (_, _, transform, side) in &players {
            if *side == h {
                holder_x = Some(transform.translation.x);
            }
        }
    }

    for (entity, player, mut transform, side) in &mut players {
        let half_w = player.width / 2.0;
        let min = cam_x - half_view + half_w;
        let max = cam_x + half_view - half_w;
        let x = transform.translation.x;

        if let (Some(h), Some(hx)) = (holder, holder_x) {
            if *side != h {
                let dir = h.advance_dir();
                let trailing = if dir > 0.0 { min } else { max };
                // behind the leader's back edge — crushed out of frame
                if (x - trailing) * dir < 0.0 {
                    commands
                        .entity(entity)
                        .insert((Dead::new(hx), Visibility::Hidden));
                    continue;
                }
            }
        }

        transform.translation.x = x.clamp(min, max);
    }
}

fn spawn_attacks(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&mut Player, &Transform, &PlayerSide, &Controls), Without<Dead>>,
    hitboxes: Query<&Hitbox>,
) {
    for (mut player, transform, side, controls) in &mut players {
        if !keyboard.just_pressed(controls.attack) {
            continue;
        }
        if !player.attack_cooldown.is_finished() {
            continue;
        }
        if !player.hitstun.is_finished() {
            continue;
        }
        if hitboxes.iter().any(|h| h.owner == *side) {
            continue;
        }

        let ground_y = PLAYER_SPAWN_Y;
        let height_above_ground = transform.translation.y - ground_y;

        let (attack_type, damage, duration, cooldown, width, height, y_offset) =
            if !player.is_grounded && height_above_ground > JUMP_MIN_HEIGHT {
                (
                    AttackType::Jumping,
                    JUMP_DAMAGE,
                    JUMP_DURATION,
                    JUMP_COOLDOWN,
                    JUMP_W,
                    JUMP_H,
                    JUMP_Y_OFFSET,
                )
            } else if player.is_crouching {
                (
                    AttackType::Crouching,
                    CROUCH_DAMAGE,
                    CROUCH_DURATION,
                    CROUCH_COOLDOWN,
                    CROUCH_W,
                    CROUCH_H,
                    CROUCH_Y_OFFSET,
                )
            } else if player.is_grounded {
                (
                    AttackType::Standing,
                    STAND_DAMAGE,
                    STAND_DURATION,
                    STAND_COOLDOWN,
                    STAND_W,
                    STAND_H,
                    STAND_Y_OFFSET,
                )
            } else {
                continue;
            };

        player.attack_cooldown = Timer::from_seconds(cooldown, TimerMode::Once);
        player.current_attack = Some(attack_type);

        commands.spawn((
            Sprite {
                color: ATTACK_COLOR,
                custom_size: Some(Vec2::new(width, height)),
                ..default()
            },
            Transform::from_xyz(
                transform.translation.x + player.facing * (player.width + width) / 2.0,
                transform.translation.y + y_offset,
                2.0,
            ),
            GlobalTransform::default(),
            Hitbox {
                owner: *side,
                width,
                height,
                life: Timer::from_seconds(duration, TimerMode::Once),
                attack_type,
                damage,
                has_hit: false,
            },
        ));
    }
}

fn update_hitboxes(
    mut commands: Commands,
    time: Res<Time>,
    mut hitboxes: Query<(Entity, &mut Hitbox, &mut Transform), Without<Player>>,
    players: Query<(&Player, &Transform, &PlayerSide), Without<Dead>>,
) {
    for (entity, mut hitbox, mut hb_transform) in &mut hitboxes {
        hitbox.life.tick(time.delta());
        if hitbox.life.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let mut owner = None;
        for (player, p_transform, side) in &players {
            if *side == hitbox.owner {
                owner = Some((player.facing, player.width, p_transform.translation));
            }
        }

        let y_offset = match hitbox.attack_type {
            AttackType::Standing => STAND_Y_OFFSET,
            AttackType::Jumping => JUMP_Y_OFFSET,
            AttackType::Crouching => CROUCH_Y_OFFSET,
        };

        match owner {
            Some((facing, width, owner_pos)) => {
                hb_transform.translation.x =
                    owner_pos.x + facing * (width + hitbox.width) / 2.0;
                hb_transform.translation.y = owner_pos.y + y_offset;
            }
            None => {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn update_attack_state(
    mut players: Query<(&mut Player, &PlayerSide), Without<Dead>>,
    hitboxes: Query<&Hitbox>,
) {
    for (mut player, side) in &mut players {
        if player.current_attack.is_none() {
            continue;
        }
        let has_active_hitbox = hitboxes.iter().any(|h| h.owner == *side);
        if !has_active_hitbox {
            player.current_attack = None;
        }
    }
}

fn hitbox_hits(
    mut commands: Commands,
    mut priority: ResMut<Priority>,
    mut hitboxes: Query<(&mut Hitbox, &Transform)>,
    mut players: Query<(Entity, &mut Player, &Transform, &PlayerSide), Without<Dead>>,
) {
    for (mut hitbox, hb_transform) in &mut hitboxes {
        if hitbox.has_hit {
            continue;
        }

        let hb_half_w = hitbox.width / 2.0;
        let hb_half_h = hitbox.height / 2.0;
        let hb_x = hb_transform.translation.x;
        let hb_y = hb_transform.translation.y;

        let mut hit_entity = None;
        let mut hit_damage = 0;

        for (entity, player, p_transform, side) in &players {
            if *side == hitbox.owner {
                continue;
            }

            let p_h = if player.is_crouching {
                player.crouch_height
            } else {
                player.height
            };
            let p_half_w = player.width / 2.0;
            let p_half_h = p_h / 2.0;
            let px = p_transform.translation.x;
            let py = p_transform.translation.y;

            let overlaps_x = (hb_x - hb_half_w) <= (px + p_half_w)
                && (hb_x + hb_half_w) >= (px - p_half_w);
            let overlaps_y = (hb_y - hb_half_h) <= (py + p_half_h)
                && (hb_y + hb_half_h) >= (py - p_half_h);

            if !overlaps_x || !overlaps_y {
                continue;
            }

            if hitbox.attack_type == AttackType::Standing && player.is_crouching {
                continue;
            }

            hit_entity = Some(entity);
            hit_damage = hitbox.damage;
            break;
        }

        if let Some(entity) = hit_entity {
            hitbox.has_hit = true;

            let mut attacker_x = 0.0;
            for (_, _, transform, side) in &players {
                if *side == hitbox.owner {
                    attacker_x = transform.translation.x;
                }
            }

            if let Ok((_, mut player, _, _)) = players.get_mut(entity) {
                player.health = player.health.saturating_sub(hit_damage);
                player.hitstun = Timer::from_seconds(HITSTUN_DURATION, TimerMode::Once);
                player.velocity.x = 0.0;

                if player.health == 0 {
                    commands
                        .entity(entity)
                        .insert((Dead::new(attacker_x), Visibility::Hidden));
                    priority.0 = Some(hitbox.owner);
                }
            }
        }
    }
}