// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch x1)
@group(0) @binding(2) var gauss_texture: texture_2d<f32>; // gaussian blur with min/max (from pass 3, 3ch x1)
@group(0) @binding(3) var common_sampler: sampler; // common sampler for all textures
@group(0) @binding(4) var output_texture: texture_storage_2d<rgba32float, write>; // upscaled deblurred result (4ch x2)

const STRENGTH: f32 = 0.8;

fn process(pos: vec2i) {
    let output_dims = textureDimensions(output_texture);
    let input_dims = textureDimensions(input_texture);

    let uv_pos = (vec2f(pos) + 0.5) / vec2f(output_dims);

    let luma_val = textureSampleLevel(luma_texture, common_sampler, uv_pos, 0.0).r;
    let gauss_vec = textureSampleLevel(gauss_texture, common_sampler, uv_pos, 0.0);

    let c = (luma_val - gauss_vec.r) * STRENGTH;
    let cc = clamp(c + luma_val, gauss_vec.g, gauss_vec.b) - luma_val;

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let hooked_color = textureSampleLevel(input_texture, common_sampler, uv_pos, 0.0);
    let result = hooked_color + vec4f(cc, cc, cc, 0.0);

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
