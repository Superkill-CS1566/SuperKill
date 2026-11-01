use bevy::prelude::*;

use crate::camera::{camera_follow, SCREEN_W};
use crate::common::{AppState, UiCamera};
use crate::loading::despawn_with;
use crate::game::{CurrentScreen, GameState};
use crate::player::{move_and_collide, Controls, Player, PlayerSide, PLAYER_SPAWN_Y};

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
                (spawn_attacks, spawn_blocking, update_hitboxes, hitbox_hits, update_attack_state)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            // a drop ends the moment the fighter lands, so this needs the
            // is_grounded that move_and_collide just wrote
            .add_systems(
                Update,
                land_drop_attack
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing))
                    .after(move_and_collide),
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
    pub fn new(killer_x: f32) -> Self {
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
    /// The drop attack — see the JUMP_ / DROP_ constants below.
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
    is_attack: bool,
}

//const ATTACK_H: f32 = 24.0;
const BLOCK_W: f32 = 25.0;
const BLOCK_H: f32 = 52.0;

/// How long a swing/block stays out — also acts as the attack cooldown, since a
//const ATTACK_W_ARMED: f32 = 55.0;
//const ATTACK_W_UNARMED: f32 = 28.0;

//const ATTACK_H: f32 = 12.0;
/// How long a swing stays out — also acts as the attack cooldown, since a
/// fighter can't swing again while their hitbox is still alive.
//const ATTACK_DURATION: f32 = 0.18;
const BLOCK_DURATION: f32 = 1.00;
const ATTACK_COLOR: Color = Color::srgb(0.9, 0.15, 0.15);
const ATTACK_COLOR_UNARMED: Color = Color::srgb(0.95, 0.55, 0.15);
const BLOCK_COLOR: Color = Color::srgb(0.2, 0.2, 1.0);

const STAND_DAMAGE: u8 = 1;
const STAND_DURATION: f32 = 0.18;
const STAND_COOLDOWN: f32 = 0.30;
const STAND_Y_OFFSET: f32 = 15.0;

// The jump attack is a drop: once a fighter has been airborne long enough,
// it pulls them straight down and strikes whatever is underneath them.
const JUMP_DAMAGE: u8 = 2;
const JUMP_COOLDOWN: f32 = 0.50;
const JUMP_Y_OFFSET: f32 = -25.0;
/// How long a fighter must be airborne before a drop is allowed.
const DROP_MIN_AIR_TIME: f32 = 0.22;
/// Safety cap on the drop hitbox. It normally ends on landing, well before this.
const DROP_MAX_DURATION: f32 = 1.5;
/// How long the fighter is stuck in the landing afterward.
const DROP_LANDING_LAG: f32 = 0.35;
/// Fall speed while dropping. Read by apply_gravity in player.rs.
/// Horizontal speed carries over unchanged from the moment the drop starts.
pub const DROP_SPEED: f32 = -1100.0;

const CROUCH_DAMAGE: u8 = 1;
const CROUCH_DURATION: f32 = 0.10;
const CROUCH_COOLDOWN: f32 = 0.18;
const CROUCH_Y_OFFSET: f32 = -8.0;

const HITSTUN_DURATION: f32 = 0.25;

// ---------------------------------------------------------------- hitbox sizes
// Every attack hitbox size lives here. Width is reach in front of the fighter
// (for the drop, the width of the area underneath); height is vertical
// coverage. Vertical *position* is the *_Y_OFFSET in each attack's block above.

// standing swing
const STAND_W: f32 = 55.0;
const STAND_H: f32 = 20.0;
const STAND_W_UNARMED: f32 = 28.0;
const STAND_H_UNARMED: f32 = 20.0;

// crouching swing
const CROUCH_W: f32 = 70.0;
const CROUCH_H: f32 = 16.0;
const CROUCH_W_UNARMED: f32 = 35.0;
const CROUCH_H_UNARMED: f32 = 16.0;

