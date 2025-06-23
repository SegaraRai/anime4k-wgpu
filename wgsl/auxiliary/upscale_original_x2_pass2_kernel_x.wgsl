// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var luma_texture: texture_2d<f32>; // luminance data (LINELUMA from pass 1, 1ch x1)
@group(0) @binding(2) var common_sampler: sampler; // common sampler for all textures
@group(0) @binding(3) var output_texture: texture_storage_2d<rg32float, write>; // horizontal gradient (LUMAD, 2ch x2)

fn process(pos: vec2i) {
    let input_dims = textureDimensions(input_texture);
    let uv_pos = (vec2f(pos) + 0.5) / vec2f(textureDimensions(output_texture));
    let pt = 1.0 / vec2f(input_dims);

    // Sample luma values for gradient computation
    let l = textureSampleLevel(luma_texture, common_sampler, uv_pos + vec2f(-1, 0) * pt, 0.0).r;
    let c = textureSampleLevel(luma_texture, common_sampler, uv_pos, 0.0).r;
    let r = textureSampleLevel(luma_texture, common_sampler, uv_pos + vec2f(1, 0) * pt, 0.0).r;

    // Horizontal Gradient: [-1 0 1] / [-2 0 2] / [-1 0 1]
    let xgrad = -l + r;

    // Vertical Gradient: [-1 -2 -1] / [0 0 0] / [1 2 1]
    let ygrad = l + c + c + r;

    textureStore(output_texture, pos, vec4f(xgrad, ygrad, 0.0, 1.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let output_dims = textureDimensions(output_texture);
    let coord = vec2i(global_id.xy);
    if coord.x >= i32(output_dims.x) || coord.y >= i32(output_dims.y) {
        return;
    }

    process(coord);
}

@compute @workgroup_size(8, 8)
fn main_unchecked(@builtin(global_invocation_id) global_id: vec3u) {
    process(vec2i(global_id.xy));
}
