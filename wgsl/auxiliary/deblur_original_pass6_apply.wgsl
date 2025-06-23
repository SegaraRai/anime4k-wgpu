// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var input_sampler: sampler; // sampler for input texture
@group(0) @binding(2) var lumad_texture: texture_2d<f32>; // gradient magnitude and refine (from pass 3, 2ch x2)
@group(0) @binding(3) var lumamm_texture: texture_2d<f32>; // normalized directional gradient (from pass 5, 2ch x2)
@group(0) @binding(4) var output_texture: texture_storage_2d<rgba32float, write>; // deblurred result (4ch x2)

fn process(pos: vec2i) {
    let output_dims = textureDimensions(output_texture);
    let input_dims = textureDimensions(input_texture);
    let bound = vec2i(input_dims) - 1;

    let uv_pos = (vec2f(pos) + 0.5) / vec2f(output_dims);
    let hooked_color = textureSampleLevel(input_texture, input_sampler, uv_pos, 0.0);

    let dval = textureLoad(lumad_texture, pos, 0).g;
    if dval < 0.1 {
        textureStore(output_texture, pos, hooked_color);
        return;
    }

    let dc = textureLoad(lumamm_texture, pos, 0);
    if abs(dc.x + dc.y) <= 0.0001 {
        textureStore(output_texture, pos, hooked_color);
        return;
    }

    let xpos = -sign(dc.x);
    let ypos = -sign(dc.y);
    let pt = 1.0 / vec2f(input_dims);

    let xval = textureSampleLevel(input_texture, input_sampler, uv_pos + vec2f(xpos, 0) * pt, 0.0);
    let yval = textureSampleLevel(input_texture, input_sampler, uv_pos + vec2f(0, ypos) * pt, 0.0);

    let xyratio = abs(dc.x) / (abs(dc.x) + abs(dc.y));
    let avg = xyratio * xval + (1.0 - xyratio) * yval;

    let result = avg * dval + hooked_color * (1.0 - dval);
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
