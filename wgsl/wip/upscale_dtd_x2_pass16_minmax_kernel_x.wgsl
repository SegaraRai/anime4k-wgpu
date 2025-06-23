// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Kernel-X (Min/Max)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var maintemp_texture: texture_2d<f32>; // upscaled luminance (MAINTEMP, 1ch x2)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // min/max kernel output (MMKERNEL, 3ch x2)

fn max3v(a: f32, b: f32, c: f32) -> f32 {
    return max(max(a, b), c);
}

fn min3v(a: f32, b: f32, c: f32) -> f32 {
    return min(min(a, b), c);
}

fn minmax3(pos: vec2i, d: vec2i) -> vec2f {
    let dims = textureDimensions(maintemp_texture);
    let bound = vec2i(dims) - 1;
    let pos_minus = clamp(pos - d, vec2i(0), bound);
    let pos_center = pos;
    let pos_plus = clamp(pos + d, vec2i(0), bound);

    let a = textureLoad(maintemp_texture, pos_minus, 0).x;
    let b = textureLoad(maintemp_texture, pos_center, 0).x;
    let c = textureLoad(maintemp_texture, pos_plus, 0).x;

    return vec2f(min3v(a, b, c), max3v(a, b, c));
}

fn lumGaussian7(pos: vec2i, d: vec2i) -> f32 {
    let dims = textureDimensions(maintemp_texture);
    let bound = vec2i(dims) - 1;
    let pos_minus2 = clamp(pos - (d + d), vec2i(0), bound);
    let pos_plus2 = clamp(pos + (d + d), vec2i(0), bound);
    let pos_minus = clamp(pos - d, vec2i(0), bound);
    let pos_plus = clamp(pos + d, vec2i(0), bound);
    let pos_center = pos;

    var g = (textureLoad(maintemp_texture, pos_minus2, 0).x + textureLoad(maintemp_texture, pos_plus2, 0).x) * 0.06136;
    g = g + (textureLoad(maintemp_texture, pos_minus, 0).x + textureLoad(maintemp_texture, pos_plus, 0).x) * 0.24477;
    g = g + textureLoad(maintemp_texture, pos_center, 0).x * 0.38774;

    return g;
}

fn process(pos: vec2i) {
    let d = vec2i(1, 0); // X direction

    let gaussian_result = lumGaussian7(pos, d);
    let minmax_result = minmax3(pos, d);

    textureStore(output_texture, pos, vec4f(gaussian_result, minmax_result.x, minmax_result.y, 1.0));
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
