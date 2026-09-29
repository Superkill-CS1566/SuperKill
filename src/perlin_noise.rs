// a function to generate random vectors for each int
fn hash(i: i32, seed: u32) -> u32 {
    let mut h = seed;

    h ^= i as u32;
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
    return (i * i * i * ( i * (6.0 * i - 15.0) + 10.0)).clamp(0.0, 1.0);
}

// implemenation of linear interpolation based on Wikipedia article
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return (1.0 - t) * a + (t * b);
}

// retrieve values of points with two integers (currently locked to i and i+1)
pub fn inter(i: i32, seed: u32) -> Vec<f32> {
    
    // calculate the gradients of endpoints
    let gradient_i = gradient_at_point(i, seed);
    let gradient_j = gradient_at_point(i + 1, seed);

    let mut inter_points: Vec<f32> = Vec::new();

    for j in 0..10 {
        // get point value
        let t = (j as f32) / 10.0;
        let point = (i as f32) + t;
        
        // weight each gradient to point
        let weight_i = gradient_i * (point - (i as f32));
        let weight_j = gradient_j * (point - ((i + 1) as f32));

        // retrieve a faded point value based on the smootherstep function
        let fade = smootherstep(t);

        // push the gradient for point onto our vec
        inter_points.push(lerp(weight_i, weight_j, fade));
    }

    inter_points
}
