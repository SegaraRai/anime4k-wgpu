// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu
// Anime4K-v3.2-Upscale-DTD-x2-Kernel-Y (Gradients)

@group(0) @binding(0) var input_texture: texture_2d<f32>; // darkened texture (HOOKED, 4ch x1)
@group(0) @binding(1) var lumad_texture: texture_2d<f32>; // gradient texture (LUMAD, 2ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<r32float, write>; // gradient magnitude output (LUMAD, 1ch x1)

fn process(pos: vec2i) {
    let dims = textureDimensions(lumad_texture);

    // Sample neighbors with boundary clamping
    let t_pos = clamp(pos + vec2i(0, -1), vec2i(0), vec2i(dims) - vec2i(1));
    let c_pos = pos;
    let b_pos = clamp(pos + vec2i(0, 1), vec2i(0), vec2i(dims) - vec2i(1));

    let tx = textureLoad(lumad_texture, t_pos, 0).x;
    let cx = textureLoad(lumad_texture, c_pos, 0).x;
    let bx = textureLoad(lumad_texture, b_pos, 0).x;

    let ty = textureLoad(lumad_texture, t_pos, 0).y;
    let by = textureLoad(lumad_texture, b_pos, 0).y;

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

    // Computes the luminance's gradient magnitude
    let norm = sqrt(xgrad * xgrad + ygrad * ygrad);
    let result = pow(norm, 0.7);

    textureStore(output_texture, pos, vec4f(result, 0.0, 0.0, 1.0));
}

@compute @workgrosup_size(8, 8)
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
