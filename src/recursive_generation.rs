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

// Uses recursive division to construct walls/create paths in odd-numbered cells.
pub fn rd_generate_maze(width: usize, height: usize, seed: u64) -> MazeGrid {
    assert!(width >= 3 && height >= 3, "maze must be at least 3 by 3");
    assert!(
        width % 2 == 1 && height % 2 == 1,
        "maze dimensions must be odd"
    );

    let mut grid = MazeGrid::new(width, height);
    let mut rng = SimpleRng::new(seed);
    let is_horizon = false;
    divide(&mut grid, &mut rng, 0, 0, width, height, is_horizon);
    grid
}

// Helper function to determine direction of cut
pub fn choose_orientation(width: usize, height: usize, prev: bool) -> bool {
    if width < height {
        return true;
    } else if height < width {
        return false;
    } else {
        return !prev;
    }
}

// Recursive function used to divide maze
fn divide(grid: &mut MazeGrid, rng: &mut SimpleRng, x: usize, y: usize, width: usize, height: usize, is_horizon: bool) {
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

    if is_horizon == false {
        curr_x = x + rng.range(1, width - 2, true);
        for curr_y in y..(y + height) {
            grid.set_wall(curr_x, curr_y);
        }
        curr_y = rng.range(y, end_y, false);
        grid.set_floor(curr_x, curr_y);
        next_w = curr_x - x;
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
        next_x = curr_x + 1;
        next_w = x + width - next_x;
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
    } else if is_horizon == true {
        curr_y = y + rng.range(1, height - 2, true);
        for curr_x in x..(x + width) {
            grid.set_wall(curr_x, curr_y);
        }
        curr_x = rng.range(x, end_x, false);
        grid.set_floor(curr_x, curr_y);
        next_h = curr_y - y;
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
        next_y = curr_y + 1;
        next_h = y + height - next_y;
        divide(grid, rng, next_x, next_y, next_w, next_h, choose_orientation(next_w, next_h, is_horizon));
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 21;
        self.state ^= self.state >> 35;
        self.state ^= self.state << 4;
        self.state
    }

    // Uses the rng function to get a random number that's either odd or even
    fn range(&mut self, low: usize, high: usize, is_odd: bool) -> usize {
        if low >= high {
            return low;
        }
        let random = self.next() as usize;
        let range = high - low + 1;
        let mut val = (random % range) + low;
        if (val % 2 == 0 && is_odd) || (val % 2 == 1 && !is_odd) {
            if val + 1 <= high {
                val += 1;
            } else if val - 1 >= low {
                val -= 1;
            }
        }
        val
    }
}

// Used same tests as procedural_generation.rs
#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use super::*;

    #[test]
    fn rd_same_seed_produces_same_maze() {
        assert_eq!(rd_generate_maze(55, 33, 11), rd_generate_maze(55, 33, 11));
    }

    #[test]
    fn rd_every_floor_tile_is_connected() {
        let grid = rd_generate_maze(21, 13, 11);
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