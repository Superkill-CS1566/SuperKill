// a function to generate random vectors for each int
fn hash(i: i32, seed: u32) -> u32 {
    let mut h = seed;

    h ^= i as u32;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;

    h
}

// retrieve a randomized point based on given seed
fn gradient_at_point(i: i32, seed: u32) -> f32 {
    let h = hash(i, seed) as f32;

    -1.0 + (h / (u32::MAX as f32)) * 2.0
}

// implementation of smootherstep based on Wikipedia article
fn smootherstep(i: f32) -> f32 {
    i * i * i * ( i * (6.0 * i - 15.0) + 10.0).clamp(0.0, 1.0)
}

// implemenation of linear interpolation based on Wikipedia article
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + (t * b)
}

// retrieve noise value at a specific point
pub fn noise(x: f32, seed: u32) -> f32 {
    // calculate upper and lower integer values
    let lower = x.floor() as i32;
    let upper = lower + 1;

    let gradient_i = gradient_at_point(lower, seed);
    let gradient_j = gradient_at_point(upper, seed);

    // run smootherstep on our position
    let t = x - lower as f32;
    let fade = smootherstep(t);

    // get the weight of each integer gradient on our position
    let weight_i = gradient_i * (x - lower as f32);
    let weight_j = gradient_j * (x - upper as f32);

    // use linear interpolation to get our result
    lerp(weight_i, weight_j, fade)
}
