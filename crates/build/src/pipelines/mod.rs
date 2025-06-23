//! Pipeline compilation and optimization system
//!
//! This module provides the core functionality for converting human-readable
//! pipeline specifications into optimized, GPU-ready executable pipelines.
//! It handles resource allocation, texture lifetime analysis, and GPU resource binding.

mod executable_pipeline;
mod physical_texture;
mod pipeline_specs;

pub use executable_pipeline::*;
pub use physical_texture::{PhysicalTexture, TextureLifetime};
pub use pipeline_specs::*;
