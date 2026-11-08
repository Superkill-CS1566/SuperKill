use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::animation::{PlayerAnim, PlayerSheets, PLAYER_SPRITE_H, PLAYER_SPRITE_W};
use crate::common::AppState;
use crate::game::{GameState, Match};
use crate::loading::despawn_with;
use crate::sim::fighter::{CROUCH_HEIGHT, PLAYER_HEIGHT, PLAYER_SPAWN_Y};
use crate::sim::Input;
use crate::weapon::ShowHitboxes;

// The fighters' rules live in the simulation now (src/sim/fighter.rs). This
// file turns the keyboard into simulation input, and draws each fighter where
// the simulation says they are.

pub use crate::sim::PlayerSide;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingInputs>()
            .add_systems(OnEnter(AppState::InGame), (spawn_players, reset_inputs))
            // read the keys as early in the frame as possible, so the
            // simulation tick later this same frame already sees them
            .add_systems(
                PreUpdate,
                collect_input
                    .after(InputSystems)
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (sync_fighter_views, align_player_sprite, draw_player_hitboxes)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>),
            )
            .add_systems(OnExit(AppState::InGame), despawn_with::<FighterView>);
    }
}

/// The sprite that draws one fighter.
#[derive(Component)]
pub struct FighterView(pub PlayerSide);

// 每个玩家自己的按键绑定
#[derive(Component)]
pub struct Controls {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub crouch: KeyCode,
    pub attack: KeyCode,
    pub throw: KeyCode,
    pub block: KeyCode,
}

fn controls_for(side: PlayerSide) -> Controls {
    match side {
        PlayerSide::Left => Controls {
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            jump: KeyCode::KeyW,
            crouch: KeyCode::KeyS,
            attack: KeyCode::KeyG,
            throw: KeyCode::KeyT,
            block: KeyCode::KeyB,
        },
        PlayerSide::Right => Controls {
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            jump: KeyCode::ArrowUp,
            crouch: KeyCode::ArrowDown,
            attack: KeyCode::KeyH,
            throw: KeyCode::KeyY,
            block: KeyCode::KeyN,
        },
    }
}

// ---------------------------------------------------------------- input

/// Each fighter's input for the next simulation tick.
///
/// Held keys are read fresh every frame. Presses are kept until a tick uses
/// them, so a tap still counts even when a frame passes with no tick in it.
#[derive(Resource, Default)]
pub struct PendingInputs(pub [Input; 2]);

impl PendingInputs {
    /// Hand the inputs to a tick, clearing the presses it has now used up.
    pub fn take(&mut self) -> [Input; 2] {
        let inputs = self.0;
        for input in &mut self.0 {
            input.jump = false;
            input.attack = false;
            input.block = false;
            input.throw = false;
        }
        inputs
    }
}

fn reset_inputs(mut pending: ResMut<PendingInputs>) {
    *pending = PendingInputs::default();
}

fn collect_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut pending: ResMut<PendingInputs>,
    players: Query<(&FighterView, &Controls)>,
) {
    for (view, controls) in &players {
        let input = &mut pending.0[view.0.index()];

        // held — whatever the keys are doing right now
        input.left = keyboard.pressed(controls.left);
        input.right = keyboard.pressed(controls.right);
        input.crouch = keyboard.pressed(controls.crouch);

        // presses — kept until a tick takes them
        input.jump |= keyboard.just_pressed(controls.jump);
        input.attack |= keyboard.just_pressed(controls.attack);
        input.block |= keyboard.just_pressed(controls.block);
        input.throw |= keyboard.just_pressed(controls.throw);
    }
}

// ---------------------------------------------------------------- drawing

fn spawn_players(mut commands: Commands, sheets: Res<PlayerSheets>) {
    for side in [PlayerSide::Left, PlayerSide::Right] {
        let (image, atlas) = PlayerAnim::starting_sheet(&sheets);

        commands.spawn((
            Sprite {
                image,
                texture_atlas: Some(atlas),
                custom_size: Some(Vec2::new(PLAYER_SPRITE_W, PLAYER_SPRITE_H)),
                ..default()
            },
            Anchor::CENTER,
            // sync_fighter_views moves it to wherever the simulation has them
            Transform::from_xyz(side.spawn_x(), PLAYER_SPAWN_Y, 1.0),
            FighterView(side),
            controls_for(side),
            PlayerAnim::new(),
        ));
    }
}

// Puts each sprite where the simulation has its fighter, smoothed between
// ticks, and hides fighters who are down.
fn sync_fighter_views(
    game: Res<Match>,
    fixed: Res<Time<Fixed>>,
    mut views: Query<(&FighterView, &mut Transform, &mut Visibility)>,
) {
    let alpha = fixed.overstep_fraction();

    for (view, mut transform, mut visibility) in &mut views {
        let drawn = game.drawn_position(view.0, alpha);
        transform.translation.x = drawn.x;
        transform.translation.y = drawn.y;

        *visibility = if game.state.fighter(view.0).is_dead() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

// tune this to place sprite feet at platform
const SPRITE_ANCHOR_Y: f32 = 0.0;

// keeps sprite animation in line when crouching
fn align_player_sprite(game: Res<Match>, mut players: Query<(&FighterView, &mut Anchor)>) {
    for (view, mut anchor) in &mut players {
        let crouch_shift = if game.state.fighter(view.0).crouching {
            (PLAYER_HEIGHT - CROUCH_HEIGHT) / 2.0
        } else {
            0.0
        };
        *anchor = Anchor(Vec2::new(
            0.0,
            SPRITE_ANCHOR_Y - crouch_shift / PLAYER_SPRITE_H,
        ));
    }
}

// Draws the collision box the simulation uses. Toggled with F1 — see
// ShowHitboxes in weapon.rs. Drawn at the exact simulated position rather
// than the smoothed one, so what you see is what actually collides.
fn draw_player_hitboxes(show: Res<ShowHitboxes>, game: Res<Match>, mut gizmos: Gizmos) {
    if !show.0 {
        return;
    }

    for fighter in &game.state.fighters {
        if fighter.is_dead() {
            continue;
        }
        let body = fighter.body();
        gizmos.rect_2d(
            Vec2::new(body.x, body.y),
            Vec2::new(body.width, body.height),
            Color::srgb(0.2, 0.8, 1.0),
        );
    }
}