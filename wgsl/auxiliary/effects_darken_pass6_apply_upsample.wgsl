// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var kernel_texture: texture_2d<f32>; // gaussian kernel result (LINEKERNEL from pass 5, 1ch F=x0.5, VF=x0.25)
@group(0) @binding(2) var kernel_sampler: sampler; // sampler for kernel texture
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba32float, write>; // darkened output (4ch x1)

const STRENGTH: f32 = 1.5;

fn process(pos: vec2i) {
    let output_dims = textureDimensions(output_texture);
    let uv_pos = (vec2f(pos) + 0.5) / vec2f(output_dims);

    let hooked_color = textureLoad(input_texture, pos, 0);
    let kernel_val = textureSampleLevel(kernel_texture, kernel_sampler, uv_pos, 0.0).r;

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let result = hooked_color + kernel_val * STRENGTH;

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
