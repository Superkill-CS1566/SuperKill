use bevy::prelude::*;

use crate::combat::{Dead, Priority, Hitbox};
use crate::common::{AppState, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN, Platform};
use crate::game::GameState;
use crate::loading::{despawn_with, LoadingAssets};
use crate::player::{Controls, NetworkControlled, Player, PlayerSide};

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ShowHitboxes(false))
            .add_systems(Startup, load_weapon_assets)
            .add_systems(
                Update,
                (
                    throw_or_pickup,
                    advance_throw,
                    move_swords,
                    sword_hits,
                    advance_sword_impact,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (toggle_hitbox_debug, draw_sword_hitboxes)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                OnExit(AppState::InGame),
                (despawn_with::<ThrownSword>, despawn_with::<GroundWeapon>),
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
const SWORD_IMPACT_FIRST: usize = 1;
const SWORD_IMPACT_LAST: usize = 4;
const SWORD_FRAME_TIME: f32 = 0.05;

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

// ---------------------------------------------------------------- components

/// On a fighter mid-throw. The sword doesn't leave the hand until
/// THROW_RELEASE_FRAME, so pressing throw and the sword appearing are
/// separate moments.
#[derive(Component)]
pub struct Throwing {
    timer: Timer,
    frame: usize,
    released: bool,
}

#[derive(PartialEq)]
enum SwordState {
    Flying,
    Impacting,
}

/// A sword in flight, or playing its impact frames before it drops.
#[derive(Component)]
pub struct ThrownSword {
    /// Which fighter threw it — they can't be hit by their own sword.
    owner: PlayerSide,
    /// -1.0 or 1.0, the direction of travel.
    direction: f32,
    /// How much further it can travel before dropping.
    remaining: f32,
    state: SwordState,
    frame: usize,
    timer: Timer,
}

/// A weapon lying on the ground, waiting to be picked up. Harmless.
#[derive(Component)]
pub struct GroundWeapon;

// ---------------------------------------------------------------- tuning

/// 6 frames of wind-up and release.
pub const THROW_FRAME_TIME: f32 = 0.06; // pub so throwing anim frame timing lines up
const THROW_FRAMES: usize = 6;
/// Zero-indexed, so 4 is the 5th frame — where the sword leaves the hand.
const THROW_RELEASE_FRAME: usize = 4;

// How big the sword is drawn on screen. Keep the same ratio as the sheet's
// cell (100x100) or the sprite comes out stretched.
const SWORD_SPRITE_W: f32 = 100.0;
const SWORD_SPRITE_H: f32 = 100.0;

// The part that actually kills. Set these to the blade inside the sprite,
// not the whole cell — most of a 100x100 cell is empty air.
const SWORD_HITBOX_W: f32 = 60.0;
const SWORD_HITBOX_H: f32 = 16.0;
// Nudge the hitbox off the sprite's center if the blade isn't centered in
// the cell. Positive x is toward the direction of travel.
const SWORD_HITBOX_OFFSET_X: f32 = 0.0;
const SWORD_HITBOX_OFFSET_Y: f32 = 0.0;

const SWORD_SPEED: f32 = 700.0;
/// How far a throw carries before the sword drops.
const MAX_THROW_DISTANCE: f32 = 1400.0;

const GROUND_WEAPON_W: f32 = 50.0;
const GROUND_WEAPON_H: f32 = 12.0;
/// A fighter this close to a grounded weapon can pick it up.
const PICKUP_RADIUS: f32 = 55.0;

// ---------------------------------------------------------------- systems

// One button does both: throw if armed, pick up if not.
fn throw_or_pickup(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<
        (Entity, &mut Player, &Transform, &Controls),
        (Without<Dead>, Without<NetworkControlled>, Without<Throwing>),
    >,
    ground_weapons: Query<(Entity, &Transform), With<GroundWeapon>>,
) {
    // a weapon can only be claimed once per frame, even if both fighters reach
    // for it — the despawn below doesn't take effect until the next flush
    let mut claimed: Vec<Entity> = Vec::new();

    for (entity, mut player, transform, controls) in &mut players {
        if !keyboard.just_pressed(controls.throw) {
            continue;
        }
        if player.is_crouching {
            continue;   // can't throw when crouched
        }

        if player.has_weapon {
            // start the wind-up; the sword spawns a few frames later
            commands.entity(entity).insert(Throwing {
                timer: Timer::from_seconds(THROW_FRAME_TIME, TimerMode::Repeating),
                frame: 0,
                released: false,
            });
        } else {
            // empty-handed — grab the nearest weapon in reach
            let mut nearest: Option<(Entity, f32)> = None;
            for (weapon_entity, weapon_transform) in &ground_weapons {
                if claimed.contains(&weapon_entity) {
                    continue;
                }
                let distance = transform.translation.distance(weapon_transform.translation);
                if distance > PICKUP_RADIUS {
                    continue;
                }
                match nearest {
                    Some((_, best)) if best <= distance => {}
                    _ => nearest = Some((weapon_entity, distance)),
                }
            }

            if let Some((weapon_entity, _)) = nearest {
                claimed.push(weapon_entity);
                commands.entity(weapon_entity).despawn();
                player.has_weapon = true;
            }
        }
    }
}

// Steps through the throw frames, releasing the sword partway in.
fn advance_throw(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<WeaponAssets>,
    mut throwers: Query<(
        Entity,
        &mut Player,
        &Transform,
        &PlayerSide,
        &mut Throwing,
    )>,
) {
    for (entity, mut player, transform, side, mut throwing) in &mut throwers {
        throwing.timer.tick(time.delta());
        if !throwing.timer.just_finished() {
            continue;
        }
        throwing.frame += 1;

        if throwing.frame >= THROW_RELEASE_FRAME && !throwing.released {
            throwing.released = true;
            player.has_weapon = false;

            commands.spawn((
                Sprite {
                    image: assets.sword.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: assets.sword_layout.clone(),
                        index: SWORD_FLIGHT_FRAME,
                    }),
                    custom_size: Some(Vec2::new(SWORD_SPRITE_W, SWORD_SPRITE_H)),
                    flip_x: player.facing < 0.0,
                    ..default()
                },
                Transform::from_xyz(
                    transform.translation.x
                        + player.facing * (player.width + SWORD_HITBOX_W) / 2.0,
                    transform.translation.y,
                    2.0,
                ),
                GlobalTransform::default(),
                ThrownSword {
                    owner: *side,
                    direction: player.facing,
                    remaining: MAX_THROW_DISTANCE,
                    state: SwordState::Flying,
                    frame: SWORD_FLIGHT_FRAME,
                    timer: Timer::from_seconds(SWORD_FRAME_TIME, TimerMode::Repeating),
                },
            ));
        }

        if throwing.frame >= THROW_FRAMES {
            commands.entity(entity).remove::<Throwing>();
        }
    }
}

