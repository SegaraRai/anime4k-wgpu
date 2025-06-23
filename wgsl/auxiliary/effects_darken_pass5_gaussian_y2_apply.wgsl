// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var kernel_texture: texture_2d<f32>; // secondary X-direction gaussian blur (from pass 4, 1ch HQ=x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // darkened output (4ch x1)

const STRENGTH: f32 = 1.5;

fn gaussian(x: f32, s: f32, m: f32) -> f32 {
    let scaled = (x - m) / s;
    return exp(-0.5 * scaled * scaled);
}

fn process(pos: vec2i) {
    let kernel_dims = textureDimensions(kernel_texture);
    let spatial_sigma = f32(kernel_dims.y) / 1080.0;
    let bound = vec2i(kernel_dims) - 1;

    let kernel_size = max(i32(ceil(spatial_sigma * 2.0)), 1) * 2 + 1;
    let kernel_half_size = kernel_size / 2;

    var g = 0.0;
    var gn = 0.0;
    for (var i: i32 = 0; i < kernel_size; i++) {
        let di = i - kernel_half_size;
        let gf = gaussian(f32(di), spatial_sigma, 0.0);
        let sample_pos = clamp(pos + vec2i(0, di), vec2i(0), bound);

        g += textureLoad(kernel_texture, sample_pos, 0).r * gf;
        gn += gf;
    }

    let kernel_val = g / gn;
    let hooked_color = textureLoad(input_texture, pos, 0);

    // This trick is only possible if the inverse Y->RGB matrix has 1 for every row (which is the case for BT.709)
    // Otherwise we would need to convert RGB to YUV, modify Y then convert back to RGB.
    let result = hooked_color + vec4f(kernel_val * STRENGTH);
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
