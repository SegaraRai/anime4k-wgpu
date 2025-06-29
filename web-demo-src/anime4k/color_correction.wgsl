@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba32float, write>;

struct ColorCorrectionUniforms {
    // Matrix selection and range flags
    source_matrix: u32,        // 0=BT.601, 1=BT.709, 2=BT.2020
    target_matrix: u32,        // 0=BT.601, 1=BT.709, 2=BT.2020
    source_range: u32,         // 0=TV range, 1=Full range
    target_range: u32,         // 0=TV range, 1=Full range

    // Transfer function flags
    source_transfer: u32,      // 0=Linear, 1=sRGB, 2=Rec.709, 3=Gamma2.2
    target_transfer: u32,      // 0=Linear, 1=sRGB, 2=Rec.709, 3=Gamma2.2

    // Additional correction flags
    enable_correction: u32,    // 0=passthrough, 1=apply correction
    reserved: u32,             // Reserved for future use
}

@group(0) @binding(2) var<uniform> uniforms: ColorCorrectionUniforms;

// sRGB to Linear
fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(
        pow((color + 0.055) / 1.055, vec3<f32>(2.4)),
        color / 12.92,
        color <= vec3<f32>(0.04045)
    );
}

// Linear to sRGB
fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return select(
        pow(color, vec3<f32>(1.0 / 2.4)) * 1.055 - 0.055,
        color * 12.92,
        color <= vec3<f32>(0.0031308)
    );
}

// Rec.709 to Linear
fn rec709_to_linear(color: vec3<f32>) -> vec3<f32> {
    let alpha = 1.09929682680944;
    let beta = 0.018053968510807;
    return select(
        pow((color + alpha - 1.0) / alpha, vec3<f32>(1.0 / 0.45)),
        color / 4.5,
        color < vec3<f32>(beta)
    );
}

// Linear to Rec.709
fn linear_to_rec709(linear: vec3<f32>) -> vec3<f32> {
    let alpha = 1.09929682680944;
    let beta = 0.018053968510807;
    return select(
        alpha * pow(linear, vec3<f32>(0.45)) - (alpha - 1.0),
        4.5 * linear,
        linear < vec3<f32>(beta / 4.5)
    );
}

// Simple gamma correction
fn gamma_to_linear(color: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(color, vec3<f32>(gamma));
}

fn linear_to_gamma(linear: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(linear, vec3<f32>(1.0 / gamma));
}

// Apply transfer function (to linear)
fn apply_source_transfer(color: vec3<f32>) -> vec3<f32> {
    switch uniforms.source_transfer {
    case 0u: { return color; }                          // Already linear
    case 1u: { return srgb_to_linear(color); }           // sRGB
    case 2u: { return rec709_to_linear(color); }         // Rec.709
    case 3u: { return gamma_to_linear(color, 2.2); }     // Gamma 2.2
    default: { return color; }
  }
}

// Apply target transfer function (from linear)
fn apply_target_transfer(linear: vec3<f32>) -> vec3<f32> {
    switch uniforms.target_transfer {
    case 0u: { return linear; }                         // Stay linear
    case 1u: { return linear_to_srgb(linear); }          // sRGB
    case 2u: { return linear_to_rec709(linear); }        // Rec.709
    case 3u: { return linear_to_gamma(linear, 2.2); }    // Gamma 2.2
    default: { return linear; }
  }
}

// RGB to YUV conversion matrices
fn rgb_to_yuv_bt601(rgb: vec3<f32>) -> vec3<f32> {
    let y = 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
    let u = -0.14713 * rgb.r - 0.28886 * rgb.g + 0.436 * rgb.b;
    let v = 0.615 * rgb.r - 0.51499 * rgb.g - 0.10001 * rgb.b;
    return vec3<f32>(y, u, v);
}

fn rgb_to_yuv_bt709(rgb: vec3<f32>) -> vec3<f32> {
    let y = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
    let u = -0.09991 * rgb.r - 0.33609 * rgb.g + 0.436 * rgb.b;
    let v = 0.615 * rgb.r - 0.55861 * rgb.g - 0.05639 * rgb.b;
    return vec3<f32>(y, u, v);
}

fn rgb_to_yuv_bt2020(rgb: vec3<f32>) -> vec3<f32> {
    let y = 0.2627 * rgb.r + 0.6780 * rgb.g + 0.0593 * rgb.b;
    let u = -0.13963 * rgb.r - 0.36037 * rgb.g + 0.5 * rgb.b;
    let v = 0.5 * rgb.r - 0.45979 * rgb.g - 0.04021 * rgb.b;
    return vec3<f32>(y, u, v);
}

