//! Anime4K CLI Image Upscaler
//!
//! A command-line tool for upscaling images using the Anime4K algorithm with GPU acceleration.
//! This tool provides a simple interface to apply various Anime4K presets and performance
//! settings to enhance image quality through neural network-based upscaling.
//!
//! # Features
//! - Multiple Anime4K algorithm presets (A, B, C, AA, BB, CA)
//! - Configurable performance levels (Light, Medium, High, Ultra, Extreme)
//! - GPU-accelerated processing using wgpu
//! - Support for various image formats
//! - Batch processing capability through command-line interface
//!
//! # Usage
//! ```bash
//! anime4k-cli input.png output.png --scale-factor 2.0 --preset a --performance high
//! ```

use anime4k_wgpu::{
    PipelineExecutor,
    presets::{Anime4KPerformancePreset, Anime4KPreset},
};
use clap::Parser;
use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;

/// Command-line arguments for the Anime4K image upscaler
///
/// Defines the interface for controlling upscaling parameters including
/// input/output files, scale factor, algorithm preset, and performance level.
#[derive(Parser)]
#[command(version, about = "CLI tool for upscaling images using Anime4K")]
struct Args {
    /// Input image file path
    input: PathBuf,

    /// Output image file path
    output: PathBuf,

    /// Scale factor (e.g., 2.0 for 2x upscaling)
    /// Note: This program does not support downscaling. Scale factors are treated as powers of 2 greater than or equal to 2.
    #[arg(long, short, default_value = "2.0")]
    scale_factor: f64,

    /// Anime4K preset (a, b, c, aa, bb, ca)
    #[arg(long, short, default_value = "a")]
    preset: String,

    /// Performance preset (light, medium, high, ultra, extreme)
    #[arg(long, short = 'e', default_value = "high")]
    performance: String,
}

/// Main application entry point
///
/// Orchestrates the complete image upscaling pipeline:
/// 1. Parse command-line arguments and validate presets
/// 2. Load and prepare the input image
/// 3. Initialize GPU context and resources
/// 4. Create and execute the Anime4K processing pipeline
/// 5. Save the upscaled result
///
/// # Returns
/// `Ok(())` on successful completion, or an error if any step fails
///
/// # Errors
/// May return errors for:
/// - Invalid command-line arguments
/// - Image loading/saving failures
/// - GPU initialization problems
/// - Pipeline execution issues
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Parse and validate Anime4K algorithm preset
    let preset = match args.preset.to_lowercase().as_str() {
        "a" => Anime4KPreset::ModeA,
        "b" => Anime4KPreset::ModeB,
        "c" => Anime4KPreset::ModeC,
        "aa" => Anime4KPreset::ModeAA,
        "bb" => Anime4KPreset::ModeBB,
        "ca" => Anime4KPreset::ModeCA,
        _ => {
            eprintln!("Invalid preset '{}'. Valid presets: a, b, c, aa, bb, ca", args.preset);
            std::process::exit(1);
        }
    };

    // Parse and validate performance preset
    let performance_preset = match args.performance.to_lowercase().as_str() {
        "light" => Anime4KPerformancePreset::Light,
        "medium" => Anime4KPerformancePreset::Medium,
        "high" => Anime4KPerformancePreset::High,
        "ultra" => Anime4KPerformancePreset::Ultra,
        "extreme" => Anime4KPerformancePreset::Extreme,
        _ => {
            eprintln!("Invalid performance preset '{}'. Valid presets: light, medium, high, ultra, extreme", args.performance);
            std::process::exit(1);
        }
    };

    // Load input image
    println!("Loading image from: {}", args.input.display());
    let input_image = image::open(&args.input)?;
    let (input_width, input_height) = input_image.dimensions();
    println!("Input image: {input_width}x{input_height}");

    // Calculate expected output dimensions based on scale factor
    let scale_factor_u32 = args.scale_factor.ceil() as u32;
    let expected_width = input_width * scale_factor_u32;
    let expected_height = input_height * scale_factor_u32;
    println!("Expected output: {}x{} (scale factor: {})", expected_width, expected_height, args.scale_factor);

    // Initialize wgpu context for GPU processing
    println!("Initializing GPU...");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    // Request high-performance GPU adapter
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;

    // Create device with required features for Anime4K processing
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::FLOAT32_FILTERABLE,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: Default::default(),
    }))?;

    println!("GPU initialized successfully");

    // Convert input image to GPU texture format
    println!("Loading image to GPU texture...");
    let input_texture = load_image_to_texture(&device, &queue, &input_image, wgpu::TextureFormat::Rgba32Float)?;

    // Create processing pipelines for the selected configuration
    println!("Setting up Anime4K pipeline with preset '{}' and performance '{}'", args.preset, args.performance);
    let pipelines = preset.create_pipelines(performance_preset, args.scale_factor);
    if pipelines.is_empty() {
        return Err("No pipelines generated for the selected preset".into());
    }

    println!("Pipeline will use {} stages", pipelines.len());

    // Create and configure the shader pipeline
    let (pipeline, output_texture) = PipelineExecutor::new(&pipelines, &device, &input_texture);

    // Execute the Anime4K processing pipeline
    println!("Executing Anime4K pipeline...");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Anime4K Pipeline") });

    pipeline.pass(&mut encoder);

    queue.submit(std::iter::once(encoder.finish()));

    // Wait for GPU processing to complete
    device.poll(wgpu::PollType::Wait)?;

    // Convert result back to image format and save
    println!("Saving result to: {}", args.output.display());
    let output_image = save_texture_to_image(&device, &queue, &output_texture)?;
    let output_rgba8 = DynamicImage::ImageRgba32F(output_image).to_rgba8();
    output_rgba8.save(&args.output)?;

    println!(
        "Successfully upscaled image from {}x{} to {}x{}",
        input_width,
        input_height,
        output_texture.width(),
        output_texture.height()
    );

    Ok(())
}

