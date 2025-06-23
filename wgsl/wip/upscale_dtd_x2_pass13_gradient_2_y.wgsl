// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Kernel-Y (Second Gradients)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var lumad2_texture: texture_2d<f32>; // second gradient x (LUMAD2, 2ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rg32float, write>; // final second gradient output (LUMAD2, 2ch x1)

fn process(pos: vec2i) {
    let dims = textureDimensions(lumad2_texture);

    // Sample neighbors with boundary clamping
    let t_pos = clamp(pos + vec2i(0, -1), vec2i(0), vec2i(dims) - vec2i(1));
    let c_pos = pos;
    let b_pos = clamp(pos + vec2i(0, 1), vec2i(0), vec2i(dims) - vec2i(1));

    let tx = textureLoad(lumad2_texture, t_pos, 0).x;
    let cx = textureLoad(lumad2_texture, c_pos, 0).x;
    let bx = textureLoad(lumad2_texture, b_pos, 0).x;

    let ty = textureLoad(lumad2_texture, t_pos, 0).y;
    let by = textureLoad(lumad2_texture, b_pos, 0).y;

    // Horizontal Gradient
    // [-1  0  1]
    // [-2  0  2]
    // [-1  0  1]
    let xgrad = (tx + cx + cx + bx) / 8.0;

    // Vertical Gradient
    // [-1 -2 -1]
    // [ 0  0  0]
    // [ 1  2  1]
    let ygrad = (-ty + by) / 8.0;

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
