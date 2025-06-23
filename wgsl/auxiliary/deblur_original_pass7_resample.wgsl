// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // deblurred texture (from pass 6, 4ch x2)
@group(0) @binding(1) var input_sampler: sampler; // sampler for deblurred texture
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // final result (4ch x1)

fn process(pos: vec2i) {
    let output_dims = textureDimensions(output_texture);
    let uv_pos = (vec2f(pos) + 0.5) / vec2f(output_dims);
    let result = textureSampleLevel(input_texture, input_sampler, uv_pos, 0.0);
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