// Swords fly straight. Running into a hill stops them with the impact frames;
// running out of distance or reaching the level edge just drops them.
fn move_swords(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<WeaponAssets>,
    mut swords: Query<(Entity, &mut ThrownSword, &mut Sprite, &mut Transform)>,
    platforms: Query<(&Platform, &Transform), Without<ThrownSword>>,
) {
    let delta = time.delta().as_secs_f32();
    let half_w = SWORD_HITBOX_W / 2.0;
    let half_h = SWORD_HITBOX_H / 2.0;

    for (entity, mut sword, mut sprite, mut transform) in &mut swords {
        if sword.state != SwordState::Flying {
            continue;
        }

        let step = SWORD_SPEED * delta;
        transform.translation.x += sword.direction * step;
        sword.remaining -= step;

        // the collider sits inside the sprite, mirrored with the flip
        let sword_x = transform.translation.x + sword.direction * SWORD_HITBOX_OFFSET_X;
        let sword_y = transform.translation.y + SWORD_HITBOX_OFFSET_Y;

        // did it fly into the side of a hill? If it's touching more than one
        // tile, use the face nearest the thrower so it doesn't end up inside
        let mut wall_face: Option<f32> = None;
        for (platform, p_transform) in &platforms {
            let px = p_transform.translation.x;
            let py = p_transform.translation.y;
            let pw = platform.width / 2.0;
            let ph = platform.height / 2.0;

            let overlaps_x = (sword_x - half_w) <= (px + pw) && (sword_x + half_w) >= (px - pw);
            let overlaps_y = (sword_y - half_h) <= (py + ph) && (sword_y + half_h) >= (py - ph);

            if overlaps_x && overlaps_y {
                // the side of this tile facing the incoming sword
                let face = px - sword.direction * pw;
                match wall_face {
                    Some(nearest) if nearest * sword.direction <= face * sword.direction => {}
                    _ => wall_face = Some(face),
                }
            }
        }

        if let Some(face) = wall_face {
            // back it out to the wall so the impact frames play against it
            transform.translation.x =
                face - sword.direction * (half_w + SWORD_HITBOX_OFFSET_X);
            start_impact(&mut sword, &mut sprite);
            continue;
        }

        // keep it inside the level even on a throw from near the edge
        let limit = LEVEL_LEN / 2.0 - half_w;
        let hit_level_edge = transform.translation.x.abs() >= limit;
        if hit_level_edge {
            transform.translation.x = transform.translation.x.clamp(-limit, limit);
        }

        if sword.remaining <= 0.0 || hit_level_edge {
            let surface = surface_y_at(transform.translation.x, transform.translation.y, &platforms);
            drop_weapon(&mut commands, &assets, transform.translation.x, surface);
            commands.entity(entity).despawn();
        }
    }
}

