// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var lumad_texture: texture_2d<f32>; // gradient magnitude and refine (from pass 3, 2ch x2)
@group(0) @binding(1) var lumamm_texture: texture_2d<f32>; // secondary X-direction gradient (from pass 4, 2ch x2)
@group(0) @binding(2) var output_texture: texture_storage_2d<rg32float, write>; // normalized directional gradient (2ch x2)

fn process(pos: vec2i) {
    let dval = textureLoad(lumad_texture, pos, 0).y;
    if dval < 0.1 {
        textureStore(output_texture, pos, vec4f());
        return;
    }

    let bound = vec2i(textureDimensions(lumamm_texture)) - 1;

    // Sample from the previous kernel pass with bounds checking
    let tx = textureLoad(lumamm_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).r;
    let cx = textureLoad(lumamm_texture, pos, 0).r;
    let bx = textureLoad(lumamm_texture, min(pos + vec2i(0, 1), bound), 0).r;

    let ty = textureLoad(lumamm_texture, max(pos + vec2i(0, -1), vec2i(0)), 0).g;
    let by = textureLoad(lumamm_texture, min(pos + vec2i(0, 1), bound), 0).g;

    // Horizontal Gradient
    let xgrad = tx + cx + cx + bx;
    // Vertical Gradient
    let ygrad = -ty + by;

    let norm = sqrt(xgrad * xgrad + ygrad * ygrad);
    var final_xgrad = xgrad;
    var final_ygrad = ygrad;
    var final_norm = norm;

    if norm <= 0.001 {
        final_xgrad = 0.0;
        final_ygrad = 0.0;
        final_norm = 1.0;
    }

    textureStore(output_texture, pos, vec4f(final_xgrad / final_norm, final_ygrad / final_norm, 0.0, 1.0));
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
