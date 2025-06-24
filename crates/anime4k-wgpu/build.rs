//! Build script for anime4k-wgpu crate
//!
//! This build script generates Rust code containing pre-compiled Anime4K shader pipelines.
//! It processes GLSL CNN shaders and WGSL manifests to create optimized executable pipelines
//! that are embedded directly into the compiled binary for maximum performance.

use anime4k_wgpu_build::{cnn_glsl_to_executable_pipeline, pipelines::ExecutablePipeline, wgsl_to_executable_pipeline};

/// Minifies WGSL shader source code to reduce binary size
///
/// Uses naga to parse, validate, and regenerate the WGSL code in a more compact form.
/// This reduces the size of embedded shaders without affecting functionality.
fn minify_wgsl(shader: &str) -> String {
    let mut module = naga::front::wgsl::parse_str(shader).expect("Failed to parse WGSL shader");

    wgsl_minifier::minify_module(&mut module);

    let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all());
    let info = validator.validate(&module).unwrap();
    let output = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty()).unwrap();

    wgsl_minifier::minify_wgsl_source(&output)
}

/// Converts WGSL shader source into a Rust string literal
///
/// Minifies the shader and escapes it for embedding as a string constant in generated Rust code.
fn dump_shader_string_literal(shader: &str) -> String {
    // Escape special characters for Rust string literal
    let escaped_shader = minify_wgsl(shader).replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    format!("\"{}\"", escaped_shader)
}

/// Generates Rust code for an ExecutablePipeline constant
///
/// Converts an analyzed pipeline into Rust source code that recreates the pipeline
/// structure with all optimizations and resource allocations pre-computed.
fn dump_executable_pipeline(name: &str, pipeline: &ExecutablePipeline) -> String {
    let mut output = String::new();
    output.push_str("ExecutablePipeline {\n");
    output.push_str(&format!("    name: \"Anime4K {name}\",\n"));

    // Generate physical texture definitions
    output.push_str("    textures: &[\n");
    for texture in &pipeline.physical_textures {
        output.push_str("        PhysicalTexture {\n");
        output.push_str(&format!("            id: {},\n", texture.id));
        output.push_str(&format!("            components: {},\n", texture.components));
        output.push_str("            scale_factor: (\n");
        output.push_str(&format!(
            "                ScaleFactor {{ numerator: {}, denominator: {} }},\n",
            texture.scale_factor.0.numerator, texture.scale_factor.0.denominator
        ));
        output.push_str(&format!(
            "                ScaleFactor {{ numerator: {}, denominator: {} }},\n",
            texture.scale_factor.1.numerator, texture.scale_factor.1.denominator
        ));
        output.push_str("            ),\n");
        output.push_str(&format!("            is_source: {},\n", texture.is_source));
        output.push_str("        },\n");
    }
    output.push_str("    ],\n");

    // Generate required sampler definitions
    output.push_str("    samplers: &[\n");
    for sampler in &pipeline.required_samplers {
        output.push_str(&format!("        SamplerFilterMode::{:?},\n", sampler));
    }
    output.push_str("    ],\n");

    // Generate shader pass definitions
    output.push_str("    passes: &[\n");
    for pass in &pipeline.passes {
        output.push_str("        ExecutablePass {\n");
        output.push_str(&format!("            name: \"Anime4K {name} {}\",\n", pass.id));
        output.push_str(&format!("            shader: {},\n", dump_shader_string_literal(&pass.shader)));
        output.push_str(&format!(
            "            compute_scale_factors: ({:.2}, {:.2}),\n",
            pass.compute_scale_factors.0, pass.compute_scale_factors.1
        ));

        // Generate input texture bindings
        output.push_str("            input_textures: &[\n");
        for input in &pass.input_textures {
            output.push_str("                InputTextureBinding {\n");
            output.push_str(&format!("                    binding: {},\n", input.binding));
            output.push_str(&format!("                    physical_texture_id: {},\n", input.physical_id));
            output.push_str("                },\n");
        }
        output.push_str("            ],\n");

        // Generate output texture bindings
        output.push_str("            output_textures: &[\n");
        for output_texture in &pass.output_textures {
            output.push_str("                OutputTextureBinding {\n");
            output.push_str(&format!("                    binding: {},\n", output_texture.binding));
            output.push_str(&format!("                    physical_texture_id: {},\n", output_texture.physical_id));
            output.push_str("                },\n");
        }
        output.push_str("            ],\n");

        // Generate sampler bindings
        output.push_str("            samplers: &[\n");
        for sampler in &pass.samplers {
            output.push_str("                SamplerBinding {\n");
            output.push_str(&format!("                    binding: {},\n", sampler.binding));
            output.push_str(&format!("                    filter_mode: SamplerFilterMode::{:?},\n", sampler.filter_mode));
            output.push_str("                },\n");
        }
        output.push_str("            ],\n");
        output.push_str("        },\n");
    }
    output.push_str("    ],\n");
    output.push('}');

    output
}