// Contact is an instant kill for now, same as a melee hit.
fn sword_hits(
    mut commands: Commands,
    mut priority: ResMut<Priority>,
    mut swords: Query<(&mut ThrownSword, &mut Sprite, &Transform)>,
    mut players: Query<
        (Entity, &mut Player, &Transform, &PlayerSide, Has<NetworkControlled>),
        Without<Dead>,
    >,
    hitboxes: Query<&Hitbox>,
) {
    for (mut sword, mut sprite, sword_transform) in &mut swords {
        if sword.state != SwordState::Flying {
            continue;
        }

        // the collider sits inside the sprite, mirrored with the flip
        let sword_x = sword_transform.translation.x
            + sword.direction * SWORD_HITBOX_OFFSET_X;
        let sword_y = sword_transform.translation.y + SWORD_HITBOX_OFFSET_Y;

        // attacker entity and their kill threshold, for remote damage reports
        let attacker_info: Option<(Entity, u8)> = players
            .iter()
            .find(|(_, _, _, s, _)| **s == sword.owner)
            .map(|(e, p, _, _, _)| (e, p.max_health));
        for (player_entity, player, player_transform, side, remote) in &players {
            if *side == sword.owner {
                continue;
            }

            let player_h = if player.is_crouching {
                player.crouch_height
            } else {
                player.height
            };
            let mut px = player_transform.translation.x;
            let py = player_transform.translation.y;
            let mut facing_same = false;
            
            for (_, p_other, _, s_other, _) in &players {
                if *s_other != sword.owner {
                    continue;
                }
                if player.facing == p_other.facing {
                    facing_same = true;
                }
            }
            
            for hitbox in &hitboxes {
                if hitbox.is_attack || facing_same {
                    continue;
                }
                px += hitbox.width * player.facing;
            }

            let overlaps_x = (sword_x - SWORD_HITBOX_W / 2.0) <= (px + player.width / 2.0)
                && (sword_x + SWORD_HITBOX_W / 2.0) >= (px - player.width / 2.0);
            let overlaps_y = (sword_y - SWORD_HITBOX_H / 2.0) <= (py + player_h / 2.0)
                && (sword_y + SWORD_HITBOX_H / 2.0) >= (py - player_h / 2.0);

            if overlaps_x && overlaps_y {
                if !player.is_blocking || facing_same {
                    if remote {
                        // remote fighter dies on their own client; report the
                        // damage so the next snapshot applies it
                        if let Some((attacker_entity, kill_health)) = attacker_info {
                            if let Ok((_, mut attacker, _, _, _)) =
                                players.get_mut(attacker_entity)
                            {
                                attacker.damage_out += kill_health;
                            }
                        }
                    } else {
                        commands
                            .entity(player_entity)
                            .insert((Dead::new(sword_x), Visibility::Hidden));
                        priority.0 = Some(sword.owner);
                    }
                }

                // stop here and play the impact frames before dropping
                start_impact(&mut sword, &mut sprite);
                break;
            }
        }
    }
}

