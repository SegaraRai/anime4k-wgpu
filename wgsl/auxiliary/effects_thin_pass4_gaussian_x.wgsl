// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var sobel_texture: texture_2d<f32>; // sobel gradient magnitude (from pass 3, 1ch HQ=x1, F=x0.5, VF=x0.25)
@group(0) @binding(1) var output_texture: texture_storage_2d<r32float, write>; // X-direction gaussian blur (1ch HQ=x1, F=x0.5, VF=x0.25)

fn gaussian(x: f32, s: f32, m: f32) -> f32 {
    let scaled = (x - m) / s;
    return exp(-0.5 * scaled * scaled);
}

fn process(pos: vec2i) {
    let sobel_dims = textureDimensions(sobel_texture);
    let spatial_sigma = f32(sobel_dims.y) * 2.0 / 1080.0;
    let bound = vec2i(sobel_dims) - 1;

    let kernel_size = max(i32(ceil(spatial_sigma * 2.0)), 1) * 2 + 1;
    let kernel_half_size = kernel_size / 2;

    var g = 0.0;
    var gn = 0.0;

    for (var i: i32 = 0; i < kernel_size; i++) {
        let di = i - kernel_half_size;
        let gf = gaussian(f32(di), spatial_sigma, 0.0);
        let sample_pos = clamp(pos + vec2i(di, 0), vec2i(0), bound);

        g += textureLoad(sobel_texture, sample_pos, 0).r * gf;
        gn += gf;
    }

    let result = g / gn;
    textureStore(output_texture, pos, vec4f(result, 0.0, 0.0, 1.0));
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
