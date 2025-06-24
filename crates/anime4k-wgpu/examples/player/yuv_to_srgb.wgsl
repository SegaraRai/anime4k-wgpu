// YUV to sRGB conversion compute shader (BT.709)

@group(0) @binding(0) var y_texture: texture_2d<f32>;
@group(0) @binding(1) var uv_texture: texture_2d<f32>;
@group(0) @binding(2) var uv_sampler: sampler;
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba32float, write>;

fn process(pos: vec2i) {
    let output_dims = textureDimensions(output_texture);
    let uv_pos = (vec2f(pos) + 0.5) / vec2f(output_dims);

    let y = textureLoad(y_texture, pos, 0).r;

    let uv = textureSampleLevel(uv_texture, uv_sampler, uv_pos, 0.0).rg;
    let u = uv.x - 0.5; // Center U around 0
    let v = uv.y - 0.5; // Center V around 0

    // BT.709 YUV to RGB conversion (assuming full range input)
    let r = y + 1.5748 * v;
    let g = y - 0.1873 * u - 0.4681 * v;
    let b = y + 1.8556 * u;

    textureStore(output_texture, pos, vec4f(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), 1.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let dims = textureDimensions(output_texture);
    if global_id.x >= dims.x || global_id.y >= dims.y {
        return;
    }

    process(vec2i(global_id.xy));
}
