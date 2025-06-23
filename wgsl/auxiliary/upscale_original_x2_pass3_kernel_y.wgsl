// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var lumad_texture: texture_2d<f32>; // horizontal gradient (LUMAD from pass 2, 2ch x2)
@group(0) @binding(1) var output_texture: texture_storage_2d<rg32float, write>; // refined gradient with strength (LUMAD, 2ch x2)

// Settings
const REFINE_STRENGTH: f32 = 0.5;
const REFINE_BIAS: f32 = 0.0;

// Polynomial fit obtained by minimizing MSE error on image
const P5: f32 = 11.68129591;
const P4: f32 = -42.46906057;
const P3: f32 = 60.28286266;
const P2: f32 = -41.84451327;
const P1: f32 = 14.05517353;
const P0: f32 = -1.081521930;

fn power_function(x: f32) -> f32 {
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x2 * x3;

    return P5 * x5 + P4 * x4 + P3 * x3 + P2 * x2 + P1 * x + P0;
}

fn process(pos: vec2i) {
    let bound = vec2i(textureDimensions(lumad_texture)) - 1;

    // Sample gradient values with bounds checking
    let tx = textureLoad(lumad_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).r;
    let cx = textureLoad(lumad_texture, pos, 0).r;
    let bx = textureLoad(lumad_texture, min(pos + vec2i(0, 1), bound), 0).r;

    let ty = textureLoad(lumad_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).g;
    let by = textureLoad(lumad_texture, min(pos + vec2i(0, 1), bound), 0).g;

    // Compute gradients
    let xgrad = tx + cx + cx + bx;
    let ygrad = -ty + by;

    let sobel_norm = clamp(sqrt(xgrad * xgrad + ygrad * ygrad), 0.0, 1.0);
    let dval = clamp(power_function(clamp(sobel_norm, 0.0, 1.0)) * REFINE_STRENGTH + REFINE_BIAS, 0.0, 1.0);

    textureStore(output_texture, pos, vec4f(sobel_norm, dval, 0.0, 1.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let dims = textureDimensions(output_texture);
    if global_id.x >= dims.x || global_id.y >= dims.y {
        return;
    }

    process(vec2i(global_id.xy));
}

@compute @workgroup_size(8, 8)
fn main_unchecked(@builtin(global_invocation_id) global_id: vec3u) {
    process(vec2i(global_id.xy));
}
