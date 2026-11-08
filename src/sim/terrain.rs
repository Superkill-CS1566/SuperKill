//! The level's solid ground, as plain rectangles.
//!
//! Every terrain type comes down to "a list of blocks," so the fighter, the
//! swords, and the server all collide against the same thing no matter which
//! generator made it.

use serde::{Deserialize, Serialize};

use super::{GROUND_THICKNESS, GROUND_Y, LEVEL_LEN};
use crate::perlin_noise::sample_chunk;
use crate::recursive_generation::{rd_generate_maze, MazeGrid, Tile};

/// One solid block, by center and size — the same shape the old `Platform`
/// entities had.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Block {
    pub fn left(&self) -> f32 {
        self.x - self.width / 2.0
    }
    pub fn right(&self) -> f32 {
        self.x + self.width / 2.0
    }
    pub fn top(&self) -> f32 {
        self.y + self.height / 2.0
    }
    pub fn bottom(&self) -> f32 {
        self.y - self.height / 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainKind {
    /// The real level: hills from Perlin noise.
    Noise,
    /// Testing only — a single flat floor.
    Flat,
    /// Testing only — a recursive-division maze.
    Maze,
}

/// The seed every level uses for now (the old `Seed(1)` resource). The server
/// and each client must generate from the same kind and seed to agree on the
/// ground.
pub const DEFAULT_SEED: u64 = 1;

// noise terrain
const NOISE_BLOCK_WIDTH: f32 = 40.0;
const NOISE_AMPLITUDE: f32 = GROUND_THICKNESS;
const NOISE_BOTTOM_Y: f32 = -400.0;

// maze terrain
const MAZE_WIDTH: usize = 39;
const MAZE_HEIGHT: usize = 21;
const MAZE_TILE_SIZE: f32 = 50.0;
/// The old `spawn_maze` always generated maze level 1.
const MAZE_LEVEL: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct Terrain {
    pub blocks: Vec<Block>,
}

impl Terrain {
    pub fn generate(kind: TerrainKind, seed: u64) -> Self {
        match kind {
            TerrainKind::Noise => Self::noise(seed),
            TerrainKind::Flat => Self::flat(),
            TerrainKind::Maze => Self::maze(seed, MAZE_LEVEL),
        }
    }

    /// One floor spanning the whole level.
    pub fn flat() -> Self {
        Self {
            blocks: vec![Block {
                x: 0.0,
                y: GROUND_Y,
                width: LEVEL_LEN,
                height: GROUND_THICKNESS,
            }],
        }
    }

    /// A row of columns whose heights follow Perlin noise — the old
    /// `spawn_noise_terrain`, producing the same blocks.
    pub fn noise(seed: u64) -> Self {
        let start_x = -LEVEL_LEN / 2.0;
        let chunk = sample_chunk(0, (LEVEL_LEN / NOISE_BLOCK_WIDTH) as u32, seed);

        let mut blocks = Vec::new();
        for (i, noise_value) in chunk.iter().enumerate() {
            let surface_y = GROUND_Y + (noise_value * NOISE_AMPLITUDE) + NOISE_AMPLITUDE;
            let height = surface_y - NOISE_BOTTOM_Y;

            blocks.push(Block {
                x: start_x + i as f32 * NOISE_BLOCK_WIDTH,
                y: NOISE_BOTTOM_Y + height / 2.0,
                width: NOISE_BLOCK_WIDTH,
                height,
            });
        }
        Self { blocks }
    }

    /// The old `spawn_maze`: wall tiles merged into as few rectangles as
    /// possible, centered on the origin.
    pub fn maze(seed: u64, level: u32) -> Self {
        let grid = rd_generate_maze(MAZE_WIDTH, MAZE_HEIGHT, seed, level);

        let start_x = -(grid.width() as f32 * MAZE_TILE_SIZE) / 2.0;
        let start_y = (grid.height() as f32 * MAZE_TILE_SIZE) / 2.0;

        let mut blocks = Vec::new();
        for (x, y, width, height) in merge_wall_tiles(&grid) {
            blocks.push(Block {
                x: start_x + (x as f32 * MAZE_TILE_SIZE) + (width as f32 * MAZE_TILE_SIZE) / 2.0,
                y: start_y - (y as f32 * MAZE_TILE_SIZE) - (height as f32 * MAZE_TILE_SIZE) / 2.0,
                width: width as f32 * MAZE_TILE_SIZE,
                height: height as f32 * MAZE_TILE_SIZE,
            });
        }
        Self { blocks }
    }

    /// The top of the ground directly under `x`. Prefers the highest surface
    /// at or below `from_y`, so a dropped sword lands on what's beneath it
    /// rather than jumping up onto a ledge overhead. If nothing is below — it
    /// ended up inside a hill — it settles on the highest surface at `x`.
    pub fn surface_below(&self, x: f32, from_y: f32) -> f32 {
        let mut below: Option<f32> = None;
        let mut highest: Option<f32> = None;

        for block in &self.blocks {
            // only blocks whose span covers this x
            if (x - block.x).abs() > block.width / 2.0 {
                continue;
            }
            let top = block.top();

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
}

// Moved here unchanged from game.rs.
fn merge_wall_tiles(grid: &MazeGrid) -> Vec<(usize, usize, usize, usize)> {
    let width = grid.width();
    let height = grid.height();

    let mut visited = vec![vec![false; width]; height];

    // Each tuple represents (x, y, width, height) in tiles
    let mut rectangles = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if visited[y][x] || grid.tile(x, y) != Tile::Wall {
                continue;
            }

            // Find the maximum horizontal stretch
            let mut rect_width = 0;

            while x + rect_width < width
                && !visited[y][x + rect_width]
                && grid.tile(x + rect_width, y) == Tile::Wall
            {
                rect_width += 1;
            }

            // Find the maximum vertical stretch
            let mut rect_height = 1;

            'outer: while y + rect_height < height {
                for dx in 0..rect_width {
                    let next_y = y + rect_height;
                    let next_x = x + dx;

                    if visited[next_y][next_x] || grid.tile(next_x, next_y) != Tile::Wall {
                        break 'outer;
                    }
                }

                rect_height += 1;
            }

            // Mark all tiles in this rectangle as visited
            for dy in 0..rect_height {
                for dx in 0..rect_width {
                    visited[y + dy][x + dx] = true;
                }
            }

            rectangles.push((x, y, rect_width, rect_height));
        }
    }

    rectangles
}