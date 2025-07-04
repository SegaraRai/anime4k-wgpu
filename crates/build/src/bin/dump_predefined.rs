//! Predefined pipeline dumping tool
//!
//! This binary processes all predefined Anime4K pipelines (both auxiliary YAML manifests
//! and CNN/GAN GLSL files) and dumps them to a single JSON file with type information.

use anime4k_wgpu_build::{
    cnn_glsl_to_executable_pipeline,
    pipelines::ExecutablePipeline,
    predefined::{PREDEFINED_PIPELINES_AUX, PREDEFINED_PIPELINES_CNN},
    wgsl_to_executable_pipeline,
};
use serde::Serialize;
use std::{collections::HashMap, env, fs, path::Path, process};

/// A tagged enum representing different types of Anime4K pipelines
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
enum PipelineType {
    /// Auxiliary pipelines from YAML manifests
    #[serde(rename = "aux")]
    Auxiliary(ExecutablePipeline),
    /// CNN/GAN pipelines from GLSL files
    #[serde(rename = "cnn")]
    Cnn(ExecutablePipeline),
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 && args.len() != 4 {
        eprintln!("Usage: {} <project_root> <output_file> [--minify]", args[0]);
        eprintln!("Processes all predefined pipelines and dumps them to a JSON file");
        eprintln!("  project_root: Path to the anime4k-wgpu project root");
        eprintln!("  output_file:  Path to the output JSON file");
        eprintln!("  --minify:     Optional flag to minify WGSL code (default is false)");
        process::exit(1);
    }

    let project_root = Path::new(&args[1]);
    let output_file = &args[2];
    let minify = args.get(3).is_some_and(|arg| arg == "--minify");

    if !project_root.exists() {
        eprintln!("Error: Project root '{}' does not exist", project_root.display());
        process::exit(1);
    }

    let mut pipelines = HashMap::new();

    // Process predefined auxiliary pipelines
    match load_predefined_auxiliary_pipelines(project_root, minify) {
        Ok(aux_pipelines) => {
            println!("Found {} auxiliary pipelines", aux_pipelines.len());
            pipelines.extend(aux_pipelines);
        }
        Err(e) => {
            eprintln!("Error loading auxiliary pipelines: {e}");
            process::exit(1);
        }
    }

    // Process predefined CNN/GAN pipelines
    let helpers_dir = project_root.join("wgsl").join("helpers");
    if helpers_dir.exists() {
        match load_predefined_cnn_pipelines(project_root, &helpers_dir, minify) {
            Ok(cnn_pipelines) => {
                println!("Found {} CNN/GAN pipelines", cnn_pipelines.len());
                pipelines.extend(cnn_pipelines);
            }
            Err(e) => {
                eprintln!("Error loading CNN/GAN pipelines: {e}");
                process::exit(1);
            }
        }
    } else {
        eprintln!("Warning: helpers directory not found at {}", helpers_dir.display());
    }

    println!("Total pipelines found: {}", pipelines.len());

    // Serialize to JSON
    match serde_json::to_string_pretty(&pipelines) {
        Ok(json) => {
            if let Err(e) = fs::write(output_file, json) {
                eprintln!("Error writing output file '{output_file}': {e}");
                process::exit(1);
            }
            println!("Successfully wrote {} pipelines to '{}'", pipelines.len(), output_file);
        }
        Err(e) => {
            eprintln!("Error serializing pipelines to JSON: {e}");
            process::exit(1);
        }
    }
}

/// Loads predefined auxiliary YAML pipeline manifests
fn load_predefined_auxiliary_pipelines(project_root: &Path, minify: bool) -> Result<HashMap<String, PipelineType>, Box<dyn std::error::Error>> {
    let mut pipelines = HashMap::new();

    for (name, path) in PREDEFINED_PIPELINES_AUX {
        let manifest_path = project_root.join(path);
        println!("Processing auxiliary pipeline: {name} ({path})");

        match wgsl_to_executable_pipeline(manifest_path.to_str().unwrap(), minify) {
            Ok(pipeline) => {
                let pipeline_with_type = PipelineType::Auxiliary(pipeline);
                pipelines.insert(name.to_string(), pipeline_with_type);
            }
            Err(e) => {
                eprintln!("Warning: Failed to load auxiliary pipeline '{name}': {e}");
            }
        }
    }

    Ok(pipelines)
}

/// Loads predefined CNN/GAN GLSL pipelines
fn load_predefined_cnn_pipelines(project_root: &Path, helpers_dir: &Path, minify: bool) -> Result<HashMap<String, PipelineType>, Box<dyn std::error::Error>> {
    let mut pipelines = HashMap::new();

    for (name, path) in PREDEFINED_PIPELINES_CNN {
        let glsl_path = project_root.join(path);
        println!("Processing CNN/GAN pipeline: {name} ({path})");

        match cnn_glsl_to_executable_pipeline(glsl_path.to_str().unwrap(), helpers_dir.to_str().unwrap(), minify) {
            Ok(pipeline) => {
                let pipeline_with_type = PipelineType::Cnn(pipeline);
                pipelines.insert(name.to_string(), pipeline_with_type);
            }
            Err(e) => {
                eprintln!("Warning: Failed to load CNN/GAN pipeline '{name}': {e}");
            }
        }
    }

    Ok(pipelines)
}
