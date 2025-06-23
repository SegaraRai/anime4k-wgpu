// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var lumad_texture: texture_2d<f32>; // refined gradient (LUMAD from pass 3, 2ch x2)
@group(0) @binding(1) var output_texture: texture_storage_2d<rg32float, write>; // directional gradient (LUMAMM, 2ch x2)

fn process(pos: vec2i) {
    let dval = textureLoad(lumad_texture, pos, 0).y;
    if dval < 0.1 {
        textureStore(output_texture, pos, vec4f());
        return;
    }

    let dims = vec2i(textureDimensions(lumad_texture));

    // Sample gradient values for secondary kernel with bounds checking
    let l = textureLoad(lumad_texture, max(pos + vec2i(-1, 0), vec2i(0)), 0).x;
    let c = textureLoad(lumad_texture, pos, 0).x;
    let r = textureLoad(lumad_texture, min(pos + vec2i(1, 0), dims - 1), 0).x;

    // Horizontal Gradient
    let xgrad = -l + r;
    // Vertical Gradient
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