// drop attack
const JUMP_W: f32 = 60.0;
const JUMP_H: f32 = 50.0;
const JUMP_W_UNARMED: f32 = 36.0;
const JUMP_H_UNARMED: f32 = 30.0;

// Armed and unarmed attacks differ only in size.
fn hitbox_size(attack_type: AttackType, armed: bool) -> (f32, f32) {
    match (attack_type, armed) {
        (AttackType::Standing, true) => (STAND_W, STAND_H),
        (AttackType::Standing, false) => (STAND_W_UNARMED, STAND_H_UNARMED),
        (AttackType::Crouching, true) => (CROUCH_W, CROUCH_H),
        (AttackType::Crouching, false) => (CROUCH_W_UNARMED, CROUCH_H_UNARMED),
        (AttackType::Jumping, true) => (JUMP_W, JUMP_H),
        (AttackType::Jumping, false) => (JUMP_W_UNARMED, JUMP_H_UNARMED),
    }
}

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
            None => {
                if !waited_long_enough {
                    continue;
                }

                side.spawn_x()
            }
            Some(holder) => {
                // where the fighter holding priority is standing
                let mut holder_x = None;
                for (transform, side) in &alive {
                    if *side == holder {
                        holder_x = Some(transform.translation.x);
                    }
                }
                match holder_x {
                    // the holder is down too, so there is no leader to wait
                    // for — come back on our own mark rather than deadlocking
                    None => {
                        if !waited_long_enough {
                            continue;
                        }
                        side.spawn_x()
                    },
                    Some(holder_x) => {
                        let dir = holder.advance_dir();
                        let advanced = (holder_x - dead_info.killer_x_at_death) * dir;

                        // positions reset on a screen change, so the distance
                        // check is only meaningful within one screen
                        if !screen_changed
                            && !waited_long_enough
                            && advanced < RESPAWN_ADVANCE
                        {
                            continue;
                        }
                        // appear at the edge the leader is pushing toward
                        camera.translation.x + (half_view - RESPAWN_INSET) * dir
                    }
                }
            }
        };

        transform.translation.x = respawn_x;
        transform.translation.y = PLAYER_SPAWN_Y;
        player.velocity = Vec2::ZERO;
        player.has_weapon = true;   
        player.is_grounded = false;
        player.health = player.max_health;
        player.attack_cooldown = Timer::from_seconds(0.0, TimerMode::Once);
        player.is_blocking = false;
        player.hitstun = Timer::from_seconds(0.0, TimerMode::Once);
        player.current_attack = None;
        // clear any drop that was interrupted by dying
        player.air_time = 0.0;
        player.is_dropping = false;
        player.landing_lag = Timer::from_seconds(0.0, TimerMode::Once);
        

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
        // covers hitstun and the landing lag after a drop
        if player.is_stunned() {
            continue;
        }
        if player.is_blocking {
            continue;
        }
        if hitboxes.iter().any(|h| h.owner == *side) {
            continue;
        }
        /*let attack_w = if player.has_weapon{
            ATTACK_W_ARMED
        } else {
            ATTACK_W_UNARMED
        };*/

        // airtime rather than height above the ground, so it works on hills
        let (attack_type, damage, duration, cooldown, y_offset) =
            if !player.is_grounded && player.air_time >= DROP_MIN_AIR_TIME {
                (
                    AttackType::Jumping,
                    JUMP_DAMAGE,
                    DROP_MAX_DURATION,
                    JUMP_COOLDOWN,
                    JUMP_Y_OFFSET,
                )
            } else if player.is_crouching {
                (
                    AttackType::Crouching,
                    CROUCH_DAMAGE,
                    CROUCH_DURATION,
                    CROUCH_COOLDOWN,
                    CROUCH_Y_OFFSET,
                )
            } else if player.is_grounded {
                (
                    AttackType::Standing,
                    STAND_DAMAGE,
                    STAND_DURATION,
                    STAND_COOLDOWN,
                    STAND_Y_OFFSET,
                )
            } else {
                // airborne, but not long enough for a drop yet
                continue;
            };

        // the size depends on whether they're holding the sword
        let (width, height) = hitbox_size(attack_type, player.has_weapon);

        player.attack_cooldown = Timer::from_seconds(cooldown, TimerMode::Once);
        player.current_attack = Some(attack_type);

        if attack_type == AttackType::Jumping {
            // start the plunge this frame instead of waiting on apply_gravity
            player.is_dropping = true;
            player.velocity.y = DROP_SPEED;
        }

        // a drop strikes whatever is directly underneath; every other
        // attack sits in front of the fighter
        let hitbox_x = if attack_type == AttackType::Jumping {
            transform.translation.x
        } else {
            transform.translation.x + player.facing * (player.width + width) / 2.0
        };

        commands.spawn((
            Sprite {
                color: if player.has_weapon { ATTACK_COLOR } else { ATTACK_COLOR_UNARMED },
                custom_size: Some(Vec2::new(width, height)),
                ..default()
            },
            Transform::from_xyz(
                hitbox_x,
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
                is_attack: true,
            },
        ));
    }
}

