//! GLSL reference engine binary
//!
//! This binary processes images using original GLSL shaders to generate
//! reference output for verification purposes.

use anime4k_wgpu_verification::glsl_reference_engine::{GlslReferenceEngine, ImageProcessor, analyze_shader};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Check if analysis mode is requested
    if args.len() > 1 && args[1] == "--analyze" {
        if args.len() != 3 {
            eprintln!("Usage: {} --analyze <shader.glsl>", args[0]);
            return Ok(());
        }

        let shader_path = &args[2];
        return analyze_shader(shader_path).await;
    }

    if args.len() != 4 {
        eprintln!("Usage: {} <shader.glsl> <input_image> <output_image>", args[0]);
        return Ok(());
    }

    let shader_path = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    println!("GLSL Reference Engine Starting...");
    println!("- Shader: {shader_path}");
    println!("- Input: {input_path}");
    println!("- Output: {output_path}");

    // Verify input files exist
    if !Path::new(shader_path).exists() {
        return Err(format!("Shader file not found: {shader_path}").into());
    }
    if !Path::new(input_path).exists() {
        return Err(format!("Input image not found: {input_path}").into());
    }

    // Initialize engine
    let engine = GlslReferenceEngine::new().await?;
    let mut processor = ImageProcessor::new(engine);

    // Process the image
    processor.process_shader_pipeline(shader_path, input_path, output_path, true)?;

    println!("Processing completed successfully!");

    Ok(())
}
