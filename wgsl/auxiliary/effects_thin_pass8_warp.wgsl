// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var input_sampler: sampler; // sampler for input_texture
@group(0) @binding(2) var sobel_texture: texture_2d<f32>; // Y-direction kernel gradient (from pass 7, 2ch HQ=x1, F=x0.5, VF=x0.25)
@group(0) @binding(3) var sobel_sampler: sampler; // sampler for sobel_texture
@group(0) @binding(4) var output_texture: texture_storage_2d<rgba32float, write>; // final warped thin result (4ch x1)

const STRENGTH: f32 = 0.6;
const ITERATIONS: i32 = 1;

fn process(pos: vec2i) {
    let input_dims = vec2f(textureDimensions(input_texture));
    let output_dims = vec2f(textureDimensions(output_texture));

    let pt = vec2f(1.0) / input_dims;
    let rel_str = input_dims.y / 1080.0 * STRENGTH;

    // Start with normalized texture coordinates (equivalent to HOOKED_pos)
    var sample_pos = (vec2f(pos) + 0.5) / output_dims;

    for (var i: i32 = 0; i < ITERATIONS; i++) {
        // Sample sobel texture at current position using normalized coordinates
        let dn = textureSampleLevel(sobel_texture, sobel_sampler, sample_pos, 0.0).xy;
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
