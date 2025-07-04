//! CNN verification binary
//!
//! This binary compares GLSL and WGSL implementations of CNN-based
//! Anime4K shaders to verify correctness of the conversion.

use anime4k_wgpu_verification::{
    compare::{CompareResult, compare_images},
    glsl_reference_engine::{GlslReferenceEngine, ImageProcessor},
    wgsl_reference_engine::{PipelineProcessor, WgslReferenceEngine},
};

include!(concat!(env!("OUT_DIR"), "/converted_cnns/cnns.rs"));

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <input_image>", args[0]);
        return Ok(());
    }

    let input_path = &args[1];
    let input_image = image::open(input_path).map_err(|e| format!("Failed to open input image: {e}"))?;

    let wgsl_content_map = cnns::get_shader_map();

    for (name, manifest_content, glsl_content) in cnns::CNN_ITEMS {
        // println!("  Processing shader: {name}");

        let glsl_engine = GlslReferenceEngine::new().await?;
        let mut glsl_processor = ImageProcessor::new(glsl_engine);
        let (glsl_output, glsl_duration) = match glsl_processor.process_shader_pipeline_no_io(glsl_content, &input_image) {
            Ok(output) => output,
            Err(e) => {
                eprintln!("✗ Error processing GLSL pipeline for {name}: {e}");
                continue;
            }
        };

        let wgsl_engine = WgslReferenceEngine::new().await?;
        let mut wgsl_processor = match PipelineProcessor::new_from_data(wgsl_engine, manifest_content, &wgsl_content_map, &input_image, false) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("✗ Error initializing WGSL processor for {name}: {e}");
                continue;
            }
        };
        let (wgsl_output, wgsl_duration) = match wgsl_processor.execute_pipeline_no_io() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("✗ Error processing WGSL pipeline for {name}: {e}");
                continue;
            }
        };

        // Compare outputs
        let result = compare_images(&glsl_output, &wgsl_output);
        match result {
            CompareResult::Match => {
                println!("✓ Outputs match for shader {name} (GLSL: {glsl_duration:.2?}, WGSL: {wgsl_duration:.2?})");
            }
            CompareResult::DimensionMismatch { glsl_dimensions, wgsl_dimensions } => {
                eprintln!("✗ Dimension mismatch for shader {name}: GLSL {glsl_dimensions:?}, WGSL {wgsl_dimensions:?}");
            }
            CompareResult::PixelMismatch {
                r_matched,
                g_matched,
                b_matched,
                a_matched,
            } => {
                eprintln!("✗ Pixel mismatch for shader {name}: R {r_matched}, G {g_matched}, B {b_matched}, A {a_matched}");
            }
        }
    }

    Ok(())
}