fn spawn_blocking(
    mut commands: Commands,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&mut Player, &Transform, &PlayerSide, &Controls), Without<Dead>>,
    hitboxes: Query<&Hitbox>,
) {
    for (mut player, transform, side, controls) in &mut players {
        player.block_cooldown.tick(time.delta());
        if !keyboard.just_pressed(controls.block) {
            continue;
        }
        // one block at a time
        if hitboxes.iter().any(|h| h.owner == *side) {
            continue;
        }
        if !player.block_cooldown.is_finished() {
            continue;
        }
        // covers hitstun and the landing lag after a drop
        if player.current_attack.is_some() || player.is_stunned() {
            continue;
        }

        commands.spawn((
            Sprite {
                color: BLOCK_COLOR,
                custom_size: Some(Vec2::new(BLOCK_W, BLOCK_H)),
                ..default()
            },
            Transform::from_xyz(
                transform.translation.x + player.facing * (player.width + BLOCK_W) / 2.0,
                transform.translation.y,
                2.0,
            ),
            GlobalTransform::default(),
            Hitbox {
                owner: *side,
                width: BLOCK_W,
                height: BLOCK_H,
                life: Timer::from_seconds(BLOCK_DURATION, TimerMode::Once),
                attack_type: AttackType::Standing, // placeholder; ignored for blocks
                damage: 0,
                has_hit: false,
                is_attack: false,
            },
        ));
        player.is_blocking = true;
    }
}

