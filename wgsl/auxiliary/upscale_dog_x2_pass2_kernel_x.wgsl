// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // X-direction gaussian blur with min/max (3ch x1)

fn max3v(a: f32, b: f32, c: f32) -> f32 {
    return max(max(a, b), c);
}

fn min3v(a: f32, b: f32, c: f32) -> f32 {
    return min(min(a, b), c);
}

fn minmax3(pos: vec2i, d: vec2i) -> vec2f {
    let bound = vec2i(textureDimensions(luma_texture)) - 1;

    let a = textureLoad(luma_texture, max(pos - d, vec2i(0)), 0).r;
    let b = textureLoad(luma_texture, pos, 0).r;
    let c = textureLoad(luma_texture, min(pos + d, bound), 0).r;

    return vec2f(min3v(a, b, c), max3v(a, b, c));
}

fn lum_gaussian7(pos: vec2i, d: vec2i) -> f32 {
    let bound = vec2i(textureDimensions(luma_texture)) - 1;

    var g: f32 = 0.0;
    g += (textureLoad(luma_texture, max(pos - (d + d), vec2i(0)), 0).r + textureLoad(luma_texture, min(pos + (d + d), bound), 0).r) * 0.06136;
    g += (textureLoad(luma_texture, max(pos - d, vec2i(0)), 0).r + textureLoad(luma_texture, min(pos + d, bound), 0).r) * 0.24477;
    g += textureLoad(luma_texture, pos, 0).r * 0.38774;
    return g;
}

fn process(pos: vec2i) {
    let gaussian = lum_gaussian7(pos, vec2i(1, 0));
    let minmax = minmax3(pos, vec2i(1, 0));

    textureStore(output_texture, pos, vec4f(gaussian, minmax.x, minmax.y, 1.0));
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
