
pub struct SimpleRng {
    state: u64,
}


impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    // a function to generate a
    fn hash(level: i32, seed: u64) -> u64 {
        let mut h = seed ^ (level as u64);

        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;

        h
    }

    pub fn next(&mut self) -> u64 {
        self.state ^= self.state << 21;
        self.state ^= self.state >> 35;
        self.state ^= self.state << 4;
        self.state
    }

    // Uses the rng function to get a random number that's either odd or even
    pub fn range(&mut self, low: usize, high: usize, is_odd: bool) -> usize {
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

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.next() as usize % (index + 1);
            values.swap(index, other);
        }
    }


    pub fn gradient_at_point(i: i32, seed: u64) -> f32 {
        let h = Self::hash(i, seed);

        (h as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32
    }

    pub fn derive_seed(master_seed: u64, level: u32) -> u64 {
        Self::hash(level as i32, master_seed)
    }
}