fn update_hitboxes(
    mut commands: Commands,
    time: Res<Time>,
    mut hitboxes: Query<(Entity, &mut Hitbox, &mut Transform), Without<Player>>,
    mut players: Query<(&mut Player, &Transform, &PlayerSide), Without<Dead>>,
) {
    for (entity, mut hitbox, mut hb_transform) in &mut hitboxes {
        hitbox.life.tick(time.delta());
        if hitbox.life.is_finished() {
            for (mut player, _transform, side) in &mut players {
                if *side == hitbox.owner && player.is_blocking {
                    player.is_blocking = false;
                    player.block_cooldown = Timer::from_seconds(2.00, TimerMode::Once);
                }
            }
            commands.entity(entity).despawn();
            continue;
        }

        let mut owner = None;
        for (player, p_transform, side) in &players {
            if *side == hitbox.owner {
                owner = Some((
                    player.facing,
                    player.width,
                    p_transform.translation,
                    player.is_dropping,
                ));
            }
        }

        let y_offset = if hitbox.is_attack {
            match hitbox.attack_type {
                AttackType::Standing => STAND_Y_OFFSET,
                AttackType::Jumping => JUMP_Y_OFFSET,
                AttackType::Crouching => CROUCH_Y_OFFSET,
            }
        } else {
            0.0
        };

        let is_drop = hitbox.is_attack && hitbox.attack_type == AttackType::Jumping;

        match owner {
            // a drop's hitbox lasts exactly as long as the drop. Checking here
            // rather than on landing also catches one spawned on the same frame
            // its owner touched down, which a landing check would miss
            Some((_, _, _, false)) if is_drop => {
                commands.entity(entity).despawn();
            }
            Some((facing, width, owner_pos, _)) => {
                // must match the placement in spawn_attacks
                hb_transform.translation.x = if is_drop {
                    owner_pos.x
                } else {
                    owner_pos.x + facing * (width + hitbox.width) / 2.0
                };
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

// A drop ends the moment the fighter touches down, leaving them stuck in the
// landing for a beat. Its hitbox is cleared by update_hitboxes once it sees
// is_dropping go false.
fn land_drop_attack(mut players: Query<&mut Player, Without<Dead>>) {
    for mut player in &mut players {
        if !player.is_dropping || !player.is_grounded {
            continue;
        }
        player.is_dropping = false;
        player.landing_lag = Timer::from_seconds(DROP_LANDING_LAG, TimerMode::Once);
    }
}

fn hitbox_hits(
    mut commands: Commands,
    mut priority: ResMut<Priority>,
    mut hitboxes: Query<(&mut Hitbox, &Transform)>,
    mut players: Query<(Entity, &mut Player, &Transform, &PlayerSide), Without<Dead>>,
) {
    let killed: Vec<PlayerSide> = Vec::new();
    let last_killer: Option<PlayerSide> = None;

    // Every active block, so an attack that runs into one stops there.
    // Collected first because the loop below borrows the hitboxes mutably.
    let mut blocks: Vec<(PlayerSide, Vec2, Vec2)> = Vec::new();
    for (hitbox, transform) in &hitboxes {
        if !hitbox.is_attack {
            blocks.push((
                hitbox.owner,
                transform.translation.truncate(),
                Vec2::new(hitbox.width / 2.0, hitbox.height / 2.0),
            ));
        }
    }

    for (mut hitbox, hb_transform) in &mut hitboxes {
        if !hitbox.is_attack || hitbox.has_hit {
            continue;
        }
        // where the attacker stands, for the respawn-distance check
        let mut attacker_x = None;
        for (_, _, transform, side) in &mut players {
            if *side == hitbox.owner {
                attacker_x = Some(transform.translation.x);
            }
        }
        let Some(attacker_x) = attacker_x else {
            continue;
        };

        let hb_half_w = hitbox.width / 2.0;
        let hb_half_h = hitbox.height / 2.0;
        let hb_x = hb_transform.translation.x;
        let hb_y = hb_transform.translation.y;

        let mut hit_entity = None;
        let mut hit_damage = 0;
        let mut blocked = false;

        for (entity, player, p_transform, side) in &players {
            if *side == hitbox.owner {
                continue;
            }

            // A block only guards the side the fighter is facing. Without this,
            // a swing from behind could reach through the body, touch the
            // block on the far side, and be stopped by it.
            let from_front = (attacker_x - p_transform.translation.x) * player.facing > 0.0;

            // ran into this fighter's block — the attack is spent on it
            let mut hits_block = false;
            if from_front {
                for (block_owner, block_center, block_half) in &blocks {
                    if *block_owner != *side {
                        continue;
                    }
                    if (hb_x - hb_half_w) <= block_center.x + block_half.x
                        && (hb_x + hb_half_w) >= block_center.x - block_half.x
                        && (hb_y - hb_half_h) <= block_center.y + block_half.y
                        && (hb_y + hb_half_h) >= block_center.y - block_half.y
                    {
                        hits_block = true;
                    }
                }
            }
            if hits_block {
                blocked = true;
                break;
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

        if blocked {
            // connected with a block, so it can't hit anything else this swing
            hitbox.has_hit = true;
            continue;
        }

        if let Some(entity) = hit_entity {
            hitbox.has_hit = true;

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
    // a trade leaves nobody in charge; the camera relocks to centre
    if killed.len() >= 2 {
        priority.0 = None;
    } else if let Some(killer) = last_killer {
        priority.0 = Some(killer);
    }
}