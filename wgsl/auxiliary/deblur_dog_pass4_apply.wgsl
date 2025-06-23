// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch x1)
@group(0) @binding(2) var mmkernel_texture: texture_2d<f32>; // min/max gaussian blur (from pass 3, 3ch x1)
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba32float, write>; // deblurred result (4ch x1)

const STRENGTH: f32 = 0.6;
const BLUR_CURVE: f32 = 0.6;
const BLUR_THRESHOLD: f32 = 0.1;
const NOISE_THRESHOLD: f32 = 0.001;

fn process(pos: vec2i) {
    let luma_val = textureLoad(luma_texture, pos, 0).r;
    let gaussian_val = textureLoad(mmkernel_texture, pos, 0).r;
    let min_val = textureLoad(mmkernel_texture, pos, 0).g;
    let max_val = textureLoad(mmkernel_texture, pos, 0).b;

    var c = (luma_val - gaussian_val) * STRENGTH;

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

    let cc = clamp(c_t + luma_val, min_val, max_val) - luma_val;

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let hooked_color = textureLoad(input_texture, pos, 0);
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
