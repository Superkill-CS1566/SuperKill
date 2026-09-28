use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};
use crate::procedural_generation::{generate_maze, Tile};

const MAZE_WIDTH: usize = 31;
const MAZE_HEIGHT: usize = 17;
const TILE_SIZE: f32 = 32.0;

pub struct ProceduralTestPlugin;

impl Plugin for ProceduralTestPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MazeSeed(1))
            .add_systems(
                OnEnter(AppState::MazeTesting),
                (spawn_ui_camera, setup_maze_test),
            )
            .add_systems(
                Update,
                maze_test_controls.run_if(in_state(AppState::MazeTesting)),
            )
            .add_systems(
                OnExit(AppState::MazeTesting),
                (despawn_maze_test, despawn_ui_camera),
            );
    }
}

#[derive(Resource)]
struct MazeSeed(u64);

#[derive(Component)]
struct ProceduralTestRoot;

fn setup_maze_test(mut commands: Commands, mut seed: ResMut<MazeSeed>) {
    seed.0 = 1;
    spawn_maze(&mut commands, seed.0);
}

fn maze_test_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut seed: ResMut<MazeSeed>,
    roots: Query<Entity, With<ProceduralTestRoot>>,
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
        spawn_maze(&mut commands, seed.0);
    }
}

fn spawn_maze(commands: &mut Commands, seed: u64) {
    let maze = generate_maze(MAZE_WIDTH, MAZE_HEIGHT, seed);
    let maze_center_y = -24.0;

    for y in 0..maze.height() {
        for x in 0..maze.width() {
            let color = match (x, y, maze.tile(x, y)) {
                (1, 0, _) => Color::srgb(0.25, 0.8, 0.45),
                (end_x, end_y, _) if end_x == maze.width() - 2 && end_y == maze.height() - 1 => {
                    Color::srgb(0.95, 0.4, 0.35)
                }
                (_, _, Tile::Wall) => Color::srgb(0.16, 0.22, 0.3),
                (_, _, Tile::Floor) => Color::srgb(0.82, 0.88, 0.9),
            };

            let world_x = (x as f32 - (maze.width() - 1) as f32 / 2.0) * TILE_SIZE;
            let world_y = ((maze.height() - 1) as f32 / 2.0 - y as f32) * TILE_SIZE
                + maze_center_y;

            commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE_SIZE - 1.0)),
                Transform::from_xyz(world_x, world_y, 0.0),
                ProceduralTestRoot,
            ));
        }
    }

    commands.spawn((
        Text::new(format!(
            "Procedural Maze   Seed: {seed}   R: Regenerate   Esc: Menu"
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
        ProceduralTestRoot,
    ));
}

fn despawn_maze_test(
    mut commands: Commands,
    query: Query<Entity, With<ProceduralTestRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
