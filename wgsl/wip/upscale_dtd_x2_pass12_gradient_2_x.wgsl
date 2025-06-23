// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Kernel-X (Second Gradients)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var lumad_texture: texture_2d<f32>; // final gradient (LUMAD, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rg32float, write>; // second gradient output (LUMAD2, 2ch x1)

fn process(pos: vec2i) {
    let dims = textureDimensions(lumad_texture);

    // Sample neighbors with boundary clamping
    let l_pos = clamp(pos + vec2i(-1, 0), vec2i(0), vec2i(dims) - vec2i(1));
    let c_pos = pos;
    let r_pos = clamp(pos + vec2i(1, 0), vec2i(0), vec2i(dims) - vec2i(1));

    let l = textureLoad(lumad_texture, l_pos, 0).x;
    let c = textureLoad(lumad_texture, c_pos, 0).x;
    let r = textureLoad(lumad_texture, r_pos, 0).x;

    // Horizontal Gradient
    // [-1  0  1]
    // [-2  0  2]
    // [-1  0  1]
    let xgrad = (-l + r);

    // Vertical Gradient
    // [-1 -2 -1]
    // [ 0  0  0]
    // [ 1  2  1]
    let ygrad = (l + c + c + r);

    // Computes the luminance's gradient
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
