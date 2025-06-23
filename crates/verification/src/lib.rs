//! Verification utilities for Anime4K-wgpu
//!
//! This crate provides tools for verifying the correctness of the wgpu-based
//! Anime4K implementation against reference implementations.

pub mod compare;
pub mod glsl_reference_engine;
mod wgpu_helpers;
pub mod wgsl_reference_engine;
