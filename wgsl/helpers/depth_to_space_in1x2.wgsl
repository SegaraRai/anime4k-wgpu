
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var overlay_texture: texture_2d<f32>;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var source_sampler: sampler;

fn process(pos: vec2i) {
    let overlay_pos = vec2i(pos.x / 2, pos.y / 2);
    let overlay_component = (pos.y % 2) * 2 + (pos.x % 2);
    let overlay_scalar = textureLoad(overlay_texture, overlay_pos, 0)[overlay_component];
    let overlay_color = vec4f(overlay_scalar, overlay_scalar, overlay_scalar, 0.0);
    let source_color = textureSampleLevel(source_texture, source_sampler, (vec2f(pos) + vec2f(0.5)) / vec2f(textureDimensions(output_texture)), 0.0);
    textureStore(output_texture, pos, source_color + overlay_color);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) pixel: vec3u) {
    let out_dim: vec2u = textureDimensions(output_texture);
    if pixel.x < out_dim.x && pixel.y < out_dim.y {
        process(vec2i(pixel.xy));
    }
}

@compute @workgroup_size(8, 8)
fn main_unchecked(@builtin(global_invocation_id) pixel: vec3u) {
    process(vec2i(pixel.xy));
}