// YUV to RGB conversion matrices
fn yuv_to_rgb_bt601(yuv: vec3<f32>) -> vec3<f32> {
    let r = yuv.x + 1.13983 * yuv.z;
    let g = yuv.x - 0.39465 * yuv.y - 0.58060 * yuv.z;
    let b = yuv.x + 2.03211 * yuv.y;
    return vec3<f32>(r, g, b);
}

fn yuv_to_rgb_bt709(yuv: vec3<f32>) -> vec3<f32> {
    let r = yuv.x + 1.28033 * yuv.z;
    let g = yuv.x - 0.21482 * yuv.y - 0.38059 * yuv.z;
    let b = yuv.x + 2.12798 * yuv.y;
    return vec3<f32>(r, g, b);
}

fn yuv_to_rgb_bt2020(yuv: vec3<f32>) -> vec3<f32> {
    let r = yuv.x + 1.7166 * yuv.z;
    let g = yuv.x - 0.18874 * yuv.y - 0.65025 * yuv.z;
    let b = yuv.x + 2.1418 * yuv.y;
    return vec3<f32>(r, g, b);
}

// Convert RGB to YUV using specified matrix
fn rgb_to_yuv(rgb: vec3<f32>, matrix: u32) -> vec3<f32> {
    switch matrix {
    case 0u: { return rgb_to_yuv_bt601(rgb); }
    case 1u: { return rgb_to_yuv_bt709(rgb); }
    case 2u: { return rgb_to_yuv_bt2020(rgb); }
    default: { return rgb_to_yuv_bt709(rgb); }
  }
}

// Convert YUV to RGB using specified matrix
fn yuv_to_rgb(yuv: vec3<f32>, matrix: u32) -> vec3<f32> {
    switch matrix {
    case 0u: { return yuv_to_rgb_bt601(yuv); }
    case 1u: { return yuv_to_rgb_bt709(yuv); }
    case 2u: { return yuv_to_rgb_bt2020(yuv); }
    default: { return yuv_to_rgb_bt709(yuv); }
  }
}

// Range conversion functions
fn tv_range_to_full_range(color: vec3<f32>) -> vec3<f32> {
    // Y: 16-235 -> 0-255, UV: 16-240 -> 0-255
    let expanded = (color - vec3<f32>(16.0 / 255.0)) / vec3<f32>((235.0 - 16.0) / 255.0, (240.0 - 16.0) / 255.0, (240.0 - 16.0) / 255.0);
    return clamp(expanded, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn full_range_to_tv_range(color: vec3<f32>) -> vec3<f32> {
    // 0-255 -> Y: 16-235, UV: 16-240
    let compressed = color * vec3<f32>((235.0 - 16.0) / 255.0, (240.0 - 16.0) / 255.0, (240.0 - 16.0) / 255.0) + vec3<f32>(16.0 / 255.0);
    return clamp(compressed, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_range_conversion(color: vec3<f32>, from_range: u32, to_range: u32) -> vec3<f32> {
    if from_range == to_range {
        return color;
    }

    if from_range == 0u && to_range == 1u {
        return tv_range_to_full_range(color);
    } else if from_range == 1u && to_range == 0u {
        return full_range_to_tv_range(color);
    }

    return color;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tex_size = textureDimensions(input_texture);
    if global_id.x >= tex_size.x || global_id.y >= tex_size.y {
        return;
    }

    let input_color = textureLoad(input_texture, vec2<i32>(global_id.xy), 0);

    // Passthrough if correction is disabled
    if uniforms.enable_correction == 0u {
        textureStore(output_texture, vec2<i32>(global_id.xy), input_color);
        return;
    }

    var color = input_color.rgb;

    // Step 1: Convert from source transfer function to linear
    color = apply_source_transfer(color);

    // Step 2: Convert RGB to YUV using source matrix (reverse browser's incorrect conversion)
    var yuv = rgb_to_yuv(color, uniforms.source_matrix);

    // Step 3: Apply range conversion in YUV space
    yuv = apply_range_conversion(yuv, uniforms.source_range, uniforms.target_range);

    // Step 4: Convert YUV back to RGB using target matrix (correct conversion)
    color = yuv_to_rgb(yuv, uniforms.target_matrix);

    // Step 5: Apply target transfer function
    color = apply_target_transfer(color);

    // Clamp to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(color, input_color.a));
}
