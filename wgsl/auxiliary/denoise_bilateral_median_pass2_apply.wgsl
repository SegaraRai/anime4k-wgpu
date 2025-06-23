// MIT License
// Copyright (c) 2019-2021 bloc97
// Ported to WGSL for anime4k-wgpu

@group(0) @binding(0) var input_texture: texture_2d<f32>; // source texture (HOOKED, 4ch x1)
@group(0) @binding(1) var luma_texture: texture_2d<f32>; // luminance data (from pass 1, 1ch x1)
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>; // bilateral median filtered result (4ch x1)

const INTENSITY_SIGMA: f32 = 0.1;
const SPATIAL_SIGMA: f32 = 1.0;
const HISTOGRAM_REGULARIZATION: f32 = 0.0;
const INTENSITY_POWER_CURVE: f32 = 1.0;

const KERNEL_SIZE: i32 = i32(max(SPATIAL_SIGMA, 1.0)) * 2 + 1;
const KERNEL_HALF_SIZE: i32 = KERNEL_SIZE / 2;
const KERNEL_LEN: i32 = KERNEL_SIZE * KERNEL_SIZE;

fn get_offset(i: i32) -> vec2i {
    return vec2i((i % KERNEL_SIZE) - KERNEL_HALF_SIZE, (i / KERNEL_SIZE) - KERNEL_HALF_SIZE);
}

fn gaussian(x: f32, s: f32, m: f32) -> f32 {
    let scaled = (x - m) / s;
    return exp(-0.5 * scaled * scaled);
}

fn get_median(values: array<vec4f, KERNEL_LEN>, luma_values: array<f32, KERNEL_LEN>, weights: array<f32, KERNEL_LEN>, n: f32) -> vec4f {
    for (var i: i32 = 0; i < KERNEL_LEN; i++) {
        var w_above = 0.0;
        var w_below = 0.0;
        for (var j: i32 = 0; j < KERNEL_LEN; j++) {
            if luma_values[j] > luma_values[i] {
                w_above += weights[j];
            } else if luma_values[j] < luma_values[i] {
                w_below += weights[j];
            }
        }

        if (n - w_above) / n >= 0.5 && w_below / n <= 0.5 {
            return values[i];
        }
    }
    return vec4f(0.0);
}

fn process(pos: vec2i) {
    let vc = textureLoad(luma_texture, pos, 0).r;

    let is = pow(vc + 0.0001, INTENSITY_POWER_CURVE) * INTENSITY_SIGMA;
    let ss = SPATIAL_SIGMA;

    let bound = vec2i(textureDimensions(input_texture)) - 1;

    var histogram_v: array<vec4f, KERNEL_LEN>;
    var histogram_l: array<f32, KERNEL_LEN>;
    var histogram_w: array<f32, KERNEL_LEN>;
    var n: f32 = 0.0;
    for (var i: i32 = 0; i < KERNEL_LEN; i++) {
        let ipos = get_offset(i);
        let sample_pos = clamp(pos + ipos, vec2i(0), bound);

        histogram_v[i] = textureLoad(input_texture, sample_pos, 0);
        histogram_l[i] = textureLoad(luma_texture, sample_pos, 0).r;
        histogram_w[i] = gaussian(histogram_l[i], is, vc) * gaussian(length(vec2f(ipos)), ss, 0.0);
        n += histogram_w[i];
    }

    if HISTOGRAM_REGULARIZATION > 0.0 {
        var histogram_wn: array<f32, KERNEL_LEN>;
        var n: f32 = 0.0;

        for (var i: i32 = 0; i < KERNEL_LEN; i++) {
            histogram_wn[i] = 0.0;
        }

        for (var i: i32 = 0; i < KERNEL_LEN; i++) {
            histogram_wn[i] += gaussian(0.0, HISTOGRAM_REGULARIZATION, 0.0) * histogram_w[i];
            for (var j: i32 = (i + 1); j < KERNEL_LEN; j++) {
                var d = gaussian(histogram_l[j], HISTOGRAM_REGULARIZATION, histogram_l[i]);
                histogram_wn[j] += d * histogram_w[i];
                histogram_wn[i] += d * histogram_w[j];
            }
            n += histogram_wn[i];
        }

        let result = get_median(histogram_v, histogram_l, histogram_wn, n);
        textureStore(output_texture, pos, result);
    } else {
        let result = get_median(histogram_v, histogram_l, histogram_w, n);
        textureStore(output_texture, pos, result);
    }
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
