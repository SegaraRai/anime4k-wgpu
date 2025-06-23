// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Luma (Upscaled)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var maintempthin_texture: texture_2d<f32>; // upscaled texture (MAINTEMPTHIN, 4ch x2)
@group(0) @binding(2) var output_texture: texture_storage_2d<r32float, write>; // upscaled luminance (MAINTEMP, 1ch x2)

fn get_luma(rgba: vec4f) -> f32 {
    return dot(vec4f(0.299, 0.587, 0.114, 0.0), rgba);
}

fn process(pos: vec2i) {
    let color = textureLoad(maintempthin_texture, pos, 0);
    let luma = get_luma(color);
    textureStore(output_texture, pos, vec4f(luma, 0.0, 0.0, 1.0));
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
