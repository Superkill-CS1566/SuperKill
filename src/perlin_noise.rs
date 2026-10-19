use crate::simplerng::SimpleRng;

// implementation of smootherstep based on Wikipedia article
fn smootherstep(i: f32) -> f32 {
    (i * i * i * (i * (6.0 * i - 15.0) + 10.0)).clamp(0.0, 1.0)
}

// implemenation of linear interpolation based on Wikipedia article
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + (t * b)
}

// retrieve noise value at a specific point
fn noise(x: f32, seed: u64) -> f32 {
    // calculate upper and lower integer values
    let lower = x.floor() as i32;
    let upper = lower + 1;

    let gradient_i = SimpleRng::gradient_at_point(lower, seed);
    let gradient_j = SimpleRng::gradient_at_point(upper, seed);

    // run smootherstep on our position
    let t = x - lower as f32;
    let fade = smootherstep(t);

    // get the weight of each integer gradient on our position
    let weight_i = gradient_i * (x - lower as f32);
    let weight_j = gradient_j * (x - upper as f32);

    // use linear interpolation to get our result
    lerp(weight_i, weight_j, fade)
}

// run multiple octaves of noise in order to get more variety in terrain
// NOTE FOR TEAM: we can mess around with this later to get better results
fn terrain_at_point(i: f32, seed: u64) -> f32 {
    let mut value = 0.0;

    value+=noise(i, seed);
    value+=noise(i * 2.0, seed) * 0.5;
    value+=noise(i * 4.0, seed) * 0.25;
    value+=noise(i * 8.0, seed) * 0.125;

    value
}

// generate a chunk of noise values (for level creation)
pub fn sample_chunk(start: i32, range: u32, seed: u64) -> Vec<f32> {
    let mut chunk = Vec::new();

    for i in start..=(start + range as i32) {
        let x = (start as f32) + ((i as f32) / 10.0);
        chunk.push(terrain_at_point(x, seed));
    }

    chunk
}

// create a noise struct when doing the test level