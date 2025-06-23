// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var sobel_texture: texture_2d<f32>; // X-direction kernel gradient (from pass 6, 2ch HQ=x1, F=x0.5, VF=x0.25)
@group(0) @binding(1) var output_texture: texture_storage_2d<rg32float, write>; // Y-direction kernel gradient (2ch HQ=x1, F=x0.5, VF=x0.25)

fn process(pos: vec2i) {
    let bound = vec2i(textureDimensions(sobel_texture)) - 1;

    let tx = textureLoad(sobel_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).r;
    let cx = textureLoad(sobel_texture, pos, 0).r;
    let bx = textureLoad(sobel_texture, min(pos + vec2i(0, 1), bound), 0).r;

    let ty = textureLoad(sobel_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).g;
    let by = textureLoad(sobel_texture, min(pos + vec2i(0, 1), bound), 0).g;

    let xgrad = (tx + cx + cx + bx) / 8.0;
    let ygrad = (-ty + by) / 8.0;

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
