// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2 Apply Darkening

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var mmkernel_texture: texture_2d<f32>; // blurred texture (MMKERNEL, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // output (4ch x1)

const STRENGTH: f32 = 1.8; // Line darken proportional strength, higher is darker.

fn process(pos: vec2i) {
    let c = textureLoad(mmkernel_texture, pos, 0).x * STRENGTH;
    let original = textureLoad(input_texture, pos, 0);

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row... (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let result = original + vec4f(c, c, c, 0.0);
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
