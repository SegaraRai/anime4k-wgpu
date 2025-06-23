//! CNN shader conversion utilities
//!
//! This module provides tools for converting GLSL CNN shaders from the original
//! Anime4K implementation to WGSL format suitable for wgpu execution.

mod convert;

pub use convert::*;
