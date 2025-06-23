// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2 Upscale and Warp

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var input_sampler: sampler; // sampler for input texture
@group(0) @binding(2) var lumad2_texture: texture_2d<f32>; // second gradients (LUMAD2, 2ch x1)
@group(0) @binding(3) var lumad2_sampler: sampler; // sampler for lumad2 texture
@group(0) @binding(4) var output_texture: texture_storage_2d<rgba32float, write>; // upscaled output (MAINTEMPTHIN, 4ch x2)

const STRENGTH: f32 = 0.4; // Strength of warping for each iteration
const ITERATIONS: i32 = 1; // Number of iterations for the forwards solver

fn process(pos: vec2i) {
    let input_dims = vec2f(textureDimensions(input_texture));
    let output_dims = vec2f(textureDimensions(output_texture));

    let pt = vec2f(1.0) / input_dims;
    let rel_str = input_dims.y / 1080.0 * STRENGTH;

    // Start with normalized texture coordinates (equivalent to HOOKED_pos)
    var sample_pos = (vec2f(pos) + 0.5) / output_dims;

    for (var i: i32 = 0; i < ITERATIONS; i++) {
        // Sample sobel texture at current position using normalized coordinates
        let dn = textureSampleLevel(lumad2_texture, lumad2_sampler, sample_pos, 0.0).xy;
        let dd = (dn / (length(dn) + 0.01)) * pt * rel_str;
        sample_pos -= dd;
    }

    // Sample input texture at final position using normalized coordinates
    let result = textureSampleLevel(input_texture, input_sampler, sample_pos, 0.0);
    textureStore(output_texture, pos, result);
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
