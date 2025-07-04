//! Build script for the verification crate
//!
//! This build script processes Anime4K GLSL shaders and converts them to WGSL
//! format for verification testing. It generates pipeline manifests and creates
//! a Rust module with embedded shader code for testing purposes.

use anime4k_wgpu_build::cnn::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Creates a YAML pipeline manifest for a converted shader
///
/// Generates a pipeline manifest that describes the shader passes, their inputs,
/// outputs, and resource bindings. This manifest is used by the verification
/// engine to execute the shader pipeline correctly.
///
/// # Arguments
/// * `shader_name` - Base name for the shader (used as ID and name)
/// * `passes` - Vector of (filename, shader) pairs representing each pass
///
/// # Returns
/// YAML manifest content as a string
fn create_manifest(shader_name: &str, passes: &[(String, WgslStageShader)]) -> String {
    let mut manifest = String::new();

    // Pipeline header with identification
    manifest.push_str(&format!("id: {shader_name}\n"));
    manifest.push_str(&format!("name: {shader_name}\n"));
    manifest.push_str(&format!("description: {shader_name}\n"));
    manifest.push_str("passes:\n");

    // Generate pass entries for each shader stage
    for (filename, wgsl_shader) in passes {
        // Format scale factor as array string for YAML
        let str_scale_factor = format!("[\"{}\", \"{}\"]", &wgsl_shader.scale_factor, &wgsl_shader.scale_factor);

        // Pass identification and shader file reference
        manifest.push_str(&format!("  - id: {}\n", wgsl_shader.name));
        manifest.push_str(&format!("    file: {filename}\n"));

        // Input texture bindings
        manifest.push_str("    inputs:\n");
        for (binding, id) in &wgsl_shader.inputs {
            manifest.push_str(&format!("      - id: {id}\n"));
            manifest.push_str(&format!("        binding: {binding}\n"));
        }

        // Output texture binding (always RGBA with 4 components)
        manifest.push_str("    outputs:\n");
        manifest.push_str(&format!("      - id: {}\n", wgsl_shader.output.1));
        manifest.push_str(&format!("        binding: {}\n", &wgsl_shader.output.0));
        manifest.push_str("        components: 4\n");
        manifest.push_str(&format!("        scale_factor: {}\n", &str_scale_factor));

        // Optional sampler binding if the shader requires texture sampling
        if let Some(binding) = &wgsl_shader.sampler {
            manifest.push_str("    samplers:\n");
            manifest.push_str(&format!("      - binding: {binding}\n"));
        }
    }

    manifest
}

/// Converts a GLSL CNN/GAN shader to WGSL and generates a pipeline manifest
///
/// Takes a GLSL shader file containing mpv hooks, splits it into individual passes,
/// converts each pass to WGSL format, and creates a pipeline manifest describing
/// the complete multi-pass shader pipeline.
///
/// # Arguments
/// * `glsl_path` - Path to the source GLSL shader file
/// * `output_dir` - Directory where to write the converted WGSL files and manifest
///
/// # Returns
/// Tuple of (manifest filename, vector of created WGSL shader filenames)
fn convert_cnn_shader(glsl_path: &Path, output_dir: &Path) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    // Read the source GLSL shader file
    let source = fs::read_to_string(glsl_path)?;
    let shader_name = glsl_path.file_stem().unwrap().to_str().unwrap();

    // Split the GLSL source into individual mpv hook passes
    let pass_sources = MpvHook::parse_mpv_hooks(&source);

    // Initialize conversion state
    let mut pass_counter = 1;
    let mut passes = Vec::new();
    let mut created_shader_filenames = Vec::new();
    let mut scale_factor_map = MpvHook::new_scale_factor_map();

    // Process each mpv hook pass in the shader
    for pass_source in pass_sources {
        // Parse the mpv hook using the existing parser from anime4k_wgpu_build
        let hook = MpvHook::new(&pass_source, &mut scale_factor_map)?;

        // Convert the parsed hook to WGSL format
        let wgsl_shader = WgslStageShader::new(hook, &scale_factor_map)?;

        // Generate unique filename for this pass
        let pass_name = format!("{shader_name}_{pass_counter}");
        let wgsl_filename = format!("{pass_name}.wgsl");
        let wgsl_path = output_dir.join(&wgsl_filename);

        // Write WGSL shader file if it's a convolution pass
        if let WgslStageShaderType::Conv { code } = &wgsl_shader.r#type {
            fs::write(&wgsl_path, code)?;
            println!("Generated: {}", wgsl_path.display());
            created_shader_filenames.push(wgsl_filename.clone());
            passes.push((wgsl_filename, wgsl_shader));
        } else {
            // For non-convolution passes, reference existing helper shaders
            println!("Skipping non-conv pass: {wgsl_filename}");
            passes.push((format!("depth_to_space_in{}x{}.wgsl", wgsl_shader.inputs.len() - 1, wgsl_shader.scale_factor), wgsl_shader));
        }

        pass_counter += 1;
    }

    // Generate pipeline manifest describing all passes
    let manifest_content = create_manifest(shader_name, &passes);
    let manifest_path = output_dir.join(format!("{shader_name}_manifest.yaml"));
    fs::write(&manifest_path, manifest_content)?;
    println!("Generated: {}", manifest_path.display());

    Ok((manifest_path.file_name().unwrap().to_str().unwrap().to_string(), created_shader_filenames))
}

