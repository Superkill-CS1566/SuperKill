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
            tiles: vec![Tile::Wall; width * height],
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
}

// Uses depth-first search to carve passages between odd-numbered grid cells.
pub fn generate_maze(width: usize, height: usize, seed: u64) -> MazeGrid {
    assert!(width >= 3 && height >= 3, "maze must be at least 3 by 3");
    assert!(
        width % 2 == 1 && height % 2 == 1,
        "maze dimensions must be odd"
    );

    let mut grid = MazeGrid::new(width, height);
    let mut rng = SimpleRng::new(seed);
    let mut stack = vec![(1_usize, 1_usize)];
    grid.set_floor(1, 1);

    while let Some(&(x, y)) = stack.last() {
        let mut directions = [(0_isize, -2_isize), (2, 0), (0, 2), (-2, 0)];
        rng.shuffle(&mut directions);

        let next = directions.into_iter().find_map(|(dx, dy)| {
            let next_x = x as isize + dx;
            let next_y = y as isize + dy;

            if next_x <= 0
                || next_y <= 0
                || next_x >= width as isize - 1
                || next_y >= height as isize - 1
            {
                return None;
            }

            let next_x = next_x as usize;
            let next_y = next_y as usize;
            (grid.tile(next_x, next_y) == Tile::Wall).then_some((next_x, next_y))
        });

        if let Some((next_x, next_y)) = next {
            grid.set_floor((x + next_x) / 2, (y + next_y) / 2);
            grid.set_floor(next_x, next_y);
            stack.push((next_x, next_y));
        } else {
            stack.pop();
        }
    }

    grid.set_floor(1, 0);
    grid.set_floor(width - 2, height - 1);
    grid
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.next() as usize % (index + 1);
            values.swap(index, other);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use super::*;

    #[test]
    fn same_seed_produces_same_maze() {
        assert_eq!(generate_maze(21, 13, 7), generate_maze(21, 13, 7));
    }

    #[test]
    fn every_floor_tile_is_connected() {
        let grid = generate_maze(21, 13, 11);
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
