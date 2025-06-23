// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Kernel-X (SIGMA=0.4)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var mmkernel_texture: texture_2d<f32>; // previous result (MMKERNEL, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<r32float, write>; // gaussian output (MMKERNEL, 1ch x1)

const SIGMA: f32 = 0.4;

fn gaussian(x: f32, s: f32, m: f32) -> f32 {
    return (1.0 / (s * sqrt(2.0 * 3.14159))) * exp(-0.5 * pow(abs(x - m) / s, 2.0));
}

fn lumGaussian(pos: vec2i, d: vec2i) -> f32 {
    let dims = textureDimensions(mmkernel_texture);
    let bound = vec2i(dims) - 1;
    let s = SIGMA * f32(dims.y) / 1080.0;
    let kernel_size = s * 2.0 + 1.0;

    var g = textureLoad(mmkernel_texture, pos, 0).x * gaussian(0.0, s, 0.0);
    var gn = gaussian(0.0, s, 0.0);

    let pos_minus = clamp(pos - d, vec2i(0), bound);
    let pos_plus = clamp(pos + d, vec2i(0), bound);
    g += (textureLoad(mmkernel_texture, pos_minus, 0).x + textureLoad(mmkernel_texture, pos_plus, 0).x) * gaussian(1.0, s, 0.0);
    gn += gaussian(1.0, s, 0.0) * 2.0;

    for (var i: i32 = 2; i < i32(ceil(kernel_size)); i++) {
        let pos_minus_i = clamp(pos - (d * i), vec2i(0), bound);
        let pos_plus_i = clamp(pos + (d * i), vec2i(0), bound);
        g += (textureLoad(mmkernel_texture, pos_minus_i, 0).x + textureLoad(mmkernel_texture, pos_plus_i, 0).x) * gaussian(f32(i), s, 0.0);
        gn += gaussian(f32(i), s, 0.0) * 2.0;
    }

    return g / gn;
}

fn process(pos: vec2i) {
    let d = vec2i(1, 0); // X direction
    let result = lumGaussian(pos, d);
    textureStore(output_texture, pos, vec4f(result, 0.0, 0.0, 1.0));
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