/// Build script main function
///
/// Processes all Anime4K CNN/GAN GLSL shaders found in the project directory,
/// converts them to WGSL format, generates pipeline manifests, and creates
/// a Rust module with embedded shader code for verification testing.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine project structure and output directories
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_dir = crate_dir.parent().unwrap().parent().unwrap();
    let build_output_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Source directories for GLSL shaders and WGSL helpers
    let anime4k_glsl_dir = project_dir.join("anime4k-glsl");
    let helpers_dir = project_dir.join("wgsl/helpers");
    let output_dir = build_output_dir.join("converted_cnns");

    // Prepare clean output directory for converted shaders
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    // Discover all CNN/GAN GLSL shader files in the anime4k-glsl directory
    let mut cnn_files = Vec::new();

    for entry in fs::read_dir(anime4k_glsl_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let subdir = entry.path();
            // Search subdirectories for shader files
            for subentry in fs::read_dir(&subdir)? {
                let subentry = subentry?;
                let path = subentry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    // Filter for CNN, GAN, and 3DGraphics shaders
                    if filename.contains("_CNN_") || filename.contains("_GAN_") || filename.contains("_3DGraphics_") {
                        cnn_files.push(path);
                    }
                }
            }
        }
    }

    // Track all shader files and converted items for Rust module generation
    let mut wgsl_shader_filenames = Vec::new();
    let mut converted_items = Vec::new();

    // Process each discovered CNN/GAN shader file
    for glsl_path in cnn_files {
        // Add file to cargo dependency tracking for rebuild detection
        println!("cargo::rerun-if-changed={}", glsl_path.display());

        let glsl_filename = glsl_path.file_name().unwrap();
        // Convert GLSL to WGSL and create pipeline manifest
        let (manifest_filename, created_shaders) = convert_cnn_shader(&glsl_path, &output_dir)?;
        // Copy original GLSL file for reference
        fs::copy(&glsl_path, output_dir.join(glsl_filename))?;

        wgsl_shader_filenames.extend(created_shaders);
        converted_items.push((manifest_filename, glsl_filename.to_str().unwrap().to_string()));
    }

    // Copy WGSL helper files needed by the converted shaders
    for entry in fs::read_dir(helpers_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("wgsl") {
            // Add helper files to cargo dependency tracking
            println!("cargo::rerun-if-changed={}", path.display());

            let filename = path.file_name().unwrap();
            fs::copy(&path, output_dir.join(filename))?;

            wgsl_shader_filenames.push(filename.to_str().unwrap().to_string());
        }
    }

    // Generate Rust module with embedded shader code for verification testing
    let mut cnns_rs = String::new();
    cnns_rs.push_str("// This file is auto-generated by build.rs\n\n");
    cnns_rs.push_str("pub mod cnns {\n");
    cnns_rs.push_str("    use std::collections::HashMap;\n\n");

    // Create constant array of (manifest_filename, manifest_content, glsl_content) tuples
    cnns_rs.push_str("    pub const CNN_ITEMS: &[(&str, &str, &str)] = &[\n");
    for (manifest, glsl_shader) in converted_items {
        cnns_rs.push_str(&format!(
            "        (\"{manifest}\", include_str!(concat!(env!(\"OUT_DIR\"), \"/converted_cnns/{manifest}\")), include_str!(concat!(env!(\"OUT_DIR\"), \"/converted_cnns/{glsl_shader}\"))),\n",
        ));
    }
    cnns_rs.push_str("    ];\n\n");

    // Create function that returns HashMap of shader filename -> shader content
    cnns_rs.push_str("    pub fn get_shader_map() -> HashMap<&'static str, &'static str> {\n");
    cnns_rs.push_str("        let mut map = HashMap::new();\n");
    for shader in wgsl_shader_filenames {
        cnns_rs.push_str(&format!("        map.insert(\"{shader}\", include_str!(concat!(env!(\"OUT_DIR\"), \"/converted_cnns/{shader}\")));\n",));
    }
    cnns_rs.push_str("        map\n");
    cnns_rs.push_str("    }\n");
    cnns_rs.push_str("}\n");

    // Write the generated Rust module to the output directory
    let cnns_rs_path = output_dir.join("cnns.rs");
    fs::write(&cnns_rs_path, cnns_rs)?;

    Ok(())
}
