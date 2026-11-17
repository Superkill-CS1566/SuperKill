use crate::simplerng::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MazeGrid {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
}

impl MazeGrid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::Floor; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn tile(&self, x: usize, y: usize) -> Tile {
        self.tiles[y * self.width + x]
    }

    fn set_floor(&mut self, x: usize, y: usize) {
        self.tiles[y * self.width + x] = Tile::Floor;
    }

    fn set_wall(&mut self, x: usize, y: usize) {
        self.tiles[y * self.width + x] = Tile::Wall;
    }
}
// -----------------------------------------------------------------------
// New Recursive Division Algorithm implemented for the game specifically.
pub fn rd_generate_maze(
    width: usize,
    height: usize,
    master_seed: u64,
    level: u32,
) -> MazeGrid {
    assert!(width >= 3 && height >= 3, "maze must be at least 3 by 3");
    assert!(
        width % 2 == 1 && height % 2 == 1,
        "maze dimensions must be odd"
    );

    let mut grid = MazeGrid::new(width + 2, height + 2);

    let level_seed = SimpleRng::derive_seed(master_seed, level);
    let mut rng = SimpleRng::new(level_seed);

    let is_horizon = false;
    new_divide(&mut grid, &mut rng, 1, 1, width + 1, height + 1, is_horizon);
    for base in 0..(width) {
        // Sets the ground
        grid.set_wall(base, height + 1);
        // Sets the roof
        grid.set_wall(base, 0);
    }
    // Puts gaps to allow spawning
    for middle_x in (((width - 1) / 2) - 2)..(((width - 1) / 2) + 5) {
        if middle_x == (((width - 1) / 2) - 1) || middle_x != (((width - 1) / 2) + 3) {
            grid.set_floor(middle_x, height - 1);
        }
        grid.set_floor(middle_x, height);
    }
    grid
}

// Increases the chance of a vertical split, as it's more playable in the main game.
pub fn new_choose_orientation(
    width: usize,
    height: usize,
    rng: &mut SimpleRng,
) -> bool {
    if height < width {
        return false;
    } else {
        return rng.mostly_false();
    }
}

// If inputs true, returns an odd number. Otherwise returns an even number.
pub fn odd_even(
    mut output: usize,
    low: usize,
    high: usize,
    is_odd: bool,
) -> usize {
    if (output % 2 == 0 && is_odd) || (output % 2 == 1 && !is_odd) {
        if output + 1 <= high {
            output += 1;
        } else if (output - 1 >= low) || (output + 1 > high) {
            output -= 1;
        } else {
            output += 1;
        }
    }
    output
}

fn new_divide(
    grid: &mut MazeGrid,
    rng: &mut SimpleRng,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    is_horizon: bool,
) {
    if width < 3 || height < 3 {
        return;
    }

    let curr_x;
    let curr_y;
    let end_x = x + width - 2;
    let end_y = y + height - 2;
    let mut next_x = x;
    let mut next_y = y;
    let mut next_w = width;
    let mut next_h = height;
    let mut rand;
    let mut orient;

    if is_horizon == false {
        rand = rng.range(2, width - 1);
        curr_x = odd_even(rand + x, 2, width - 1, false);
        println!("{}", curr_x);
        for curr_y in y..(y + height - 1) {
            grid.set_wall(curr_x, curr_y);
        }
        rand = rng.range(y, end_y);
        curr_y = rand;
        grid.set_floor(curr_x, curr_y);
        next_w = curr_x - x;
        // Partitioning the Left Side
        orient = new_choose_orientation(next_w, next_h, rng);
        new_divide(grid, rng, next_x, next_y, next_w, next_h, orient);
        next_x = curr_x + 1;
        next_w = x + width - next_x;
        // Partitioning the Right Side
        orient = new_choose_orientation(next_w, next_h, rng);
        new_divide(grid, rng, next_x, next_y, next_w, next_h, orient);
    } else if is_horizon == true {
        rand = rng.range(2, height - 1);
        curr_y = odd_even(rand + y, 2, height - 1, false);
        for curr_x in x..(x + width - 1) {
            grid.set_wall(curr_x, curr_y);
        }
        rand = rng.range(x, end_x);
        curr_x = odd_even(rand, x, end_x, true);
        grid.set_floor(curr_x, curr_y);
        next_h = curr_y - y;
        // Partitioning the Top Half
        orient = new_choose_orientation(next_w, next_h, rng);
        new_divide(grid, rng, next_x, next_y, next_w, next_h, orient);
        next_y = curr_y + 1;
        next_h = y + height - next_y;
        // Partitioning the Bottom Half
        orient = new_choose_orientation(next_w, next_h, rng);
        new_divide(grid, rng, next_x, next_y, next_w, next_h, orient);
    }
}

