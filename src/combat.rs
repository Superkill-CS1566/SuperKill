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
                (spawn_attacks, update_hitboxes, hitbox_hits)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            // runs once the camera has settled, so the edges are final
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

/// Which fighter currently owns the camera and may advance screens.
/// None = neutral: camera locked, nobody crosses.
#[derive(Resource)]
pub struct Priority(pub Option<PlayerSide>);

/// On a dead fighter. Records where the killer stood at the moment of the
/// kill, so we know how far they've pushed before the body comes back.
#[derive(Component)]
pub struct Dead {
    killer_x_at_death: f32,
    /// Backstop: brings them back even if the leader never advances.
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

/// How far the leader must advance before the dead fighter returns.
const RESPAWN_ADVANCE: f32 = 220.0;
/// How far inside the screen edge they reappear.
const RESPAWN_INSET: f32 = 60.0;
/// Longest a fighter can stay down, regardless of the leader's progress.
const RESPAWN_MAX_WAIT: f32 = 4.0;

/// A swing. Lives briefly in front of its owner and kills on contact.
#[derive(Component)]
pub struct Hitbox {
    owner: PlayerSide,
    width: f32,
    height: f32,
    life: Timer,
}

const ATTACK_W: f32 = 55.0;
const ATTACK_H: f32 = 24.0;
/// How long a swing stays out — also acts as the attack cooldown, since a
/// fighter can't swing again while their hitbox is still alive.
const ATTACK_DURATION: f32 = 0.18;
const ATTACK_COLOR: Color = Color::srgb(0.9, 0.15, 0.15);

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
    players: Query<(&Player, &Transform, &PlayerSide, &Controls), Without<Dead>>,
    hitboxes: Query<&Hitbox>,
) {
    for (player, transform, side, controls) in &players {
        if !keyboard.just_pressed(controls.attack) {
            continue;
        }
        // one swing at a time
        if hitboxes.iter().any(|h| h.owner == *side) {
            continue;
        }

        commands.spawn((
            Sprite {
                color: ATTACK_COLOR,
                custom_size: Some(Vec2::new(ATTACK_W, ATTACK_H)),
                ..default()
            },
            Transform::from_xyz(
                transform.translation.x + player.facing * (player.width + ATTACK_W) / 2.0,
                transform.translation.y,
                2.0,
            ),
            GlobalTransform::default(),
            Hitbox {
                owner: *side,
                width: ATTACK_W,
                height: ATTACK_H,
                life: Timer::from_seconds(ATTACK_DURATION, TimerMode::Once),
            },
        ));
    }
}

// the swing travels with its owner, and expires on its own
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

        // stay in front of whoever swung it
        let mut owner = None;
        for (player, p_transform, side) in &players {
            if *side == hitbox.owner {
                owner = Some((player.facing, player.width, p_transform.translation));
            }
        }

        match owner {
            Some((facing, width, owner_pos)) => {
                hb_transform.translation.x =
                    owner_pos.x + facing * (width + hitbox.width) / 2.0;
                hb_transform.translation.y = owner_pos.y;
            }
            // owner died mid-swing
            None => {
                commands.entity(entity).despawn();
            }
        }
    }
}

// contact is an instant kill for now — no health, no blocking
fn hitbox_hits(
    mut commands: Commands,
    mut priority: ResMut<Priority>,
    hitboxes: Query<(&Hitbox, &Transform)>,
    players: Query<(Entity, &Player, &Transform, &PlayerSide), Without<Dead>>,
) {
    for (hitbox, hb_transform) in &hitboxes {
        // where the attacker stands, for the respawn-distance check
        let mut attacker_x = None;
        for (_, _, transform, side) in &players {
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

            if overlaps_x && overlaps_y {
                commands
                    .entity(entity)
                    .insert((Dead::new(attacker_x), Visibility::Hidden));
                priority.0 = Some(hitbox.owner);
            }
        }
    }
}