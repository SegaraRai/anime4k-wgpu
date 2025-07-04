//! ExecutablePipeline compilation and optimization
//!
//! This module implements the core pipeline compilation system that converts
//! human-readable pipeline specifications into GPU-optimized ExecutablePipeline
//! structures with pre-allocated resources and optimal memory layouts.

use super::{PhysicalTexture, PipelineSpec, SamplerBinding, SamplerFilterMode, ScaleFactor, TextureLifetime, physical_texture::assign_physical_textures};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// A complete analyzed pipeline manifest ready for execution
///
/// This structure represents a fully compiled and optimized shader pipeline
/// with all resources pre-allocated and shader code embedded for maximum performance.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutablePipeline {
    /// Unique identifier for this pipeline
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Physical textures with optimized allocation
    pub physical_textures: Vec<PhysicalTexture>,
    /// Executable passes with resolved bindings
    pub passes: Vec<ExecutablePass>,
    /// Required sampler filter modes
    pub required_samplers: Vec<SamplerFilterMode>,
}

/// A single shader pass within an executable pipeline
///
/// Contains all the information needed to execute one stage of the pipeline,
/// including compiled shader code and optimized resource bindings.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutablePass {
    /// Unique identifier for this pass
    pub id: String,
    /// WGSL shader source code
    pub shader: String,
    /// Compute dispatch scale factors (width, height)
    pub compute_scale_factors: (f64, f64),
    /// Input texture bindings
    pub input_textures: Vec<PhysicalTextureBinding>,
    /// Output texture bindings
    pub output_textures: Vec<PhysicalTextureBinding>,
    /// Sampler bindings
    pub samplers: Vec<SamplerBinding>,
}

/// Binding of a physical texture in an executable pass
#[derive(Debug, Clone, Serialize)]
pub struct PhysicalTextureBinding {
    /// Original logical texture identifier
    pub logical_id: String,
    /// Assigned physical texture ID
    pub physical_id: u32,
    /// Shader binding point
    pub binding: u32,
    /// Number of color components
    pub components: u32,
    /// Scale factor relative to input
    pub scale_factor: (ScaleFactor, ScaleFactor),
}

