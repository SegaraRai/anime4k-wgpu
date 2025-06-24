// sRGB to screen shader with scaling
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct ScaleUniforms {
    scale: vec2<f32>,
    offset: vec2<f32>,
}

@group(0) @binding(1) var<uniform> scale_uniforms: ScaleUniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Apply scaling for `object-fit: contain` behavior
    var scaled_position = input.position.xy * scale_uniforms.scale + scale_uniforms.offset;
    output.position = vec4(scaled_position, input.position.z, 1.0);
    output.tex_coords = input.tex_coords;

    return output;
}

@group(0) @binding(0) var rgb_texture: texture_2d<f32>;
@group(0) @binding(2) var input_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(rgb_texture, input_sampler, input.tex_coords, 0.0);
}
