// Display shader with proper UV coordinates

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate a full-screen triangle with proper UV coordinates
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>(3.0, -1.0),  // Bottom-right (extended)
        vec2<f32>(-1.0, 3.0)   // Top-left (extended)
    );

    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),   // Bottom-left
        vec2<f32>(2.0, 1.0),   // Bottom-right (extended)
        vec2<f32>(0.0, -1.0)   // Top-left (extended)
    );

    out.clip_position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.tex_coords = uv[vertex_index];

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample and display the texture content
    return textureSample(input_texture, texture_sampler, in.tex_coords);
}
