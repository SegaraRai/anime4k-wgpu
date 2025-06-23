//! Executable pipeline definitions and runtime structures
//!
//! This module defines the core data structures used for GPU-optimized shader pipeline execution.
//! ExecutablePipeline represents a fully compiled and optimized shader pipeline with pre-allocated
//! resources, embedded shader code, and optimized texture binding layouts.

/// Represents a rational scale factor as a fraction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleFactor {
    /// The numerator of the scale factor fraction
    pub numerator: u32,
    /// The denominator of the scale factor fraction
    pub denominator: u32,
}

/// Texture sampling filter modes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SamplerFilterMode {
    /// Nearest neighbor sampling - sharp, pixelated
    #[allow(unused)]
    Nearest,
    /// Linear interpolation sampling - smooth, blurred
    Linear,
}

/// A complete analyzed pipeline manifest ready for execution
///
/// This structure contains all the information needed to execute a shader pipeline
/// on the GPU, with optimized resource allocation and embedded shader code.
#[derive(Debug, Clone)]
pub struct ExecutablePipeline {
    /// Human-readable name for debugging
    pub(crate) name: &'static str,
    /// Physical textures used by this pipeline
    pub(crate) textures: &'static [PhysicalTexture],
    /// Sampler filter modes required by this pipeline
    pub(crate) samplers: &'static [SamplerFilterMode],
    /// Shader passes to execute in sequence
    pub(crate) passes: &'static [ExecutablePass],
}

/// Represents a physical texture resource in the GPU
#[derive(Debug, Clone)]
pub struct PhysicalTexture {
    /// Unique identifier for this texture
    pub id: u32,
    /// Number of color components (1=R, 2=RG, 4=RGBA)
    pub components: u32,
    /// Scale factors for width and height relative to input
    pub scale_factor: (ScaleFactor, ScaleFactor),
    /// Whether this texture represents the source input
    pub is_source: bool,
}

/// A single shader pass within a pipeline
#[derive(Debug, Clone)]
pub struct ExecutablePass {
    /// Human-readable name for debugging
    pub name: &'static str,
    /// WGSL shader source code
    pub shader: &'static str,
    /// Compute dispatch scale factors (width, height)
    pub compute_scale_factors: (f64, f64),
    /// Input texture bindings for this pass
    pub input_textures: &'static [InputTextureBinding],
    /// Output texture bindings for this pass
    pub output_textures: &'static [OutputTextureBinding],
    /// Sampler bindings for this pass
    pub samplers: &'static [SamplerBinding],
}

/// Binding information for an input texture
#[derive(Debug, Clone)]
pub struct InputTextureBinding {
    /// Shader binding point index
    pub binding: u32,
    /// ID of the physical texture to bind
    pub physical_texture_id: u32,
}

/// Binding information for an output texture
#[derive(Debug, Clone)]
pub struct OutputTextureBinding {
    /// Shader binding point index
    pub binding: u32,
    /// ID of the physical texture to bind
    pub physical_texture_id: u32,
}

/// Binding information for a texture sampler
#[derive(Debug, Clone)]
pub struct SamplerBinding {
    /// Shader binding point index
    pub binding: u32,
    /// Filter mode for this sampler
    pub filter_mode: SamplerFilterMode,
}
