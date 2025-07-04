//! Auxiliary shader verification binary
//!
//! This binary compares GLSL and WGSL implementations of auxiliary
//! Anime4K shaders (non-CNN based) to verify correctness of the conversion.

use anime4k_wgpu_verification::{
    compare::{CompareResult, compare_images},
    glsl_reference_engine::{GlslReferenceEngine, ImageProcessor},
    wgsl_reference_engine::{PipelineProcessor, WgslReferenceEngine},
};

/// Returns pairs of (GLSL path, WGSL manifest path) for verification
fn get_preset_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("anime4k-glsl/Restore/Anime4K_Clamp_Highlights.glsl", "wgsl/auxiliary/clamp_highlights_manifest.yaml"),
        ("anime4k-glsl/Deblur/Anime4K_Deblur_DoG.glsl", "wgsl/auxiliary/deblur_dog_manifest.yaml"),
        ("anime4k-glsl/Deblur/Anime4K_Deblur_Original.glsl", "wgsl/auxiliary/deblur_original_manifest.yaml"),
        ("anime4k-glsl/Denoise/Anime4K_Denoise_Bilateral_Mean.glsl", "wgsl/auxiliary/denoise_bilateral_mean_manifest.yaml"),
        ("anime4k-glsl/Denoise/Anime4K_Denoise_Bilateral_Median.glsl", "wgsl/auxiliary/denoise_bilateral_median_manifest.yaml"),
        ("anime4k-glsl/Denoise/Anime4K_Denoise_Bilateral_Mode.glsl", "wgsl/auxiliary/denoise_bilateral_mode_manifest.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Darken_HQ.glsl", "wgsl/auxiliary/effects_darken_manifest_hq.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Darken_Fast.glsl", "wgsl/auxiliary/effects_darken_manifest_fast.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Darken_VeryFast.glsl", "wgsl/auxiliary/effects_darken_manifest_veryfast.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Thin_HQ.glsl", "wgsl/auxiliary/effects_thin_manifest_hq.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Thin_Fast.glsl", "wgsl/auxiliary/effects_thin_manifest_fast.yaml"),
        ("anime4k-glsl/Experimental-Effects/Anime4K_Thin_VeryFast.glsl", "wgsl/auxiliary/effects_thin_manifest_veryfast.yaml"),
        ("anime4k-glsl/Upscale/Anime4K_Upscale_DoG_x2.glsl", "wgsl/auxiliary/upscale_dog_x2_manifest.yaml"),
        //("anime4k-glsl/Upscale/Anime4K_Upscale_DTD_x2.glsl", "wgsl/auxiliary/upscale_dtd_x2_manifest.yaml"),
        ("anime4k-glsl/Upscale/Anime4K_Upscale_Original_x2.glsl", "wgsl/auxiliary/upscale_original_x2_manifest.yaml"),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <input_image>", args[0]);
        return Ok(());
    }

    let input_path = &args[1];
    let input_image = image::open(input_path).map_err(|e| format!("Failed to open input image: {e}"))?;

    for (glsl_path, wgsl_path) in get_preset_pairs() {
        // println!("  Processing shader: {glsl_path} and {wgsl_path}");

        let glsl_content = std::fs::read_to_string(glsl_path).map_err(|e| format!("Failed to read GLSL shader file {glsl_path}: {e}"))?;
        let glsl_engine = GlslReferenceEngine::new().await?;
        let mut glsl_processor = ImageProcessor::new(glsl_engine);
        let (glsl_output, glsl_duration) = match glsl_processor.process_shader_pipeline_no_io(&glsl_content, &input_image) {
            Ok(output) => output,
            Err(e) => {
                eprintln!("✗ Error processing GLSL pipeline for {glsl_path}: {e}");
                continue;
            }
        };

        let wgsl_engine = WgslReferenceEngine::new().await?;
        let mut wgsl_processor = match PipelineProcessor::new_from_file(wgsl_engine, wgsl_path, input_path, false) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("✗ Error initializing WGSL processor for {wgsl_path}: {e}");
                continue;
            }
        };
        let (wgsl_output, wgsl_duration) = match wgsl_processor.execute_pipeline_no_io() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("✗ Error processing WGSL pipeline for {wgsl_path}: {e}");
                continue;
            }
        };

        // Compare outputs
        let result = compare_images(&glsl_output, &wgsl_output);
        match result {
            CompareResult::Match => {
                println!("✓ Outputs match for shader {wgsl_path} (GLSL: {glsl_duration:.2?}, WGSL: {wgsl_duration:.2?})");
            }
            CompareResult::DimensionMismatch { glsl_dimensions, wgsl_dimensions } => {
                eprintln!("✗ Dimension mismatch for shader {wgsl_path}: GLSL {glsl_dimensions:?}, WGSL {wgsl_dimensions:?}");
            }
            CompareResult::PixelMismatch {
                r_matched,
                g_matched,
                b_matched,
                a_matched,
            } => {
                eprintln!("✗ Pixel mismatch for shader {wgsl_path}: R {r_matched}, G {g_matched}, B {b_matched}, A {a_matched}");
            }
        }
    }

    Ok(())
}