/// Generates a Rust constant declaration for a CNN shader from GLSL
///
/// Converts a GLSL CNN/GAN shader file to an optimized ExecutablePipeline constant.
fn dump_cnn_shader_decl(id: &str, glsl_filepath: &str, helpers_dir: &str) -> String {
    let pipeline = cnn_glsl_to_executable_pipeline(glsl_filepath, helpers_dir).expect("Failed to convert CNN GLSL to executable pipeline");
    format!("    pub const {id}: ExecutablePipeline = {};\n", dump_executable_pipeline(id, &pipeline))
}

/// Generates a Rust constant declaration for an auxiliary shader from WGSL manifest
///
/// Converts a WGSL pipeline manifest to an optimized ExecutablePipeline constant.
fn dump_aux_shader_decl(id: &str, wgsl_manifest_filepath: &str) -> String {
    let pipeline = wgsl_to_executable_pipeline(wgsl_manifest_filepath).expect("Failed to convert WGSL to executable pipeline");
    format!("    pub const {id}: ExecutablePipeline = {};\n", dump_executable_pipeline(id, &pipeline))
}

/// Generates the complete pipelines.rs file with all Anime4K shader constants
///
/// Creates both CNN and auxiliary shader modules with pre-compiled pipeline definitions.
fn write_code() {
    // Determine project directory structure
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .parent()
        .expect("Failed to get grandparent directory")
        .to_str()
        .expect("Failed to convert path to string")
        .to_string();
    let helpers_dir = format!("{project_dir}/wgsl/helpers");

    let mut code = String::new();

    // File header
    code.push_str("// This file is generated by the build script.\n\n");

    // Generate auxiliary shader module with utility and experimental shaders
    code.push_str("pub mod aux {\n");
    code.push_str("use crate::executable_pipeline::*;\n\n");
    for (id, filepath) in [
        // Image processing utilities
        ("CLAMP_HIGHLIGHTS", "wgsl/auxiliary/clamp_highlights_manifest.yaml"),
        // Deblur algorithms
        ("DEBLUR_DOG", "wgsl/auxiliary/deblur_dog_manifest.yaml"),
        ("DEBLUR_ORIGINAL", "wgsl/auxiliary/deblur_original_manifest.yaml"),
        // Denoise algorithms with different statistical approaches
        ("DENOISE_BILATERAL_MEAN", "wgsl/auxiliary/denoise_bilateral_mean_manifest.yaml"),
        ("DENOISE_BILATERAL_MEDIAN", "wgsl/auxiliary/denoise_bilateral_median_manifest.yaml"),
        ("DENOISE_BILATERAL_MODE", "wgsl/auxiliary/denoise_bilateral_mode_manifest.yaml"),
        // Visual effects with different performance profiles
        ("EFFECTS_DARKEN_HQ", "wgsl/auxiliary/effects_darken_manifest_hq.yaml"),
        ("EFFECTS_DARKEN_FAST", "wgsl/auxiliary/effects_darken_manifest_fast.yaml"),
        ("EFFECTS_DARKEN_VERYFAST", "wgsl/auxiliary/effects_darken_manifest_veryfast.yaml"),
        ("EFFECTS_THIN_HQ", "wgsl/auxiliary/effects_thin_manifest_hq.yaml"),
        ("EFFECTS_THIN_FAST", "wgsl/auxiliary/effects_thin_manifest_fast.yaml"),
        ("EFFECTS_THIN_VERYFAST", "wgsl/auxiliary/effects_thin_manifest_veryfast.yaml"),
        // Alternative upscaling algorithms
        ("UPSCALE_DOG_X2", "wgsl/auxiliary/upscale_dog_x2_manifest.yaml"),
        ("UPSCALE_ORIGINAL_X2", "wgsl/auxiliary/upscale_original_x2_manifest.yaml"),
    ] {
        println!("Processing auxiliary shader: {id} from {filepath}");
        let decl = dump_aux_shader_decl(id, &format!("{project_dir}/{filepath}"));
        code.push_str(&decl);
    }
    code.push_str("}\n\n");

    // Generate CNN/GAN shader module with all variants organized by category
    code.push_str("pub mod cnn {\n");
    code.push_str("use crate::executable_pipeline::*;\n\n");
    for (id, filepath) in [
        // Restore variants - improve image quality without upscaling
        ("RESTORE_CNN_S", "anime4k-glsl/Restore/Anime4K_Restore_CNN_S.glsl"),
        ("RESTORE_CNN_M", "anime4k-glsl/Restore/Anime4K_Restore_CNN_M.glsl"),
        ("RESTORE_CNN_L", "anime4k-glsl/Restore/Anime4K_Restore_CNN_L.glsl"),
        ("RESTORE_CNN_VL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_VL.glsl"),
        ("RESTORE_CNN_UL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_UL.glsl"),
        // Restore GAN variants - generative adversarial network restoration
        ("RESTORE_GAN_UL", "anime4k-glsl/Restore/Anime4K_Restore_GAN_UL.glsl"),
        ("RESTORE_GAN_UUL", "anime4k-glsl/Restore/Anime4K_Restore_GAN_UUL.glsl"),
        // Restore Soft variants - gentler restoration algorithms
        ("RESTORE_SOFT_CNN_S", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_S.glsl"),
        ("RESTORE_SOFT_CNN_M", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_M.glsl"),
        ("RESTORE_SOFT_CNN_L", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_L.glsl"),
        ("RESTORE_SOFT_CNN_VL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_VL.glsl"),
        ("RESTORE_SOFT_CNN_UL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_UL.glsl"),
        // Upscale variants - 2x upscaling with different quality levels
        ("UPSCALE_CNN_X2_S", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_S.glsl"),
        ("UPSCALE_CNN_X2_M", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_M.glsl"),
        ("UPSCALE_CNN_X2_L", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_L.glsl"),
        ("UPSCALE_CNN_X2_VL", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_VL.glsl"),
        ("UPSCALE_CNN_X2_UL", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_UL.glsl"),
        // Upscale GAN variants - generative adversarial network upscaling
        ("UPSCALE_GAN_X2_S", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x2_S.glsl"),
        ("UPSCALE_GAN_X2_M", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x2_M.glsl"),
        ("UPSCALE_GAN_X3_L", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x3_L.glsl"),
        ("UPSCALE_GAN_X3_VL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x3_VL.glsl"),
        ("UPSCALE_GAN_X4_UL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x4_UL.glsl"),
        ("UPSCALE_GAN_X4_UUL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x4_UUL.glsl"),
        // Upscale + Denoise variants - combined upscaling and noise reduction
        ("UPSCALE_DENOISE_CNN_X2_S", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_S.glsl"),
        ("UPSCALE_DENOISE_CNN_X2_M", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_M.glsl"),
        ("UPSCALE_DENOISE_CNN_X2_L", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_L.glsl"),
        ("UPSCALE_DENOISE_CNN_X2_VL", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_VL.glsl"),
        ("UPSCALE_DENOISE_CNN_X2_UL", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_UL.glsl"),
        // 3D Graphics variants - specialized for 3D rendered content
        ("UPSCALE_3DCG_CNN_X2_US", "anime4k-glsl/Upscale/Anime4K_3DGraphics_Upscale_x2_US.glsl"),
        ("UPSCALE_3DCG_AA_CNN_X2_US", "anime4k-glsl/Upscale/Anime4K_3DGraphics_AA_Upscale_x2_US.glsl"),
    ] {
        println!("Processing CNN shader: {id} from {filepath}");
        let decl = dump_cnn_shader_decl(id, &format!("{project_dir}/{filepath}"), &helpers_dir);
        code.push_str(&decl);
    }
    code.push_str("}\n\n");

    code.push_str("// END OF GENERATED CODE\n");

    // Write the generated code to the build output directory
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = std::path::PathBuf::from(out_dir).join("pipelines.rs");
    std::fs::write(output_path, code).expect("Failed to write pipelines.rs");
}

/// Build script main function
///
/// Sets up conditional compilation flags and generates all shader pipeline constants.
fn main() {
    // Configure conditional compilation aliases for platform-specific features
    cfg_aliases::cfg_aliases! {
        // Define 'vulkan' cfg for platforms that support Vulkan backend
        vulkan: {
            any(
                windows,  // Windows supports Vulkan
                all(
                    unix,
                    // Unix platforms except macOS, iOS, and WebAssembly support Vulkan
                    not(any(target_os = "macos", target_os = "ios", target_os = "emscripten"))
                )
            )
        },
    }

    // Generate all pipeline constants
    write_code();
}
