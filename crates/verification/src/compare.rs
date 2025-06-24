//! Image comparison utilities for verification
//!
//! This module provides functions for comparing GLSL and WGSL output images
//! to verify implementation correctness.

/// Result of comparing two images
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareResult {
    /// Images match exactly
    Match,
    /// Images have different dimensions
    DimensionMismatch {
        /// Dimensions of the GLSL reference output
        glsl_dimensions: (u32, u32),
        /// Dimensions of the WGSL implementation output
        wgsl_dimensions: (u32, u32),
    },
    /// Images have matching dimensions but different pixel values
    PixelMismatch {
        /// Whether red component values match
        r_matched: bool,
        /// Whether green component values match
        g_matched: bool,
        /// Whether blue component values match
        b_matched: bool,
        /// Whether alpha component values match
        a_matched: bool,
    },
}

/// Compares two RGBA32F images pixel by pixel
///
/// # Arguments
/// * `glsl_output` - Reference image from GLSL implementation
/// * `wgsl_output` - Test image from WGSL implementation
///
/// # Returns
/// A `CompareResult` indicating whether the images match and details about any differences
pub fn compare_images(glsl_output: &image::Rgba32FImage, wgsl_output: &image::Rgba32FImage) -> CompareResult {
    // First check if the images have the same dimensions
    // If dimensions don't match, the comparison fails immediately
    if glsl_output.dimensions() != wgsl_output.dimensions() {
        return CompareResult::DimensionMismatch {
            glsl_dimensions: glsl_output.dimensions(),
            wgsl_dimensions: wgsl_output.dimensions(),
        };
    }

    // Track which color components match across all pixels
    // Start with assumption that all components match
    let mut matched: [bool; 4] = [true; 4];

    // Compare each pixel between the two images
    for (glsl_pixel, wgsl_pixel) in glsl_output.pixels().zip(wgsl_output.pixels()) {
        // Check each color component (RGBA)
        for i in 0..4 {
            // If any pixel differs in this component, mark component as mismatched
            if glsl_pixel[i] != wgsl_pixel[i] {
                matched[i] = false;
            }
        }
    }

    // Return appropriate result based on component matching
    if matched.iter().all(|&x| x) {
        // All components matched perfectly
        CompareResult::Match
    } else {
        // Some components had mismatches - report which ones
        CompareResult::PixelMismatch {
            r_matched: matched[0],
            g_matched: matched[1],
            b_matched: matched[2],
            a_matched: matched[3],
        }
    }
}
