// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var stats_texture: texture_2d<f32>; // max luminance per region (from pass 2, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // clamped output (4ch x1)

fn get_luma(rgba: vec4f) -> f32 {
    return dot(vec4f(0.299, 0.587, 0.114, 0.0), rgba);
}

fn process(pos: vec2i) {
    let hooked_color = textureLoad(input_texture, pos, 0);
    let current_luma = get_luma(hooked_color);
    let max_luma = textureLoad(stats_texture, pos, 0).r;
    let new_luma = min(current_luma, max_luma);

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let result = hooked_color - vec4f(current_luma - new_luma);
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