impl ExecutablePipeline {
    /// Creates an ExecutablePipeline from a raw pipeline specification
    ///
    /// Compiles and optimizes the pipeline specification into an executable form.
    ///
    /// # Arguments
    /// * `raw` - The pipeline specification to compile
    /// * `load_shader_file` - Function to load shader source files
    ///
    /// # Returns
    /// An optimized ExecutablePipeline ready for GPU execution
    pub fn from_raw(raw: PipelineSpec, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<Self, std::io::Error> {
        raw.compile(load_shader_file)
    }

    /// Creates an ExecutablePipeline from YAML content
    ///
    /// Parses the YAML specification and compiles it into an executable pipeline.
    ///
    /// # Arguments
    /// * `yaml_content` - YAML pipeline specification content
    /// * `load_shader_file` - Function to load shader source files
    ///
    /// # Returns
    /// An optimized ExecutablePipeline ready for GPU execution
    pub fn from_yaml(yaml_content: &str, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = PipelineSpec::from_yaml(yaml_content)?;
        Ok(raw.compile(load_shader_file)?)
    }

    /// Creates an ExecutablePipeline from a YAML file
    ///
    /// Loads and parses a YAML pipeline manifest file, then compiles it into an executable pipeline.
    ///
    /// # Arguments
    /// * `path` - Path to the YAML manifest file
    /// * `load_shader_file` - Function to load shader source files
    ///
    /// # Returns
    /// An optimized ExecutablePipeline ready for GPU execution
    pub fn from_file<P: AsRef<std::path::Path>>(path: P, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = PipelineSpec::from_file(path)?;
        Ok(raw.compile(load_shader_file)?)
    }

    /// Gets the physical texture ID of the source texture
    ///
    /// The source texture is the input image that the pipeline processes.
    /// It's marked with `is_source: true` in the physical texture list.
    ///
    /// # Returns
    /// The physical texture ID of the source texture, or None if not found
    pub fn get_source_texture_id(&self) -> Option<u32> {
        self.physical_textures.iter().find(|tex| tex.is_source).map(|tex| tex.id)
    }

    /// Gets the physical texture ID of the final result texture
    ///
    /// The result texture is the final output of the pipeline, identified by
    /// the logical ID "RESULT" in the last pass of the pipeline.
    ///
    /// # Returns
    /// The physical texture ID of the result texture, or None if not found
    pub fn get_result_texture_id(&self) -> Option<u32> {
        self.passes
            .last()
            .and_then(|pass| pass.output_textures.iter().find(|output| output.logical_id == "RESULT"))
            .map(|output| output.physical_id)
    }

    /// Gets the final scale factor of the pipeline output
    ///
    /// This represents how much the final output is scaled relative to the
    /// input image. For example, (2x, 2x) means the output is twice the
    /// resolution in both dimensions.
    ///
    /// # Returns
    /// The scale factors (width, height) of the final result, or None if not found
    pub fn get_final_scale_factor(&self) -> Option<(ScaleFactor, ScaleFactor)> {
        self.passes
            .last()
            .and_then(|pass| pass.output_textures.iter().find(|output| output.logical_id == "RESULT"))
            .map(|output| output.scale_factor)
    }
}

impl PipelineSpec {
    /// Compiles this pipeline specification into an executable pipeline
    ///
    /// Performs texture lifetime analysis, resource optimization, and shader compilation
    /// to create a GPU-ready ExecutablePipeline.
    ///
    /// # Arguments
    /// * `load_shader_file` - Function to load shader source files
    ///
    /// # Returns
    /// An optimized ExecutablePipeline ready for GPU execution
    pub fn compile(self, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<ExecutablePipeline, std::io::Error> {
        let compiler = PipelineCompiler::new(self);
        compiler.compile(load_shader_file)
    }

    /// Validates the pipeline specification for correctness
    ///
    /// Checks for common errors like missing IDs, empty passes, and invalid texture references.
    ///
    /// # Returns
    /// Ok(()) if valid, or a specific validation error
    pub fn validate(&self) -> Result<(), PipelineValidationError> {
        if self.id.is_empty() {
            return Err(PipelineValidationError::EmptyId);
        }

        if self.name.is_empty() {
            return Err(PipelineValidationError::EmptyName);
        }

        if self.passes.is_empty() {
            return Err(PipelineValidationError::NoPasses);
        }

        // Check that each pass has at least one input and one output
        for (i, pass) in self.passes.iter().enumerate() {
            if pass.inputs.is_empty() {
                return Err(PipelineValidationError::PassMissingInputs(i));
            }
            if pass.outputs.is_empty() {
                return Err(PipelineValidationError::PassMissingOutputs(i));
            }
        }

        // Check binding uniqueness within each pass
        for (i, pass) in self.passes.iter().enumerate() {
            let mut used_bindings = std::collections::HashSet::new();

            for input in &pass.inputs {
                if !used_bindings.insert(input.binding) {
                    return Err(PipelineValidationError::DuplicateBinding(i, input.binding));
                }
            }

            for output in &pass.outputs {
                if !used_bindings.insert(output.binding) {
                    return Err(PipelineValidationError::DuplicateBinding(i, output.binding));
                }
            }

            for sampler in &pass.samplers {
                if !used_bindings.insert(sampler.binding) {
                    return Err(PipelineValidationError::DuplicateBinding(i, sampler.binding));
                }
            }
        }

        // Check that RESULT output is only in the last pass
        for (i, pass) in self.passes.iter().enumerate() {
            for output in &pass.outputs {
                if output.id == "RESULT" && i != self.passes.len() - 1 {
                    return Err(PipelineValidationError::ResultNotInLastPass(i));
                }
            }
        }

        // Check that textures are not overwritten
        let mut created_textures = HashSet::new();
        created_textures.insert("SOURCE".to_string()); // SOURCE always exists

        for (i, pass) in self.passes.iter().enumerate() {
            // Check outputs don't overwrite existing textures
            for output in &pass.outputs {
                if created_textures.contains(&output.id) {
                    return Err(PipelineValidationError::TextureOverwritten(i, output.id.clone()));
                }
                created_textures.insert(output.id.clone());
            }
        }

        // Check that input textures exist
        let mut available_textures = HashSet::new();
        available_textures.insert("SOURCE".to_string());

        for (i, pass) in self.passes.iter().enumerate() {
            // Check all inputs are available
            for input in &pass.inputs {
                if !available_textures.contains(&input.id) {
                    return Err(PipelineValidationError::InputTextureNotFound(i, input.id.clone()));
                }
            }

            // Add outputs to available textures for next passes
            for output in &pass.outputs {
                available_textures.insert(output.id.clone());
            }
        }

        Ok(())
    }
}

/// Internal compiler for converting pipeline specifications to executable pipelines
///
/// Handles the complex process of texture lifetime analysis, resource allocation,
/// and shader compilation with optimization.
struct PipelineCompiler {
    /// The raw pipeline specification to compile
    raw: PipelineSpec,
}

impl PipelineCompiler {
    /// Creates a new pipeline compiler for the given specification
    fn new(raw: PipelineSpec) -> Self {
        Self { raw }
    }

