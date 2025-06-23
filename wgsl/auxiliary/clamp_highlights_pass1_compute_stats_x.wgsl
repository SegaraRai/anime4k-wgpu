// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var output_texture: texture_storage_2d<r32float, write>; // max luminance per row (1ch x1)

const KERNEL_SIZE: i32 = 5;
const KERNEL_HALF_SIZE: i32 = 2;

fn get_luma(rgba: vec4f) -> f32 {
    return dot(vec4f(0.299, 0.587, 0.114, 0.0), rgba);
}

fn process(pos: vec2i) {
    var gmax: f32 = 0.0;
    for (var i: i32 = 0; i < KERNEL_SIZE; i++) {
        let sample_coord = pos + vec2i(i - KERNEL_HALF_SIZE, 0);
        let g = get_luma(textureLoad(input_texture, sample_coord, 0));
        gmax = max(g, gmax);
    }

    textureStore(output_texture, pos, vec4f(gmax, 0.0, 0.0, 1.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let dims = textureDimensions(input_texture);
    if global_id.x >= dims.x || global_id.y >= dims.y {
        return;
    }

    process(vec2i(global_id.xy));
}

@compute @workgroup_size(8, 8)
fn main_unchecked(@builtin(global_invocation_id) global_id: vec3u) {
    process(vec2i(global_id.xy));
}
