//! wgpu utility functions for verification tests
//!
//! This module provides helper functions for creating wgpu resources commonly
//! used in verification tests, including textures, samplers, and format selection.

use anime4k_wgpu_build::pipelines::SamplerFilterMode;

/// Texture usage flags for storage textures (output)
///
/// Includes binding, storage, and copy source capabilities
pub const TEXTURE_USAGE_STORAGE: wgpu::TextureUsages = wgpu::TextureUsages::TEXTURE_BINDING.union(wgpu::TextureUsages::STORAGE_BINDING).union(wgpu::TextureUsages::COPY_SRC);

/// Texture usage flags for input textures
///
/// Includes binding, storage, copy source, and copy destination capabilities
pub const TEXTURE_USAGE_INPUT: wgpu::TextureUsages = wgpu::TextureUsages::TEXTURE_BINDING
    .union(wgpu::TextureUsages::STORAGE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

/// Creates a texture sampler with the specified filter mode
///
/// # Arguments
/// * `device` - The wgpu device to create the sampler on
/// * `filter_mode` - The filtering mode (nearest or linear)
///
/// # Returns
/// A configured texture sampler
pub fn create_sampler(device: &wgpu::Device, filter_mode: SamplerFilterMode) -> wgpu::Sampler {
    // Convert from our filter mode enum to wgpu filter modes
    let (mag_filter, min_filter) = match filter_mode {
        SamplerFilterMode::Nearest => (wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
        SamplerFilterMode::Linear => (wgpu::FilterMode::Linear, wgpu::FilterMode::Linear),
    };

    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Texture Sampler"),
        // Clamp to edge to avoid artifacts when sampling at texture boundaries
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        // Use the filter mode specified by the caller
        mag_filter,
        min_filter,
        // No mipmapping for verification textures
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    })
}

/// Creates a 2D texture with the specified parameters
///
/// # Arguments
/// * `device` - The wgpu device to create the texture on
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
/// * `format` - Texture format
/// * `usage` - Texture usage flags
///
/// # Returns
/// A configured texture
pub fn create_texture(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Processing Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1, // 2D texture, single layer
        },
        mip_level_count: 1, // No mipmapping
        sample_count: 1,    // No multisampling
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[], // No alternative view formats needed
    })
}

/// Loads an image into a wgpu texture
///
/// # Arguments
/// * `device` - The wgpu device
/// * `queue` - The wgpu command queue
/// * `image` - The source image to load
///
/// # Returns
/// A Rgba32Float texture containing the image data
pub fn load_image_as_texture(device: &wgpu::Device, queue: &wgpu::Queue, image: &image::DynamicImage) -> Result<wgpu::Texture, Box<dyn std::error::Error>> {
    // Convert image to RGBA32F format for consistent processing
    let rgba_image = image.to_rgba32f();
    let (width, height) = rgba_image.dimensions();

    // Create texture with input usage flags
    let texture = create_texture(device, width, height, wgpu::TextureFormat::Rgba32Float, TEXTURE_USAGE_INPUT);

    // Upload image data to the texture
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // Convert f32 pixel data to little-endian bytes
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

/// Loads an image file into a wgpu texture
///
/// # Arguments
/// * `device` - The wgpu device
/// * `queue` - The wgpu command queue
/// * `image_path` - Path to the image file
///
/// # Returns
/// A Rgba32Float texture containing the loaded image data
pub fn load_image_file_as_texture(device: &wgpu::Device, queue: &wgpu::Queue, image_path: &str) -> Result<wgpu::Texture, Box<dyn std::error::Error>> {
    // Open the image file and load it into a texture
    load_image_as_texture(device, queue, &image::open(image_path)?)
}

/// Reads a wgpu texture back to an RGBA32F image
///
/// # Arguments
/// * `device` - The wgpu device
/// * `queue` - The wgpu command queue
/// * `texture` - The texture to read from
///
/// # Returns
/// An RGBA32F image containing the texture data
pub fn save_texture_as_image(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Result<image::Rgba32FImage, Box<dyn std::error::Error>> {
    // Get texture dimensions and format
    let wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: _,
    } = texture.size();
    let format = texture.format();

    // Determine the number of components and bytes per component based on format
    let (components, bytes_per_component) = match format {
        wgpu::TextureFormat::R32Float => (1, 4),    // Single component, 4 bytes per float
        wgpu::TextureFormat::Rg32Float => (2, 4),   // Two components, 4 bytes per float
        wgpu::TextureFormat::Rgba32Float => (4, 4), // Four components, 4 bytes per float
        _ => return Err(format!("Unsupported texture format for saving: {format:?}").into()),
    };

    // Calculate buffer requirements
    let buffer_size = (width * height * components * bytes_per_component) as u64;
    let bytes_per_row = width * components * bytes_per_component;

    // Create a buffer to copy texture data to CPU
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture to buffer
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

    // Map the buffer for reading (async operation)
    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Wait for the mapping operation to complete
    device.poll(wgpu::PollType::Wait)?;
    pollster::block_on(receiver.receive()).ok_or("Failed to map buffer for reading")??;

    // Get the mapped data as f32 values
    let data = buffer_slice.get_mapped_range();
    let float_data: &[f32] = bytemuck::cast_slice(&data);

    // Convert the texture data to RGBA32F format based on source format
    let image = match components {
        1 => {
            // R32Float - expand single component to grayscale RGBA
            let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
            for &r in float_data {
                // Use absolute value to handle potential negative values
                let val = r.abs();
                rgba_data.push(val); // R
                rgba_data.push(val); // G
                rgba_data.push(val); // B
                rgba_data.push(1.0); // A
            }
            image::Rgba32FImage::from_raw(width, height, rgba_data).ok_or("Failed to create RGBA32F image from data")?
        }
        2 => {
            // RG32Float - expand two components to RGBA with R,G components and blue=0, alpha=1
            let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
            for chunk in float_data.chunks(2) {
                rgba_data.push(chunk[0].abs()); // R
                rgba_data.push(chunk[1].abs()); // G
                rgba_data.push(0.0); // B
                rgba_data.push(1.0); // A
            }
            image::Rgba32FImage::from_raw(width, height, rgba_data).ok_or("Failed to create RGBA32F image from data")?
        }
        4 => {
            // RGBA32Float - direct conversion, already in the right format
            image::Rgba32FImage::from_raw(width, height, float_data.to_vec()).ok_or("Failed to create RGBA32F image from data")?
        }
        _ => return Err(format!("Unsupported component count: {components}").into()),
    };

    Ok(image)
}

/// Saves a wgpu texture to an image file
///
/// # Arguments
/// * `device` - The wgpu device
/// * `queue` - The wgpu command queue
/// * `texture` - The texture to save
/// * `output_path` - Path where to save the image file
///
/// # Returns
/// Result indicating success or failure
pub fn save_texture_as_image_file(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Convert texture to RGBA32F image
    let image = save_texture_as_image(device, queue, texture)?;
    // Convert to 8-bit RGBA for standard image formats
    let image_rgba8 = image::DynamicImage::ImageRgba32F(image).to_rgba8();
    // Save to file
    image_rgba8.save(output_path)?;
    Ok(())
}
