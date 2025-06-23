// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2 Final Apply

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var maintempthin_texture: texture_2d<f32>; // upscaled texture (MAINTEMPTHIN, 4ch x2)
@group(0) @binding(2) var maintemp_texture: texture_2d<f32>; // upscaled luminance (MAINTEMP, 1ch x2)
@group(0) @binding(3) var mmkernel_texture: texture_2d<f32>; // min/max kernel (MMKERNEL, 3ch x2)
@group(0) @binding(4) var output_texture: texture_storage_2d<rgba32float, write>; // final output (4ch x2)

const STRENGTH: f32 = 0.5; // De-blur proportional strength, higher is sharper
const BLUR_CURVE: f32 = 0.8; // De-blur power curve, lower is sharper
const BLUR_THRESHOLD: f32 = 0.1; // Value where curve kicks in
const NOISE_THRESHOLD: f32 = 0.004; // Value where curve stops

fn process(pos: vec2i) {
    let luma_orig = textureLoad(maintemp_texture, pos, 0).x;
    let luma_blur = textureLoad(mmkernel_texture, pos, 0).x;
    var c = (luma_orig - luma_blur) * STRENGTH;

    let t_range = BLUR_THRESHOLD - NOISE_THRESHOLD;

    var c_t = abs(c);
    if c_t > NOISE_THRESHOLD {
        c_t = (c_t - NOISE_THRESHOLD) / t_range;
        c_t = pow(c_t, BLUR_CURVE);
        c_t = c_t * t_range + NOISE_THRESHOLD;
        c_t = c_t * sign(c);
    } else {
        c_t = c;
    }

    let min_val = textureLoad(mmkernel_texture, pos, 0).y;
    let max_val = textureLoad(mmkernel_texture, pos, 0).z;
    let cc = clamp(c_t + luma_orig, min_val, max_val) - luma_orig;

    let original_color = textureLoad(maintempthin_texture, pos, 0);

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row... (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let result = original_color + vec4f(cc, cc, cc, 0.0);
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
