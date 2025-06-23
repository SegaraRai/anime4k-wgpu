// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba32float, write>; // bilateral mean filtered result (4ch x1)

const INTENSITY_SIGMA: f32 = 0.1;
const SPATIAL_SIGMA: f32 = 1.0;
const INTENSITY_POWER_CURVE: f32 = 1.0;

const KERNEL_SIZE: i32 = i32(max(ceil(SPATIAL_SIGMA * 2.0), 1.0)) * 2 + 1;
const KERNEL_HALF_SIZE: i32 = KERNEL_SIZE / 2;
const KERNEL_LEN: i32 = KERNEL_SIZE * KERNEL_SIZE;

fn get_offset(i: i32) -> vec2i {
    return vec2i((i % KERNEL_SIZE) - KERNEL_HALF_SIZE, (i / KERNEL_SIZE) - KERNEL_HALF_SIZE);
}

fn gaussian_vec(x: vec4f, s: vec4f, m: vec4f) -> vec4f {
    let scaled = (x - m) / s;
    return exp(-0.5 * scaled * scaled);
}

fn gaussian(x: f32, s: f32, m: f32) -> f32 {
    let scaled = (x - m) / s;
    return exp(-0.5 * scaled * scaled);
}

fn process(pos: vec2i) {
    let vc = textureLoad(input_texture, pos, 0);

    let is = pow(vc + vec4f(0.0001), vec4f(INTENSITY_POWER_CURVE)) * INTENSITY_SIGMA;
    let ss = SPATIAL_SIGMA;

    let bound = vec2i(textureDimensions(input_texture)) - 1;

    var sum = vec4f(0.0);
    var n = vec4f(0.0);
    for (var i: i32 = 0; i < KERNEL_LEN; i++) {
        let ipos = get_offset(i);
        let sample_pos = clamp(pos + ipos, vec2i(0), bound);
        let v = textureLoad(input_texture, sample_pos, 0);
        let d = gaussian_vec(v, is, vc) * gaussian(length(vec2f(ipos)), ss, 0.0);
        sum += d * v;
        n += d;
    }

    let result = sum / n;
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