    /// Compiles the pipeline specification into an optimized executable pipeline
    ///
    /// This performs the core compilation work including texture lifetime analysis,
    /// physical resource allocation, and shader loading with optimization.
    fn compile(self, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<ExecutablePipeline, std::io::Error> {
        let texture_lifetimes = self.collect_texture_lifetimes();
        let (physical_textures, texture_assignments) = assign_physical_textures(&texture_lifetimes);
        let shader_passes = self.create_executable_passes(&texture_assignments, load_shader_file)?;

        let mut required_samplers = Vec::new();
        for pass in &self.raw.passes {
            for sampler in &pass.samplers {
                if !required_samplers.contains(&sampler.filter_mode) {
                    required_samplers.push(sampler.filter_mode);
                }
            }
        }

        Ok(ExecutablePipeline {
            id: self.raw.id,
            name: self.raw.name,
            description: self.raw.description,
            physical_textures,
            passes: shader_passes,
            required_samplers,
        })
    }

    /// Analyzes texture usage patterns to determine resource lifetimes
    ///
    /// This function tracks when each texture is created and when it's last used
    /// to enable optimal memory allocation and texture reuse.
    fn collect_texture_lifetimes(&self) -> Vec<TextureLifetime> {
        let mut texture_lifetimes = Vec::new();

        // Collect all texture lifetimes
        for (pass_idx, pass) in self.raw.passes.iter().enumerate() {
            for output in &pass.outputs {
                if output.id == "SOURCE" {
                    continue; // Skip SOURCE as it's always available
                }

                // Find when this texture is last used
                let mut last_used_at = pass_idx;
                for (later_pass_idx, later_pass) in self.raw.passes.iter().enumerate().skip(pass_idx + 1) {
                    for input in &later_pass.inputs {
                        if input.id == output.id {
                            last_used_at = later_pass_idx;
                        }
                    }
                }

                texture_lifetimes.push(TextureLifetime {
                    logical_id: output.id.clone(),
                    components: output.components,
                    scale_factor: (output.scale_factor[0], output.scale_factor[1]),
                    created_at: pass_idx,
                    last_used_at,
                });
            }
        }

        // Sort by creation time for processing
        texture_lifetimes.sort_by_key(|t| t.created_at);
        texture_lifetimes
    }

    /// Creates executable passes with optimized resource bindings
    ///
    /// Converts raw pass specifications into executable passes with physical texture
    /// assignments and loaded shader code.
    fn create_executable_passes(&self, texture_assignments: &HashMap<String, u32>, load_shader_file: impl Fn(&str) -> Result<String, std::io::Error>) -> Result<Vec<ExecutablePass>, std::io::Error> {
        self.raw
            .passes
            .iter()
            .map(|pass| -> Result<ExecutablePass, std::io::Error> {
                let input_textures = pass
                    .inputs
                    .iter()
                    .map(|input| {
                        let physical_id = texture_assignments[&input.id];
                        let physical_texture = self.find_physical_texture_info(&input.id);

                        PhysicalTextureBinding {
                            logical_id: input.id.clone(),
                            physical_id,
                            binding: input.binding,
                            components: physical_texture.0,
                            scale_factor: physical_texture.1,
                        }
                    })
                    .collect();

                let output_textures = pass
                    .outputs
                    .iter()
                    .map(|output| {
                        let physical_id = texture_assignments[&output.id];

                        PhysicalTextureBinding {
                            logical_id: output.id.clone(),
                            physical_id,
                            binding: output.binding,
                            components: output.components,
                            scale_factor: (output.scale_factor[0], output.scale_factor[1]),
                        }
                    })
                    .collect();

                let samplers = pass.samplers.clone();

                let first_output = pass.outputs.first().unwrap();
                let compute_scale_factors = (first_output.scale_factor[0].to_f64(), first_output.scale_factor[1].to_f64());

                Ok(ExecutablePass {
                    id: pass.id.clone(),
                    shader: load_shader_file(&pass.file)?,
                    compute_scale_factors,
                    input_textures,
                    output_textures,
                    samplers,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    /// Finds physical texture information for a logical texture ID
    ///
    /// This method looks up the component count and scale factors for a given
    /// logical texture ID by searching through the pass outputs where it was defined.
    /// Special handling is provided for the SOURCE texture.
    ///
    /// # Arguments
    /// * `logical_id` - The logical texture identifier to look up
    ///
    /// # Returns
    /// A tuple containing (component_count, (width_scale, height_scale))
    fn find_physical_texture_info(&self, logical_id: &str) -> (u32, (ScaleFactor, ScaleFactor)) {
        if logical_id == "SOURCE" {
            return (4, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)));
        }

        // Find the output definition for this logical texture
        for pass in &self.raw.passes {
            for output in &pass.outputs {
                if output.id == logical_id {
                    return (output.components, (output.scale_factor[0], output.scale_factor[1]));
                }
            }
        }

        // Fallback
        (4, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)))
    }
}

/// Errors that can occur during pipeline validation
///
/// These errors indicate problems with the pipeline specification that
/// prevent successful compilation or execution.
#[derive(Debug, Clone)]
pub enum PipelineValidationError {
    /// Pipeline ID field is empty
    EmptyId,
    /// Pipeline name field is empty
    EmptyName,
    /// Pipeline contains no shader passes
    NoPasses,
    /// A shader pass has no input textures (pass index)
    PassMissingInputs(usize),
    /// A shader pass has no output textures (pass index)
    PassMissingOutputs(usize),
    /// Two or more bindings in the same pass use the same binding point (pass index, binding)
    DuplicateBinding(usize, u32),
    /// RESULT output found in a pass other than the last one (pass index)
    ResultNotInLastPass(usize),
    /// A texture is being overwritten by multiple passes (pass index, texture ID)
    TextureOverwritten(usize, String),
    /// An input texture was not created by any previous pass (pass index, texture ID)
    InputTextureNotFound(usize, String),
}

impl fmt::Display for PipelineValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "Pipeline ID cannot be empty"),
            Self::EmptyName => write!(f, "Pipeline name cannot be empty"),
            Self::NoPasses => write!(f, "Pipeline must have at least one pass"),
            Self::PassMissingInputs(pass) => write!(f, "Pass {pass} is missing inputs"),
            Self::PassMissingOutputs(pass) => write!(f, "Pass {pass} is missing outputs"),
            Self::DuplicateBinding(pass, binding) => {
                write!(f, "Duplicate binding {binding} in pass {pass}")
            }
            Self::ResultNotInLastPass(pass) => {
                write!(f, "RESULT output found in pass {pass} but must only be in the last pass")
            }
            Self::TextureOverwritten(pass, texture) => {
                write!(f, "Texture '{texture}' is being overwritten in pass {pass}")
            }
            Self::InputTextureNotFound(pass, texture) => {
                write!(f, "Input texture '{texture}' in pass {pass} was not created by any previous pass or is not SOURCE")
            }
        }
    }
}

