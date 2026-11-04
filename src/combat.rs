use bevy::prelude::*;

use crate::common::AppState;
use crate::game::{GameState, Match};
use crate::loading::despawn_with;
use crate::player::PlayerSide;

// The combat rules live in the simulation now (src/sim/combat.rs). This file
// draws attacks and blocks, and holds the debug kill keys.

pub use crate::sim::AttackType;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_combat_boxes)
            .add_systems(
                Update,
                debug_kill
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<Match>),
            )
            .add_systems(
                Update,
                sync_combat_boxes
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>),
            )
            .add_systems(OnExit(AppState::InGame), despawn_with::<CombatBox>);
    }
}

const ATTACK_COLOR: Color = Color::srgb(0.9, 0.15, 0.15);
const ATTACK_COLOR_UNARMED: Color = Color::srgb(0.95, 0.55, 0.15);
const BLOCK_COLOR: Color = Color::srgb(0.2, 0.2, 1.0);

/// A colored rectangle showing one fighter's active attack or block.
#[derive(Component)]
struct CombatBox {
    side: PlayerSide,
    kind: BoxKind,
}

#[derive(Clone, Copy, PartialEq)]
enum BoxKind {
    Attack,
    Block,
}

// One attack box and one block box per fighter, hidden until they're used.
fn spawn_combat_boxes(mut commands: Commands) {
    for side in [PlayerSide::Left, PlayerSide::Right] {
        for kind in [BoxKind::Attack, BoxKind::Block] {
            commands.spawn((
                Sprite {
                    color: if kind == BoxKind::Attack { ATTACK_COLOR } else { BLOCK_COLOR },
                    custom_size: Some(Vec2::ONE),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 2.0),
                Visibility::Hidden,
                CombatBox { side, kind },
            ));
        }
    }
}

// Shows each box while its attack or block is out, sized and placed from the
// simulation. The size depends on whether the fighter swung holding the sword.
fn sync_combat_boxes(
    game: Res<Match>,
    fixed: Res<Time<Fixed>>,
    mut boxes: Query<(&CombatBox, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let alpha = fixed.overstep_fraction();

    for (combat_box, mut sprite, mut transform, mut visibility) in &mut boxes {
        let fighter = game.state.fighter(combat_box.side);
        let rect = if fighter.is_dead() {
            None
        } else {
            match combat_box.kind {
                BoxKind::Attack => fighter.attack_hitbox(),
                BoxKind::Block => fighter.block_box(),
            }
        };
        let Some(rect) = rect else {
            *visibility = Visibility::Hidden;
            continue;
        };

        // the box sits relative to its fighter, so shift it by however far the
        // drawn fighter is from the simulated one this frame
        let drawn = game.drawn_position(combat_box.side, alpha);
        transform.translation.x = rect.x + (drawn.x - fighter.x);
        transform.translation.y = rect.y + (drawn.y - fighter.y);
        sprite.custom_size = Some(Vec2::new(rect.width, rect.height));

        if combat_box.kind == BoxKind::Attack {
            sprite.color = if fighter.attack_armed {
                ATTACK_COLOR
            } else {
                ATTACK_COLOR_UNARMED
            };
        }
        *visibility = Visibility::Inherited;
    }
}

// placeholder for real combat — 1 kills the left fighter, 2 kills the right
fn debug_kill(keyboard: Res<ButtonInput<KeyCode>>, mut game: ResMut<Match>) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        game.state.debug_kill(PlayerSide::Left);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        game.state.debug_kill(PlayerSide::Right);
    }
}