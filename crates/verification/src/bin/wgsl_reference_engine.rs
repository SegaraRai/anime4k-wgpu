//! WGSL reference engine binary
//!
//! This binary processes images using WGSL shader pipelines to generate
//! reference output for verification purposes. It executes pre-compiled
//! pipeline manifests and outputs processed images for comparison testing.

use anime4k_wgpu_verification::wgsl_reference_engine::{PipelineProcessor, WgslReferenceEngine};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <pipeline.yaml> <input_image> <output_image>", args[0]);
        return Ok(());
    }

    let pipeline_path = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    println!("WGSL Reference Engine Starting...");
    println!("- Pipeline: {}", pipeline_path);
    println!("- Input: {}", input_path);
    println!("- Output: {}", output_path);

    // Verify input files exist
    if !Path::new(pipeline_path).exists() {
        return Err(format!("Pipeline file not found: {}", pipeline_path).into());
    }
    if !Path::new(input_path).exists() {
        return Err(format!("Input image not found: {}", input_path).into());
    }

    // Initialize engine
    let engine = WgslReferenceEngine::new().await?;

    // Initialize processor with all resources pre-allocated
    let mut processor = PipelineProcessor::new_from_file(engine, pipeline_path, input_path, true)?;

    // Execute the pre-prepared pipeline
    let output_path_base = Path::new(output_path).with_extension("").to_str().unwrap().to_string();
    processor.execute_pipeline(output_path, Some(&output_path_base))?;

    println!("Processing completed successfully!");

    Ok(())
}
