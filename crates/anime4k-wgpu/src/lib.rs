//! Anime4K-wgpu implementation for real-time anime upscaling
//!
//! This crate provides a wgpu-based implementation of the Anime4K algorithm,
//! designed for real-time upscaling of anime and cartoon content. It supports
//! various quality presets and performance levels to balance quality and speed.

pub(crate) mod executable_pipeline;
mod pipeline_executor;

pub mod pipelines;
pub mod presets;

pub use executable_pipeline::ExecutablePipeline;
pub use pipeline_executor::PipelineExecutor;