// -----------------------------------------------------------------------
// Old Recursive Division Algorithm implemented only for the tests.

// Uses recursive division to construct walls/create paths in odd-numbered cells.
pub fn old_rd_generate_maze(
    width: usize,
    height: usize,
    master_seed: u64,
    level: u32,
) -> MazeGrid {
    assert!(width >= 3 && height >= 3, "maze must be at least 3 by 3");
    assert!(
        width % 2 == 1 && height % 2 == 1,
        "maze dimensions must be odd"
    );

    let mut grid = MazeGrid::new(width, height);

    let level_seed = SimpleRng::derive_seed(master_seed, level);
    let mut rng = SimpleRng::new(level_seed);

    let is_horizon = false;
    divide(&mut grid, &mut rng, 0, 0, width, height, is_horizon);
    grid
}

// Helper function to determine direction of cut.
// If true, splits horizontally. Starts false
pub fn choose_orientation(
    width: usize,
    height: usize,
    prev: bool,
) -> bool {
    if width < height {
        return true;
    } else if height < width {
        return false;
    } else {
        return !prev;
    }
}

// Recursive function used to divide maze
fn divide(
    grid: &mut MazeGrid,
    rng: &mut SimpleRng,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    is_horizon: bool,
) {
    if width < 2 || height < 2 {
        return;
    }

    let curr_x;
    let curr_y;
    let end_x = x + width - 1;
    let end_y = y + height - 1;
    let mut next_x = x;
    let mut next_y = y;
    let mut next_w = width;
    let mut next_h = height;
    let mut rand;

    if is_horizon == false {
        rand = rng.range(1, width - 2);
        curr_x = x + odd_even(rand, 1, width - 2, true);
        for curr_y in y..(y + height) {
            grid.set_wall(curr_x, curr_y);
        }
        rand = rng.range(y, end_y);
        curr_y = odd_even(rand, y, end_y, false);
        grid.set_floor(curr_x, curr_y);
        next_w = curr_x - x;
        // Partitioning the Left Side
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
        next_x = curr_x + 1;
        next_w = x + width - next_x;
        // Partitioning the Right Side
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
    } else if is_horizon == true {
        rand = rng.range(1, height - 2);
        curr_y = y + odd_even(rand, 1, height - 2, true);
        for curr_x in x..(x + width) {
            grid.set_wall(curr_x, curr_y);
        }
        rand = rng.range(x, end_x);
        curr_x = odd_even(rand, x, end_x, false);
        grid.set_floor(curr_x, curr_y);
        next_h = curr_y - y;
        // Partitioning the Top Half
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
        next_y = curr_y + 1;
        next_h = y + height - next_y;
        // Partitioning the Bottom Half
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
    }
}

// Used same tests as procedural_generation.rs
#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use super::*;

    #[test]
    fn rd_same_seed_produces_same_maze() {
        assert_eq!(rd_generate_maze(55, 33, 11, 1), rd_generate_maze(55, 33, 11, 1));
    }

    #[test]
    fn rd_every_floor_tile_is_connected() {
        let grid = rd_generate_maze(21, 13, 11, 1);
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([(1_usize, 0_usize)]);

        while let Some((x, y)) = queue.pop_front() {
            if !visited.insert((x, y)) {
                continue;
            }

            for (dx, dy) in [(0_isize, -1_isize), (1, 0), (0, 1), (-1, 0)] {
                let next_x = x as isize + dx;
                let next_y = y as isize + dy;
                if next_x < 0
                    || next_y < 0
                    || next_x >= grid.width() as isize
                    || next_y >= grid.height() as isize
                {
                    continue;
                }

                let next = (next_x as usize, next_y as usize);
                if grid.tile(next.0, next.1) == Tile::Floor && !visited.contains(&next) {
                    queue.push_back(next);
                }
            }
        }

        let floor_count = (0..grid.height())
            .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.tile(x, y) == Tile::Floor)
            .count();

        assert_eq!(visited.len(), floor_count);
        assert_eq!(grid.tile(grid.width() - 2, grid.height() - 1), Tile::Floor);
    }
}