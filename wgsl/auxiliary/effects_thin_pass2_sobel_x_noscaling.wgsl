// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch HQ=x1)
@group(0) @binding(1) var output_texture: texture_storage_2d<rg32float, write>; // X-direction sobel gradient (2ch HQ=x1)

fn process(pos: vec2i) {
    let bound = vec2i(textureDimensions(output_texture)) - 1;

    let l = textureLoad(luma_texture, max(pos + vec2i(-1, 0), vec2i(0)), 0).r;
    let c = textureLoad(luma_texture, pos, 0).r;
    let r = textureLoad(luma_texture, min(pos + vec2i(1, 0), bound), 0).r;

    let xgrad = -l + r;
    let ygrad = l + c + c + r;

    textureStore(output_texture, pos, vec4f(xgrad, ygrad, 0.0, 1.0));
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