impl std::error::Error for PipelineValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic executable pipeline creation from YAML specification
    #[test]
    fn test_executable_pipeline_creation() {
        let yaml = r#"
id: test_pipeline
name: Test Pipeline
description: A test pipeline
passes:
  - id: pass1
    file: pass1.wgsl
    inputs:
      - id: SOURCE
        binding: 0
    outputs:
      - id: RESULT
        binding: 1
        components: 4
        scale_factor: ["2", "2"]
"#;

        let load_shader_file = |file: &str| -> Result<String, std::io::Error> {
            // Simulate loading shader file content
            Ok(format!("Shader content for {file}"))
        };
        let executable = ExecutablePipeline::from_yaml(yaml, load_shader_file).unwrap();
        assert_eq!(executable.id, "test_pipeline");
        assert_eq!(executable.name, "Test Pipeline");
        assert_eq!(executable.passes.len(), 1);

        // Should have SOURCE texture (u32::MAX) and one regular texture (0)
        assert_eq!(executable.physical_textures.len(), 2);

        let source_texture = executable.physical_textures.iter().find(|t| t.is_source).unwrap();
        assert_eq!(source_texture.id, u32::MAX);

        let result_texture = executable.physical_textures.iter().find(|t| !t.is_source).unwrap();
        assert_eq!(result_texture.components, 4);
        assert_eq!(result_texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
    }

    /// Tests validation of a correctly structured pipeline spec
    ///
    /// Verifies that a valid pipeline passes all validation checks
    /// without errors.
    #[test]
    fn test_validation() {
        let yaml = r#"
id: test_pipeline
name: Test Pipeline
description: A test pipeline
passes:
  - id: pass1
    file: pass1.wgsl
    inputs:
      - id: SOURCE
        binding: 0
    outputs:
      - id: RESULT
        binding: 1
        components: 4
        scale_factor: ["2", "2"]
"#;

        let raw = PipelineSpec::from_yaml(yaml).unwrap();
        assert!(raw.validate().is_ok());
    }
}
