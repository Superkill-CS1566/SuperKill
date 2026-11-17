//! The level's solid ground, as plain rectangles.
//!
//! Every terrain type comes down to "a list of blocks," so the fighter, the
//! swords, and the server all collide against the same thing no matter which
//! generator made it.

use serde::{Deserialize, Serialize};

use super::{ARENA_WIDTH, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN};
use crate::perlin_noise::{sample_chunk, terrain_at_column};
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
    /// Endless noise hills, generated around the play area as it scrolls —
    /// for the autoscroll level.
    NoiseEndless,
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

// endless terrain
/// Hill height for the endless level. Separate from NOISE_AMPLITUDE so the
/// two levels can be tuned apart — lower this if the step test fails.
const ENDLESS_AMPLITUDE: f32 = NOISE_AMPLITUDE;
/// How far each side of the play area's center stays generated.
const WINDOW_HALF_WIDTH: f32 = ARENA_WIDTH * 1.5;

// maze terrain
const MAZE_WIDTH: usize = 39;
const MAZE_HEIGHT: usize = 3;
const MAZE_TILE_SIZE: f32 = 100.0;
/// The old `spawn_maze` always generated maze level 1.
const MAZE_LEVEL: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct Terrain {
    pub blocks: Vec<Block>,
    /// Endless terrain has no level edges: fighters aren't clamped to
    /// `LEVEL_LEN`, and `blocks` is only a window around the play area.
    pub endless: bool,
    /// Column index of `blocks[0]`. Endless terrain only.
    pub first_col: i32,
    pub seed: u64,
}

impl Terrain {
    pub fn generate(kind: TerrainKind, seed: u64) -> Self {
        match kind {
            TerrainKind::Noise => Self::noise(seed),
            TerrainKind::NoiseEndless => Self::noise_endless(seed),
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
            ..Default::default()
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
        Self {
            blocks,
            ..Default::default()
        }
    }

    /// Endless hills. Every column's height is a pure function of its index
    /// and the seed, so the server and every client build identical ground no
    /// matter where their windows started.
    pub fn noise_endless(seed: u64) -> Self {
        let mut terrain = Self {
            endless: true,
            seed,
            ..Default::default()
        };
        terrain.update_window(0.0);
        terrain
    }

    /// Keep the columns around `center` generated and drop the rest. Call once
    /// per tick, before the match steps, with the play area's center. Does
    /// nothing for finite terrain.
    pub fn update_window(&mut self, center: f32) {
        if !self.endless {
            return;
        }
        let first = ((center - WINDOW_HALF_WIDTH) / NOISE_BLOCK_WIDTH).floor() as i32;
        let last = ((center + WINDOW_HALF_WIDTH) / NOISE_BLOCK_WIDTH).ceil() as i32;
        let have = self.blocks.len() as i32;
        if have > 0 && first == self.first_col && last == self.first_col + have - 1 {
            return;
        }

        self.first_col = first;
        self.blocks = (first..=last)
            .map(|col| endless_column(col, self.seed))
            .collect();
    }

    /// The old `spawn_maze`: wall tiles merged into as few rectangles as
    /// possible, centered slightly above the origin to incorporate the floor.
    pub fn maze(seed: u64, level: u32) -> Self {
        let grid = rd_generate_maze(MAZE_WIDTH, MAZE_HEIGHT, seed, level);

        let start_x = -(grid.width() as f32 * MAZE_TILE_SIZE) / 2.0;
        let start_y = (grid.height() as f32 * MAZE_TILE_SIZE) / 2.0 - 120.0;

        let mut blocks = Vec::new();
        for (x, y, width, height) in merge_wall_tiles(&grid) {
            blocks.push(Block {
                x: start_x + (x as f32 * MAZE_TILE_SIZE) + (width as f32 * MAZE_TILE_SIZE) / 2.0,
                y: start_y - (y as f32 * MAZE_TILE_SIZE) - (height as f32 * MAZE_TILE_SIZE) / 2.0,
                width: width as f32 * MAZE_TILE_SIZE,
                height: height as f32 * MAZE_TILE_SIZE,
            });
        }
        Self {
            blocks,
            ..Default::default()
        }
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

/// One endless column, centered on `col * NOISE_BLOCK_WIDTH`.
fn endless_column(col: i32, seed: u64) -> Block {
    let surface_y = GROUND_Y + terrain_at_column(col, seed) * ENDLESS_AMPLITUDE + ENDLESS_AMPLITUDE;
    let height = surface_y - NOISE_BOTTOM_Y;
    Block {
        x: col as f32 * NOISE_BLOCK_WIDTH,
        y: NOISE_BOTTOM_Y + height / 2.0,
        width: NOISE_BLOCK_WIDTH,
        height,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A full jump peaks near 102 units; steps well under that stay clearable
    /// with the edge closing in.
    const MAX_STEP: f32 = 40.0;

    #[test]
    fn endless_steps_stay_jumpable_everywhere() {
        let mut terrain = Terrain::noise_endless(DEFAULT_SEED);
        // far left, the origin, and far right — covers negative columns
        for center in [-50_000.0, -1_000.0, 0.0, 12_345.0, 50_000.0] {
            terrain.update_window(center);
            for pair in terrain.blocks.windows(2) {
                let step = (pair[1].top() - pair[0].top()).abs();
                assert!(step <= MAX_STEP, "step of {step} near x = {}", pair[0].x);
            }
        }
    }

    #[test]
    fn endless_window_follows_the_center_and_matches_pure_columns() {
        let mut terrain = Terrain::noise_endless(DEFAULT_SEED);
        terrain.update_window(5_000.0);

        let first = terrain.blocks.first().unwrap();
        let last = terrain.blocks.last().unwrap();
        assert!(first.x < 5_000.0 - ARENA_WIDTH && last.x > 5_000.0 + ARENA_WIDTH);

        // the same column built from a different window is identical
        let col = terrain.first_col + 3;
        let mut other = Terrain::noise_endless(DEFAULT_SEED);
        other.update_window(col as f32 * NOISE_BLOCK_WIDTH);
        let a = terrain.blocks[3];
        let b = other.blocks[(col - other.first_col) as usize];
        assert_eq!(a, b);
    }

    #[test]
    fn finite_noise_is_unchanged() {
        let terrain = Terrain::noise(DEFAULT_SEED);
        assert!(!terrain.endless);
        assert_eq!(terrain.blocks.len(), (LEVEL_LEN / NOISE_BLOCK_WIDTH) as usize + 1);
    }
}
