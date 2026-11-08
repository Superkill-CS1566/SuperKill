use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::common::AppState;
use crate::game::{GameState, Match};
use crate::loading::despawn_with;
use crate::player::FighterView;
#[cfg(debug_assertions)]
use crate::player::PlayerSide;
use crate::sim::fighter::MAX_HEALTH;

pub struct HealthVisualsPlugin;

impl Plugin for HealthVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (spawn_health_bars, update_health_bars, tint_low_health)
                .chain()
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameState::Playing))
                .run_if(resource_exists::<Match>)
                .before(TransformSystems::Propagate),
        )
        .add_systems(OnExit(AppState::InGame), despawn_with::<HealthBar>);

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            preview_low_health
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(GameState::Playing))
                .run_if(resource_exists::<Match>),
        );
    }
}

const BAR_WIDTH: f32 = 54.0;
const BAR_HEIGHT: f32 = 5.0;
const BAR_OFFSET: f32 = 16.0;
const HEALTHY_COLOR: Color = Color::srgb(0.2, 0.85, 0.5);
const LOW_COLOR: Color = Color::srgb(1.0, 0.22, 0.18);

#[derive(Component)]
struct HealthBar {
    owner: Entity,
    fill: bool,
}

fn health_ratio(health: u8, max_health: u8) -> f32 {
    if max_health == 0 {
        return 0.0;
    }
    f32::from(health.min(max_health)) / f32::from(max_health)
}

fn is_low_health(health: u8, max_health: u8) -> bool {
    health > 0 && health < max_health && u16::from(health) * 2 <= u16::from(max_health)
}

fn warning_color(seconds: f32) -> Color {
    let pulse = (seconds * std::f32::consts::TAU).sin() * 0.5 + 0.5;
    Color::srgb(1.0, 0.55 + 0.2 * pulse, 0.55 + 0.2 * pulse)
}

fn spawn_health_bars(mut commands: Commands, players: Query<Entity, Added<FighterView>>) {
    for owner in &players {
        for fill in [false, true] {
            commands.spawn((
                Sprite::from_color(
                    if fill {
                        HEALTHY_COLOR
                    } else {
                        Color::srgb(0.08, 0.08, 0.1)
                    },
                    if fill {
                        Vec2::new(BAR_WIDTH, BAR_HEIGHT)
                    } else {
                        Vec2::new(BAR_WIDTH + 4.0, BAR_HEIGHT + 4.0)
                    },
                ),
                Transform::default(),
                HealthBar { owner, fill },
            ));
        }
    }
}

