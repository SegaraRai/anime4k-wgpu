//! Pipeline analysis tool
//!
//! This binary analyzes and displays the structure of Anime4K pipeline manifests.
//! It loads a YAML manifest file, compiles it into an ExecutablePipeline, and
//! displays detailed information about the resulting pipeline structure.

use anime4k_wgpu_build::pipelines::ExecutablePipeline;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <manifest.yaml>", args[0]);
        eprintln!("Analyzes a YAML manifest and dumps the analyzed result to stdout");
        process::exit(1);
    }

    let manifest_path = &args[1];

    // Check if the manifest file exists
    if !Path::new(manifest_path).exists() {
        eprintln!("Error: Manifest file '{manifest_path}' does not exist");
        process::exit(1);
    }

    // Define shader file loader function
    let load_shader_file = |file_path: &str| -> Result<String, std::io::Error> {
        // Try to load the shader file relative to the manifest directory
        let manifest_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));
        let shader_path = manifest_dir.join(file_path);

        if shader_path.exists() {
            fs::read_to_string(shader_path)
        } else {
            // If not found relative to manifest, try absolute path
            fs::read_to_string(file_path)
        }
    };

    // Load and compile the pipeline manifest
    match ExecutablePipeline::from_file(manifest_path, load_shader_file) {
        Ok(executable) => {
            // Dump the result using Debug formatting
            println!("{executable:#?}");
        }
        Err(e) => {
            eprintln!("Error compiling manifest '{manifest_path}': {e}");
            process::exit(1);
        }
    }
}
