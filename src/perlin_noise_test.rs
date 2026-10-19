use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};
use crate::perlin_noise::sample_chunk;
use crate::simplerng::SimpleRng;

pub struct PerlinNoiseTestPlugin;

impl Plugin for PerlinNoiseTestPlugin {
    fn build(&self, app: &mut App){
        app.insert_resource(PNoiseSeed(1))
            .add_systems(
                OnEnter(AppState::PNoiseTesting),
                (spawn_ui_camera, setup_pnoise_test),
            )
            .add_systems(
                Update,
                terrain_test_controls.run_if(in_state(AppState::PNoiseTesting)),
            )
            .add_systems(
                OnExit(AppState::PNoiseTesting),
                (despawn_terrain, despawn_ui_camera),
            );
    }
}

#[derive(Resource)]
struct PNoiseSeed(u64);

#[derive(Component)]
struct PNoiseRoot;

fn setup_pnoise_test(mut commands: Commands, mut seed: ResMut<PNoiseSeed>) {
    seed.0 = 0;
    spawn_terrain(&mut commands, seed.0, 1);
}

fn terrain_test_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut seed: ResMut<PNoiseSeed>,
    roots: Query<Entity, With<PNoiseRoot>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
        return;
    }

    if keys.just_pressed(KeyCode::KeyR) {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        seed.0 += 1;
        spawn_terrain(&mut commands, seed.0, 1);
    }
}

// display terrain
// NOTE FOR TEAM: figure out best way to get this to auto-scroll and scale
fn spawn_terrain(commands: &mut Commands, master_seed: u64, level: u32) {
    const WIDTH: u32 = 200;
    const X_STEP: f32 = 5.0;
    const HEIGHT_SCALE: f32 = 100.0;
    const GROUND_Y: f32 = -200.0;

    let level_seed = SimpleRng::derive_seed(master_seed, level);

    let chunk = sample_chunk(0, WIDTH as u32, level_seed);

    let mut inc = 0;
    for i in &chunk {
        let height = i * HEIGHT_SCALE;

        let world_x =
            inc as f32 * X_STEP - (WIDTH as f32 * X_STEP / 2.0);

        let terrain_height = height - GROUND_Y;

        commands.spawn((
            Sprite::from_color(
                Color::srgb(0.25, 0.8, 0.45),
                Vec2::new(X_STEP, terrain_height),
            ),
            Transform::from_xyz(
                world_x,
                GROUND_Y + terrain_height / 2.0,
                0.0,
            ),
            PNoiseRoot,
        ));

        commands.spawn((
        Text::new(format!(
            "Perlin Noise Terrain   Seed: {master_seed}   R: Regenerate   Esc: Menu"
        )),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(18.0),
            ..default()
        },
        PNoiseRoot,
    ));

    inc = inc + 1;
    }
}

fn despawn_terrain(mut commands: Commands, query: Query<Entity, With<PNoiseRoot>>) {
    for entity in &query{
        commands.entity(entity).despawn();
    }
}