fn update_health_bars(
    mut commands: Commands,
    game: Res<Match>,
    owners: Query<(&FighterView, &Transform)>,
    mut bars: Query<
        (
            Entity,
            &HealthBar,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        ),
        Without<FighterView>,
    >,
) {
    for (entity, bar, mut sprite, mut transform, mut visibility) in &mut bars {
        let Ok((view, owner_transform)) = owners.get(bar.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        let fighter = game.state.fighter(view.0);

        *visibility = if fighter.is_dead() || fighter.health == 0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        let mut position = owner_transform.translation;
        position.y += fighter.height() / 2.0 + BAR_OFFSET;
        position.z += if bar.fill { 3.0 } else { 2.0 };
        if bar.fill {
            let width = BAR_WIDTH * health_ratio(fighter.health, MAX_HEALTH);
            sprite.custom_size = Some(Vec2::new(width, BAR_HEIGHT));
            sprite.color = if is_low_health(fighter.health, MAX_HEALTH) {
                LOW_COLOR
            } else {
                HEALTHY_COLOR
            };
            position.x -= (BAR_WIDTH - width) / 2.0;
        }
        transform.translation = position;
    }
}

fn tint_low_health(
    time: Res<Time>,
    game: Res<Match>,
    mut players: Query<(&FighterView, &mut Sprite)>,
) {
    for (view, mut sprite) in &mut players {
        let fighter = game.state.fighter(view.0);
        sprite.color = if !fighter.is_dead() && is_low_health(fighter.health, MAX_HEALTH) {
            warning_color(time.elapsed_secs())
        } else {
            Color::WHITE
        };
    }
}

// Debug: 3 and 4 flip each fighter between full and low health.
#[cfg(debug_assertions)]
fn preview_low_health(keyboard: Res<ButtonInput<KeyCode>>, mut game: ResMut<Match>) {
    for side in [PlayerSide::Left, PlayerSide::Right] {
        let key = match side {
            PlayerSide::Left => KeyCode::Digit3,
            PlayerSide::Right => KeyCode::Digit4,
        };
        if !keyboard.just_pressed(key) {
            continue;
        }

        let fighter = &mut game.state.fighters[side.index()];
        if fighter.is_dead() {
            continue;
        }
        fighter.health = if is_low_health(fighter.health, MAX_HEALTH) {
            MAX_HEALTH
        } else {
            MAX_HEALTH / 2
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Death, FighterState, PlayerSide, Terrain};

    fn left_fighter(app: &mut App) -> &mut FighterState {
        &mut app.world_mut().resource_mut::<Match>().into_inner().state.fighters[0]
    }

    #[test]
    fn low_health_threshold_and_ratio_handle_edge_cases() {
        assert!(is_low_health(1, 2));
        assert!(is_low_health(2, 5));
        assert!(!is_low_health(2, 2));
        assert!(!is_low_health(0, 2));
        assert!(!is_low_health(1, 1));
        assert!(!is_low_health(0, 0));
        assert!(!is_low_health(200, 255));
        assert_eq!(health_ratio(1, 2), 0.5);
        assert_eq!(health_ratio(4, 2), 1.0);
        assert_eq!(health_ratio(1, 0), 0.0);
    }

    #[test]
    fn bars_and_tint_update_after_damage_death_and_respawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Match::new(Terrain::flat()))
            .add_systems(
                PostUpdate,
                (spawn_health_bars, update_health_bars, tint_low_health).chain(),
            );
        let owner = app
            .world_mut()
            .spawn((
                FighterView(PlayerSide::Left),
                Sprite::default(),
                Transform::default(),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HealthBar>()
                .iter(app.world())
                .count(),
            2
        );

        left_fighter(&mut app).health = 1;
        app.update();
        assert_ne!(
            app.world().get::<Sprite>(owner).unwrap().color,
            Color::WHITE
        );
        let world = app.world_mut();
        for (bar, sprite, transform) in world
            .query::<(&HealthBar, &Sprite, &Transform)>()
            .iter(world)
        {
            if bar.fill {
                assert_eq!(sprite.custom_size.unwrap().x, BAR_WIDTH / 2.0);
                assert_eq!(sprite.color, LOW_COLOR);
                assert_eq!(transform.translation.x, -BAR_WIDTH / 4.0);
            }
        }

        left_fighter(&mut app).death = Some(Death {
            killer_x: 0.0,
            ticks_down: 0,
        });
        app.update();
        let world = app.world_mut();
        for visibility in world
            .query_filtered::<&Visibility, With<HealthBar>>()
            .iter(world)
        {
            assert_eq!(*visibility, Visibility::Hidden);
        }

        let fighter = left_fighter(&mut app);
        fighter.death = None;
        fighter.health = MAX_HEALTH;
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(owner).unwrap().color,
            Color::WHITE
        );
        let world = app.world_mut();
        for (bar, sprite, visibility) in world
            .query::<(&HealthBar, &Sprite, &Visibility)>()
            .iter(world)
        {
            assert_eq!(*visibility, Visibility::Inherited);
            if bar.fill {
                assert_eq!(sprite.custom_size.unwrap().x, BAR_WIDTH);
                assert_eq!(sprite.color, HEALTHY_COLOR);
            }
        }
        app.world_mut().entity_mut(owner).despawn();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HealthBar>()
                .iter(app.world())
                .count(),
            0
        );
    }
}