/// Loads an image into a wgpu texture for GPU processing
///
/// Converts the input image to RGBA32F format and uploads it to GPU memory
/// with appropriate usage flags for both reading and writing operations.
///
/// # Arguments
/// * `device` - wgpu device for creating GPU resources
/// * `queue` - Command queue for uploading data
/// * `image` - Input image to convert
/// * `format` - Target texture format (typically RGBA32F)
///
/// # Returns
/// A GPU texture containing the image data ready for processing
///
/// # Errors
/// Returns an error if texture creation or data upload fails
// Helper functions for texture operations
fn load_image_to_texture(device: &wgpu::Device, queue: &wgpu::Queue, image: &DynamicImage, format: wgpu::TextureFormat) -> Result<wgpu::Texture, Box<dyn std::error::Error>> {
    // Convert image to RGBA32F format for high-precision processing
    let rgba_image = image.to_rgba32f();
    let (width, height) = rgba_image.dimensions();

    // Create texture with appropriate usage flags
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Input Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        // Enable binding for reading and storage for writing during processing
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Upload image data to GPU memory
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // Convert f32 values to byte representation
        &rgba_image.as_raw().iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<_>>(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4 * 4), // 4 components * 4 bytes per f32
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    Ok(texture)
}

/// Saves a wgpu texture back to an image format
///
/// Downloads texture data from GPU memory and converts it back to a standard
/// image format. Handles different texture formats and expands them to RGBA
/// as needed for compatibility with image saving libraries.
///
/// # Arguments
/// * `device` - wgpu device for creating GPU resources
/// * `queue` - Command queue for data transfer operations
/// * `texture` - GPU texture containing the processed image data
///
/// # Returns
/// An RGBA32F image ready for format conversion and saving
///
/// # Errors
/// Returns an error if:
/// - Texture format is unsupported
/// - GPU memory mapping fails
/// - Image reconstruction fails
fn save_texture_to_image(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Result<image::Rgba32FImage, Box<dyn std::error::Error>> {
    let wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: _,
    } = texture.size();
    let format = texture.format();

    // Determine format characteristics for proper data interpretation
    let (components, bytes_per_component) = match format {
        wgpu::TextureFormat::R32Float => (1, 4),
        wgpu::TextureFormat::Rg32Float => (2, 4),
        wgpu::TextureFormat::Rgba32Float => (4, 4),
        _ => return Err(format!("Unsupported texture format for saving: {format:?}").into()),
    };

    let buffer_size = (width * height * components * bytes_per_component) as u64;
    let bytes_per_row = width * components * bytes_per_component;

    // Create staging buffer for GPU-to-CPU data transfer
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture data to staging buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Copy Encoder") });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map buffer for CPU access and wait for completion
    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    device.poll(wgpu::PollType::Wait)?;

    pollster::block_on(receiver.receive()).ok_or("Failed to map buffer for reading")??;

    // Convert raw bytes back to float data
    let data = buffer_slice.get_mapped_range();
    let float_data: &[f32] = bytemuck::cast_slice(&data);

    // Convert data to RGBA format based on source format
    let image = match components {
        1 => {
            // R32Float - expand single component to grayscale RGBA
            let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
            for &r in float_data {
                rgba_data.push(r.abs());
                rgba_data.push(r.abs());
                rgba_data.push(r.abs());
                rgba_data.push(1.0);
            }
            image::Rgba32FImage::from_raw(width, height, rgba_data).ok_or("Failed to create RGBA32F image from data")?
        }
        2 => {
            // RG32Float - expand two components to RGBA with zero blue and full alpha
            let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
            for chunk in float_data.chunks(2) {
                rgba_data.push(chunk[0].abs());
                rgba_data.push(chunk[1].abs());
                rgba_data.push(0.0);
                rgba_data.push(1.0);
            }
            image::Rgba32FImage::from_raw(width, height, rgba_data).ok_or("Failed to create RGBA32F image from data")?
        }
        4 => {
            // RGBA32Float - direct conversion, already in correct format
            image::Rgba32FImage::from_raw(width, height, float_data.to_vec()).ok_or("Failed to create RGBA32F image from data")?
        }
        _ => return Err(format!("Unsupported number of components: {components}").into()),
    };

    Ok(image)
}
