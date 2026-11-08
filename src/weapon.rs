use bevy::prelude::*;

use crate::common::AppState;
use crate::game::Match;
use crate::loading::{despawn_with, LoadingAssets};
use crate::sim::weapons::{GROUND_WEAPON_H, GROUND_WEAPON_W, PICKUP_RADIUS};

// The sword rules live in the simulation now (src/sim/weapons.rs). This file
// draws the swords, and holds the F1 hitbox overlay.

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ShowHitboxes(false))
            .add_systems(Startup, load_weapon_assets)
            .add_systems(
                Update,
                (sync_swords, sync_ground_weapons)
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>),
            )
            .add_systems(
                Update,
                (toggle_hitbox_debug, draw_sword_hitboxes)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Match>),
            )
            .add_systems(
                OnExit(AppState::InGame),
                (despawn_with::<SwordView>, despawn_with::<GroundWeaponView>),
            );
    }
}

// ---------------------------------------------------------------- assets

#[derive(Resource)]
pub struct WeaponAssets {
    sword: Handle<Image>,
    sword_layout: Handle<TextureAtlasLayout>,
    ground: Handle<Image>,
}

/// Cell size of the thrown-sword sheet. Must match the PNG exactly or every
/// frame after the first drifts sideways.
const SWORD_CELL_W: u32 = 100;
const SWORD_CELL_H: u32 = 100;
/// 5 cells in one row: index 0 is sustained flight, 1..=4 play on impact.
const SWORD_SHEET_COLS: u32 = 5;

const SWORD_FLIGHT_FRAME: usize = 0;

// How big the sword is drawn on screen. Keep the same ratio as the sheet's
// cell (100x100) or the sprite comes out stretched.
const SWORD_SPRITE_W: f32 = 100.0;
const SWORD_SPRITE_H: f32 = 100.0;

fn load_weapon_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut loading_assets: ResMut<LoadingAssets>,
) {
    let sword: Handle<Image> = asset_server.load("sprites/player/sword_thrown.png");
    loading_assets.0.push(sword.clone().untyped());

    let ground: Handle<Image> = asset_server.load("sprites/player/sword_ground.png");
    loading_assets.0.push(ground.clone().untyped());

    let sword_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(SWORD_CELL_W, SWORD_CELL_H),
        SWORD_SHEET_COLS,
        1,
        None,
        None,
    ));

    commands.insert_resource(WeaponAssets {
        sword,
        sword_layout,
        ground,
    });
}

// ---------------------------------------------------------------- drawing

/// Draws one thrown sword, matched to the simulation's sword by its id.
#[derive(Component)]
pub struct SwordView {
    id: u32,
}

/// Draws one weapon lying on the ground, matched by id.
#[derive(Component)]
pub struct GroundWeaponView {
    id: u32,
}

// Keeps one sprite per sword in the simulation: moves the ones already drawn,
// removes any that are gone, and adds any thrown since last frame.
fn sync_swords(
    mut commands: Commands,
    game: Res<Match>,
    fixed: Res<Time<Fixed>>,
    assets: Res<WeaponAssets>,
    mut views: Query<(Entity, &SwordView, &mut Sprite, &mut Transform)>,
) {
    let alpha = fixed.overstep_fraction();
    let mut drawn = Vec::new();

    for (entity, view, mut sprite, mut transform) in &mut views {
        let Some(sword) = game.state.swords.iter().find(|s| s.id == view.id) else {
            // dropped to the ground, or cleared by a screen change
            commands.entity(entity).despawn();
            continue;
        };
        drawn.push(view.id);

        let position = game.drawn_sword_position(sword, alpha);
        transform.translation.x = position.x;
        transform.translation.y = position.y;

        // frame 0 in flight, then 1 through 4 where it strikes
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = sword.impact_frame().unwrap_or(SWORD_FLIGHT_FRAME);
        }
    }

    for sword in &game.state.swords {
        if drawn.contains(&sword.id) {
            continue;
        }
        commands.spawn((
            Sprite {
                image: assets.sword.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: assets.sword_layout.clone(),
                    index: sword.impact_frame().unwrap_or(SWORD_FLIGHT_FRAME),
                }),
                custom_size: Some(Vec2::new(SWORD_SPRITE_W, SWORD_SPRITE_H)),
                flip_x: sword.direction < 0.0,
                ..default()
            },
            Transform::from_xyz(sword.x, sword.y, 2.0),
            SwordView { id: sword.id },
        ));
    }
}

// The same for weapons on the ground, which never move once dropped.
fn sync_ground_weapons(
    mut commands: Commands,
    game: Res<Match>,
    assets: Res<WeaponAssets>,
    views: Query<(Entity, &GroundWeaponView)>,
) {
    let mut drawn = Vec::new();

    for (entity, view) in &views {
        if game.state.ground_weapons.iter().any(|w| w.id == view.id) {
            drawn.push(view.id);
        } else {
            // picked up, or cleared by a screen change
            commands.entity(entity).despawn();
        }
    }

    for weapon in &game.state.ground_weapons {
        if drawn.contains(&weapon.id) {
            continue;
        }
        commands.spawn((
            Sprite {
                image: assets.ground.clone(),
                custom_size: Some(Vec2::new(GROUND_WEAPON_W, GROUND_WEAPON_H)),
                ..default()
            },
            // the simulation already rests it on the surface
            Transform::from_xyz(weapon.x, weapon.y, 1.0),
            GroundWeaponView { id: weapon.id },
        ));
    }
}

// ---------------------------------------------------------------- debug

/// Draw the colliders on screen. Toggled with F1, off by default.
#[derive(Resource)]
pub struct ShowHitboxes(pub bool);

const HITBOX_DEBUG_KEY: KeyCode = KeyCode::F1;

fn toggle_hitbox_debug(keyboard: Res<ButtonInput<KeyCode>>, mut show: ResMut<ShowHitboxes>) {
    if keyboard.just_pressed(HITBOX_DEBUG_KEY) {
        show.0 = !show.0;
    }
}

// Drawn straight from the simulation's own hitboxes, so what you see is what kills.
pub fn draw_sword_hitboxes(show: Res<ShowHitboxes>, game: Res<Match>, mut gizmos: Gizmos) {
    if !show.0 {
        return;
    }

    for sword in &game.state.swords {
        let hitbox = sword.hitbox();
        // red while it can kill, grey once it's just playing impact frames
        let color = if sword.is_flying() {
            Color::srgb(1.0, 0.1, 0.1)
        } else {
            Color::srgb(0.5, 0.5, 0.5)
        };
        gizmos.rect_2d(
            Vec2::new(hitbox.x, hitbox.y),
            Vec2::new(hitbox.width, hitbox.height),
            color,
        );
    }

    // pickup range is a radius, not a box
    for weapon in &game.state.ground_weapons {
        gizmos.circle_2d(
            Vec2::new(weapon.x, weapon.y),
            PICKUP_RADIUS,
            Color::srgb(0.2, 0.9, 0.4),
        );
    }
}