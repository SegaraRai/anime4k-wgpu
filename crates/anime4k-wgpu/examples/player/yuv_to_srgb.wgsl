// YUV to sRGB conversion shader (BT.709)
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4(input.position, 1.0);
    output.tex_coords = input.tex_coords;
    return output;
}

@group(0) @binding(0) var y_texture: texture_2d<f32>;
@group(0) @binding(1) var uv_texture: texture_2d<f32>;
@group(0) @binding(2) var input_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var y = textureSample(y_texture, input_sampler, input.tex_coords).r;
    var uv = textureSample(uv_texture, input_sampler, input.tex_coords).rg;
    var u = uv.x - 0.5; // Center U around 0
    var v = uv.y - 0.5; // Center V around 0

    // BT.709 YUV to RGB conversion (assuming full range input)
    let r = y + 1.5748 * v;
    let g = y - 0.1873 * u - 0.4681 * v;
    let b = y + 1.8556 * u;

    // Clamp to valid range
    let r_clamped = clamp(r, 0.0, 1.0);
    let g_clamped = clamp(g, 0.0, 1.0);
    let b_clamped = clamp(b, 0.0, 1.0);

    return vec4<f32>(r_clamped, g_clamped, b_clamped, 1.0);
}
