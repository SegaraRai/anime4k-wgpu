//! Build script for anime4k-wgpu crate
//!
//! This build script generates Rust code containing pre-compiled Anime4K shader pipelines.
//! It processes GLSL CNN shaders and WGSL manifests to create optimized executable pipelines
//! that are embedded directly into the compiled binary for maximum performance.

use anime4k_wgpu_build::{
    cnn_glsl_to_executable_pipeline,
    pipelines::ExecutablePipeline,
    predefined::{PREDEFINED_PIPELINES_AUX, PREDEFINED_PIPELINES_CNN},
    wgsl_to_executable_pipeline,
};

/// Converts WGSL shader source into a Rust string literal
///
/// Minifies the shader and escapes it for embedding as a string constant in generated Rust code.
fn dump_shader_string_literal(shader: &str) -> String {
    // Escape special characters for Rust string literal
    let escaped_shader = shader.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    format!("\"{escaped_shader}\"")
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
        output.push_str(&format!("        SamplerFilterMode::{sampler:?},\n"));
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
fn dump_cnn_shader_decl(id: &str, glsl_filepath: &str, helpers_dir: &str, minify: bool) -> String {
    let pipeline = cnn_glsl_to_executable_pipeline(glsl_filepath, helpers_dir, minify).expect("Failed to convert CNN GLSL to executable pipeline");
    format!("    pub const {id}: ExecutablePipeline = {};\n", dump_executable_pipeline(id, &pipeline))
}

/// Generates a Rust constant declaration for an auxiliary shader from WGSL manifest
///
/// Converts a WGSL pipeline manifest to an optimized ExecutablePipeline constant.
fn dump_aux_shader_decl(id: &str, wgsl_manifest_filepath: &str, minify: bool) -> String {
    let pipeline = wgsl_to_executable_pipeline(wgsl_manifest_filepath, minify).expect("Failed to convert WGSL to executable pipeline");
    format!("    pub const {id}: ExecutablePipeline = {};\n", dump_executable_pipeline(id, &pipeline))
}

/// Generates the complete pipelines.rs file with all Anime4K shader constants
///
/// Creates both CNN and auxiliary shader modules with pre-compiled pipeline definitions.
fn write_code(minify: bool) {
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
    for (id, filepath) in PREDEFINED_PIPELINES_AUX.iter() {
        println!("Processing auxiliary shader: {id} from {filepath}");
        let decl = dump_aux_shader_decl(id, &format!("{project_dir}/{filepath}"), minify);
        code.push_str(&decl);
    }
    code.push_str("}\n\n");

    // Generate CNN/GAN shader module with all variants organized by category
    code.push_str("pub mod cnn {\n");
    code.push_str("use crate::executable_pipeline::*;\n\n");
    for (id, filepath) in PREDEFINED_PIPELINES_CNN.iter() {
        println!("Processing CNN shader: {id} from {filepath}");
        let decl = dump_cnn_shader_decl(id, &format!("{project_dir}/{filepath}"), &helpers_dir, minify);
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
    let minify = true;
    write_code(minify);
}