// Plays frames 1..=4 where the sword struck, then leaves a weapon behind.
fn advance_sword_impact(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<WeaponAssets>,
    mut swords: Query<(Entity, &mut ThrownSword, &mut Sprite, &Transform)>,
    platforms: Query<(&Platform, &Transform), Without<ThrownSword>>,
) {
    for (entity, mut sword, mut sprite, transform) in &mut swords {
        if sword.state != SwordState::Impacting {
            continue;
        }

        sword.timer.tick(time.delta());
        if !sword.timer.just_finished() {
            continue;
        }

        sword.frame += 1;
        if sword.frame > SWORD_IMPACT_LAST {
            let surface = surface_y_at(transform.translation.x, transform.translation.y, &platforms);
            drop_weapon(&mut commands, &assets, transform.translation.x, surface);
            commands.entity(entity).despawn();
            continue;
        }

        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = sword.frame;
        }
    }
}

// Shared by a hit on a player and a hit on a hill.
fn start_impact(sword: &mut ThrownSword, sprite: &mut Sprite) {
    sword.state = SwordState::Impacting;
    sword.frame = SWORD_IMPACT_FIRST;
    sword.timer.reset();
    if let Some(atlas) = &mut sprite.texture_atlas {
        atlas.index = SWORD_IMPACT_FIRST;
    }
}

/// The top of the ground directly under `x`. Prefers the highest tile that is
/// at or below `from_y`, so a sword lands on the surface beneath it rather
/// than jumping up onto something overhead. If nothing is below — the sword
/// ended up inside a hill — it settles on top of the highest tile at `x`.
fn surface_y_at(
    x: f32,
    from_y: f32,
    platforms: &Query<(&Platform, &Transform), Without<ThrownSword>>,
) -> f32 {
    let mut below: Option<f32> = None;
    let mut highest: Option<f32> = None;

    for (platform, transform) in platforms {
        // only tiles whose span covers this x
        if (x - transform.translation.x).abs() > platform.width / 2.0 {
            continue;
        }
        let top = transform.translation.y + platform.height / 2.0;

        match highest {
            Some(h) if h >= top => {}
            _ => highest = Some(top),
        }
        if top <= from_y {
            match below {
                Some(b) if b >= top => {}
                _ => below = Some(top),
            }
        }
    }

    // fall back to where it already is if there's somehow no ground at all
    below.or(highest).unwrap_or(from_y)
}

fn drop_weapon(commands: &mut Commands, assets: &WeaponAssets, x: f32, surface_y: f32) {
    commands.spawn((
        Sprite {
            image: assets.ground.clone(),
            custom_size: Some(Vec2::new(GROUND_WEAPON_W, GROUND_WEAPON_H)),
            ..default()
        },
        // resting on the surface, not sunk into it
        Transform::from_xyz(x, surface_y + GROUND_WEAPON_H / 2.0, 1.0),
        GlobalTransform::default(),
        GroundWeapon,
    ));
}

// ---------------------------------------------------------------- debug

/// Draw the sword colliders on screen. Toggled with F1, off by default.
#[derive(Resource)]
pub struct ShowHitboxes(pub bool);

const HITBOX_DEBUG_KEY: KeyCode = KeyCode::F1;

fn toggle_hitbox_debug(keyboard: Res<ButtonInput<KeyCode>>, mut show: ResMut<ShowHitboxes>) {
    if keyboard.just_pressed(HITBOX_DEBUG_KEY) {
        show.0 = !show.0;
    }
}

// Mirrors the exact math in sword_hits, so what you see is what kills.
pub fn draw_sword_hitboxes(
    show: Res<ShowHitboxes>,
    mut gizmos: Gizmos,
    swords: Query<(&ThrownSword, &Transform)>,
    ground_weapons: Query<&Transform, With<GroundWeapon>>,
) {
    if !show.0 {
        return;
    }

    for (sword, transform) in &swords {
        let center = Vec2::new(
            transform.translation.x + sword.direction * SWORD_HITBOX_OFFSET_X,
            transform.translation.y + SWORD_HITBOX_OFFSET_Y,
        );
        // red while it can kill, grey once it's just playing impact frames
        let color = match sword.state {
            SwordState::Flying => Color::srgb(1.0, 0.1, 0.1),
            SwordState::Impacting => Color::srgb(0.5, 0.5, 0.5),
        };
        gizmos.rect_2d(
            center,
            Vec2::new(SWORD_HITBOX_W, SWORD_HITBOX_H),
            color,
        );
    }

    // pickup range is a radius, not a box
    for transform in &ground_weapons {
        gizmos.circle_2d(
            transform.translation.truncate(),
            PICKUP_RADIUS,
            Color::srgb(0.2, 0.9, 0.4),
        );
    }
}