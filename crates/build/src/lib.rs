//! Anime4K build utilities
//!
//! This crate provides tools for converting Anime4K shaders from various formats
//! into optimized ExecutablePipeline structures. It handles both CNN/GAN shaders
//! from GLSL and auxiliary shaders from WGSL manifests.

mod minify;

pub mod cnn;
pub mod pipelines;
pub mod predefined;

pub use minify::minify_wgsl;

/// Converts a CNN/GAN GLSL shader file to an optimized ExecutablePipeline
///
/// This function processes mpv-style GLSL hooks used in original Anime4K implementations
/// and converts them to WGSL compute shaders with optimized resource allocation.
///
/// # Arguments
/// * `glsl_filepath` - Path to the GLSL shader file containing mpv hooks
/// * `helpers_dir` - Directory containing WGSL helper functions
/// * `minify` - Whether to minify the WGSL code
///
/// # Returns
/// An ExecutablePipeline ready for GPU execution
pub fn cnn_glsl_to_executable_pipeline(glsl_filepath: &str, helpers_dir: &str, minify: bool) -> Result<pipelines::ExecutablePipeline, std::boxed::Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    let mpv_hook_source = std::fs::read_to_string(glsl_filepath)?;
    let pass_sources = cnn::MpvHook::parse_mpv_hooks(&mpv_hook_source);

    let mut files = HashMap::new();
    let mut passes = Vec::new();
    let mut scale_factor_map = cnn::MpvHook::new_scale_factor_map();
    for (pass_index, pass_source) in pass_sources.iter().enumerate() {
        // Parse the pass source to create a WGSL shader
        let hook = cnn::MpvHook::new(pass_source, &mut scale_factor_map)?;
        let wgsl_shader = cnn::WgslStageShader::new(hook, &scale_factor_map)?;

        // Generate the filename and code for the WGSL shader
        let (filename, code) = if let cnn::WgslStageShaderType::Conv { code } = &wgsl_shader.r#type {
            let filename = format!("pass_{pass_index}.wgsl");
            (filename, code.clone())
        } else {
            let filename = format!("depth_to_space_in{}x{}.wgsl", wgsl_shader.inputs.len() - 1, wgsl_shader.scale_factor);
            let code = std::fs::read_to_string(format!("{helpers_dir}/{filename}"))?;
            (filename, code)
        };

        // Minify the WGSL code if requested
        let code = if minify { minify_wgsl(&code)? } else { code };

        // Insert the WGSL code into the files map
        files.insert(filename.clone(), code);

        // Create the pass specification
        passes.push(pipelines::Pass {
            id: format!("Pass {}", pass_index + 1),
            file: filename,
            inputs: wgsl_shader
                .inputs
                .iter()
                .map(|(binding, id)| pipelines::TextureBindingSpec { binding: *binding, id: id.clone() })
                .collect(),
            outputs: vec![pipelines::TextureOutput {
                binding: wgsl_shader.output.0,
                id: wgsl_shader.output.1.clone(),
                components: 4, // Always 4 components for CNNs
                scale_factor: [
                    pipelines::ScaleFactor {
                        numerator: wgsl_shader.scale_factor.parse().unwrap(),
                        denominator: 1,
                    },
                    pipelines::ScaleFactor {
                        numerator: wgsl_shader.scale_factor.parse().unwrap(),
                        denominator: 1,
                    },
                ],
            }],
            samplers: wgsl_shader.sampler.map_or(vec![], |binding| {
                vec![pipelines::SamplerBinding {
                    binding,
                    filter_mode: pipelines::SamplerFilterMode::Linear,
                }]
            }),
        });
    }

    let spec = pipelines::PipelineSpec {
        id: "anime4k_cnn".to_string(),
        name: "Anime4K CNN".to_string(),
        description: None,
        passes,
    };

    let pipeline = pipelines::ExecutablePipeline::from_raw(spec, |filename: &str| {
        files
            .get(filename)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("File not found: {}", filename)))
    })?;

    Ok(pipeline)
}

/// Converts a WGSL manifest file to an optimized ExecutablePipeline
///
/// Loads a YAML pipeline manifest and compiles it into an ExecutablePipeline
/// with all resources pre-allocated and optimized.
///
/// # Arguments
/// * `wgsl_manifest_filepath` - Path to the YAML manifest file
/// * `minify` - Whether to minify the WGSL code
///
/// # Returns
/// An ExecutablePipeline ready for GPU execution
pub fn wgsl_to_executable_pipeline(wgsl_manifest_filepath: &str, minify: bool) -> Result<pipelines::ExecutablePipeline, std::boxed::Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(wgsl_manifest_filepath).parent().unwrap();
    pipelines::ExecutablePipeline::from_file(wgsl_manifest_filepath, |filename: &str| {
        let path = dir.join(filename);
        let code = std::fs::read_to_string(&path).inspect_err(|e| {
            eprintln!("Error reading file {path:?}: {e}");
        })?;
        let code = if minify {
            minify_wgsl(&code).map_err(|e| {
                eprintln!("Error minifying WGSL code in file {path:?}: {e}");
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to minify WGSL code in file {path:?}"))
            })?
        } else {
            code
        };
        Ok(code)
    })
}
