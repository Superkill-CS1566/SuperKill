use bevy::prelude::*;

use crate::camera::{camera_follow, SCREEN_W};
use crate::common::{AppState, UiCamera};
use crate::game::GameState;
use crate::player::{Player, PlayerSide, PLAYER_SPAWN_Y};

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
            // runs once the camera has settled, so the edges are final
            .add_systems(
                Update,
                cage_and_crush
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing))
                    .after(camera_follow),
            );
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
}

/// How far the leader must advance before the dead fighter returns.
const RESPAWN_ADVANCE: f32 = 220.0;
/// How far inside the screen edge they reappear.
const RESPAWN_INSET: f32 = 60.0;

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

    commands.entity(victim).insert((
        Dead {
            killer_x_at_death: killer_x,
        },
        Visibility::Hidden,
    ));

    // a kill hands priority to the survivor; a double-down resets to neutral
    priority.0 = if killer_alive {
        Some(target.other())
    } else {
        None
    };
}

fn check_respawn(
    mut commands: Commands,
    priority: Res<Priority>,
    camera: Single<&Transform, With<UiCamera>>,
    mut dead: Query<
        (Entity, &mut Player, &mut Transform, &PlayerSide, &Dead),
        Without<UiCamera>,
    >,
    alive: Query<(&Transform, &PlayerSide), (With<Player>, Without<Dead>)>,
) {
    let half_view = SCREEN_W / 2.0;

    for (entity, mut player, mut transform, side, dead_info) in &mut dead {
        let respawn_x = match priority.0 {
            // neutral — both went down, everyone back to their starting marks
            None => side.spawn_x(),
            Some(holder) => {
                let Some((holder_t, _)) = alive.iter().find(|(_, s)| **s == holder) else {
                    continue;
                };
                let dir = holder.advance_dir();
                let advanced = (holder_t.translation.x - dead_info.killer_x_at_death) * dir;
                if advanced < RESPAWN_ADVANCE {
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
    let holder_x = holder.and_then(|h| {
        players
            .iter()
            .find(|(_, _, _, s)| **s == h)
            .map(|(_, _, t, _)| t.translation.x)
    });

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
                    commands.entity(entity).insert((
                        Dead {
                            killer_x_at_death: hx,
                        },
                        Visibility::Hidden,
                    ));
                    continue;
                }
            }
        }

        transform.translation.x = x.clamp(min, max);
    }
}