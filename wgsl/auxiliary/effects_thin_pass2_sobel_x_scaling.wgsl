// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch F=x1, VF=x0.5)
@group(0) @binding(1) var luma_sampler: sampler; // sampler for luma_texture
@group(0) @binding(2) var output_texture: texture_storage_2d<rg32float, write>; // X-direction sobel gradient (2ch F=x0.5, VF=x0.25)

fn process(pos: vec2i) {
    let dims = vec2f(textureDimensions(output_texture));
    let luma_dims = vec2f(textureDimensions(luma_texture));
    let uv_pos = (vec2f(pos) + 0.5) / dims;
    let texel_size = 1.0 / luma_dims;

    let l = textureSampleLevel(luma_texture, luma_sampler, uv_pos + vec2f(-texel_size.x, 0.0), 0.0).r;
    let c = textureSampleLevel(luma_texture, luma_sampler, uv_pos, 0.0).r;
    let r = textureSampleLevel(luma_texture, luma_sampler, uv_pos + vec2f(texel_size.x, 0.0), 0.0).r;

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
