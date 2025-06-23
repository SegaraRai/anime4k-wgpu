//! WGSL reference implementation engine
//!
//! This module provides a reference implementation engine that processes
//! WGSL shader pipelines to generate reference output for verification.

use crate::wgpu_helpers::*;
use anime4k_wgpu_build::pipelines::{ExecutablePass, ExecutablePipeline, PhysicalTexture, SamplerFilterMode};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Workgroup size for compute shaders (X dimension)
const COMPUTE_WORKGROUP_SIZE_X: u32 = 8;
/// Workgroup size for compute shaders (Y dimension)
const COMPUTE_WORKGROUP_SIZE_Y: u32 = 8;

/// Calculates the number of workgroups needed for a given size
fn calculate_workgroup_count(size: u32, workgroup_size: u32) -> u32 {
    size.div_ceil(workgroup_size)
}

/// A shader pass prepared for execution with bound resources
#[derive(Debug)]
struct PreparedPass {
    /// Unique identifier for this pass
    id: String,
    /// The compute pipeline
    pipeline: wgpu::ComputePipeline,
    /// Bind group with all resources
    bind_group: wgpu::BindGroup,
    /// Physical texture IDs for outputs
    output_physical_ids: Vec<u32>,
    /// Compute dispatch dimensions (width, height)
    compute_dimensions: (u32, u32),
}

/// WGSL reference engine for generating reference output
///
/// Provides a high-performance GPU-based engine for executing WGSL shader pipelines.
/// Used primarily for verification and testing of converted shader implementations.
pub struct WgslReferenceEngine {
    /// The wgpu device
    device: wgpu::Device,
    /// The wgpu command queue
    queue: wgpu::Queue,
    /// Whether the device supports filterable 32-bit float textures
    has_float32_filterable: bool,
}

/// Pipeline processor that manages execution of an analyzed pipeline
///
/// Handles resource allocation, shader compilation, and execution of complete
/// WGSL shader pipelines with pre-optimized resource binding and memory layout.
pub struct PipelineProcessor {
    /// The underlying WGSL reference engine
    engine: WgslReferenceEngine,
    /// The pipelines to execute
    executable_pipeline: ExecutablePipeline,
    /// Physical textures allocated for the pipeline
    physical_textures: HashMap<u32, wgpu::Texture>,
    /// Prepared shader passes ready for execution
    prepared_passes: Vec<PreparedPass>,
    /// Cache of texture samplers by filter mode
    sampler_map: HashMap<SamplerFilterMode, wgpu::Sampler>,
    /// Input image width
    input_width: u32,
    /// Input image height
    input_height: u32,
    /// Whether to enable debug logging
    log: bool,
}

impl WgslReferenceEngine {
    /// Creates a new WGSL reference engine
    ///
    /// # Returns
    /// A new engine instance or an error if initialization fails
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("WGSL Reference Engine"),
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | wgpu::Features::FLOAT32_FILTERABLE,
                required_limits: wgpu::Limits {
                    max_storage_textures_per_shader_stage: 30,
                    ..Default::default()
                },
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await?;

        // Check if the device actually supports FLOAT32_FILTERABLE
        let has_float32_filterable = device.features().contains(wgpu::Features::FLOAT32_FILTERABLE);

        Ok(Self {
            device,
            queue,
            has_float32_filterable,
        })
    }
}

impl PipelineProcessor {
    /// Creates a new pipeline processor from file paths
    ///
    /// # Arguments
    /// * `engine` - The WGSL reference engine to use
    /// * `pipeline_path` - Path to the pipeline manifest YAML file
    /// * `input_path` - Path to the input image file
    /// * `log` - Whether to enable debug logging
    ///
    /// # Returns
    /// A configured pipeline processor ready for execution
    pub fn new_from_file(engine: WgslReferenceEngine, pipeline_path: &str, input_path: &str, log: bool) -> Result<Self, Box<dyn std::error::Error>> {
        // Load input image and determine appropriate texture format
        let format = determine_texture_format(engine.has_float32_filterable);
        let input_texture = load_image_file_as_texture(&engine.device, &engine.queue, input_path, format)?;

        let wgpu::Extent3d {
            width: input_width,
            height: input_height,
            ..
        } = input_texture.size();

        // Load and compile pipeline
        let executable_pipeline = Self::load_and_compile_pipeline(pipeline_path)?;

        let mut sampler_map: HashMap<SamplerFilterMode, wgpu::Sampler> = HashMap::new();
        for filter_mode in executable_pipeline.required_samplers.iter().copied() {
            let sampler = create_sampler(&engine.device, filter_mode);
            sampler_map.insert(filter_mode, sampler);
        }

        let mut processor = Self {
            engine,
            executable_pipeline,
            physical_textures: HashMap::new(),
            prepared_passes: Vec::new(),
            sampler_map,
            input_width,
            input_height,
            log,
        };

        processor.initialize_all_resources(input_texture)?;

        Ok(processor)
    }

    pub fn new_from_data(
        engine: WgslReferenceEngine,
        pipeline_content: &str,
        shader_map: &HashMap<&str, &str>,
        input_image: &image::DynamicImage,
        log: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load input image
        let format = determine_texture_format(engine.has_float32_filterable);
        let input_texture = load_image_as_texture(&engine.device, &engine.queue, input_image, format)?;

        let wgpu::Extent3d {
            width: input_width,
            height: input_height,
            ..
        } = input_texture.size();

        // Load and compile pipeline
        let executable_pipeline = ExecutablePipeline::from_yaml(pipeline_content, |file| {
            shader_map
                .get(file)
                .map(|&content| content.to_string())
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("Failed to load shader file '{}'", file)))
        })?;

        let mut sampler_map: HashMap<SamplerFilterMode, wgpu::Sampler> = HashMap::new();
        for filter_mode in executable_pipeline.required_samplers.iter().copied() {
            let sampler = create_sampler(&engine.device, filter_mode);
            sampler_map.insert(filter_mode, sampler);
        }

        let mut processor = Self {
            engine,
            executable_pipeline,
            physical_textures: HashMap::new(),
            prepared_passes: Vec::new(),
            sampler_map,
            input_width,
            input_height,
            log,
        };

        processor.initialize_all_resources(input_texture)?;

        Ok(processor)
    }

    /// Loads and compiles a pipeline from a YAML manifest file
    ///
    /// Reads the pipeline specification from a YAML file and compiles it into
    /// an optimized ExecutablePipeline ready for GPU execution.
    ///
    /// # Arguments
    /// * `pipeline_path` - Path to the YAML pipeline manifest
    ///
    /// # Returns
    /// An ExecutablePipeline ready for execution
    fn load_and_compile_pipeline(pipeline_path: &str) -> Result<ExecutablePipeline, Box<dyn std::error::Error>> {
        // Get the directory containing the manifest file for relative shader path resolution
        let manifest_dir = Path::new(pipeline_path).parent().ok_or("Failed to get manifest directory")?;

        // Create a shader file loader that resolves paths relative to the manifest
        let load_shader_file = |file: &str| -> Result<String, std::io::Error> {
            // Shader filenames in manifests are relative to the manifest file location
            let shader_path = manifest_dir.join(file);
            fs::read_to_string(shader_path)
        };

        // Load, parse, and compile the pipeline specification from the manifest file
        ExecutablePipeline::from_file(pipeline_path, load_shader_file)
    }

    /// Determines the appropriate texture format based on channel count and device capabilities
    ///
    /// # Arguments
    /// * `channels` - Number of color channels (1-4)
    ///
    /// # Returns
    /// The most appropriate texture format for the given channel count
    fn get_texture_format_for_channels(&self, channels: u32) -> wgpu::TextureFormat {
        if !self.engine.has_float32_filterable {
            // Use filterable 16-bit formats when 32-bit filtering is not supported
            match channels {
                1 => wgpu::TextureFormat::R16Float,        // Single channel, 16-bit float
                2 => wgpu::TextureFormat::Rg16Float,       // Two channels, 16-bit float
                3 | 4 => wgpu::TextureFormat::Rgba16Float, // 3 or 4 channels, 16-bit float (no RGB16Float)
                _ => wgpu::TextureFormat::Rgba16Float,     // Default fallback
            }
        } else {
            // Use high precision 32-bit formats when device supports float32 filtering
            match channels {
                1 => wgpu::TextureFormat::R32Float,        // Single channel, 32-bit float
                2 => wgpu::TextureFormat::Rg32Float,       // Two channels, 32-bit float
                3 | 4 => wgpu::TextureFormat::Rgba32Float, // 3 or 4 channels, 32-bit float
                _ => wgpu::TextureFormat::Rgba32Float,     // Default fallback
            }
        }
    }

    /// Initializes all GPU resources needed for pipeline execution
    ///
    /// # Arguments
    /// * `input_texture` - The input texture to process
    ///
    /// # Returns
    /// Result indicating success or failure
    fn initialize_all_resources(&mut self, input_texture: wgpu::Texture) -> Result<(), Box<dyn std::error::Error>> {
        // Log pipeline information if debug logging is enabled
        if self.log {
            println!("Initializing pipeline: {} ({})", self.executable_pipeline.name, self.executable_pipeline.id);
            println!("Description: {}", self.executable_pipeline.description.as_deref().unwrap_or("No description"));
            println!("Found {} shader passes", self.executable_pipeline.passes.len());
            println!("Found {} physical textures", self.executable_pipeline.physical_textures.len());
        }

        // Step 1: Allocate all physical textures based on the executable pipeline
        self.allocate_physical_textures(input_texture)?;

        // Step 2: Prepare all shader passes (compile shaders, create pipelines and bind groups)
        self.prepare_all_shader_passes()?;

        // Log successful completion if debug logging is enabled
        if self.log {
            println!("All resources initialized successfully");
        }

        Ok(())
    }

    /// Allocates all physical textures needed by the pipeline
    ///
    /// # Arguments
    /// * `input_texture` - The source input texture
    ///
    /// # Returns
    /// Result indicating success or failure
    fn allocate_physical_textures(&mut self, input_texture: wgpu::Texture) -> Result<(), Box<dyn std::error::Error>> {
        // Process each physical texture defined in the pipeline manifest
        for physical_texture in &self.executable_pipeline.physical_textures {
            if physical_texture.is_source {
                // Source texture: use the provided input texture directly
                self.physical_textures.insert(physical_texture.id, input_texture.clone());
                if self.log {
                    println!(
                        "Assigned SOURCE texture (ID {}): {}x{} channels={}",
                        physical_texture.id, self.input_width, self.input_height, physical_texture.channels
                    );
                }
            } else {
                // Intermediate/output texture: calculate dimensions and create new texture
                let (width, height) = self.calculate_physical_texture_dimensions(physical_texture);
                let format = self.get_texture_format_for_channels(physical_texture.channels);

                // Create texture with storage usage for shader writes
                let texture = create_texture(&self.engine.device, width, height, format, TEXTURE_USAGE_STORAGE);
                self.physical_textures.insert(physical_texture.id, texture);
                if self.log {
                    println!(
                        "Allocated physical texture (ID {}): {}x{} {:?} channels={}",
                        physical_texture.id, width, height, format, physical_texture.channels
                    );
                }
            }
        }

        Ok(())
    }

    /// Calculates the dimensions for a physical texture based on scale factors
    ///
    /// # Arguments
    /// * `physical_texture` - The physical texture descriptor
    ///
    /// # Returns
    /// Tuple of (width, height) for the texture
    fn calculate_physical_texture_dimensions(&self, physical_texture: &PhysicalTexture) -> (u32, u32) {
        // Extract scale factors from the physical texture definition
        let width_scale = physical_texture.scale_factor.0.to_f64();
        let height_scale = physical_texture.scale_factor.1.to_f64();

        // Apply scale factors to input dimensions and floor to get integer dimensions
        let width = (self.input_width as f64 * width_scale).floor() as u32;
        let height = (self.input_height as f64 * height_scale).floor() as u32;

        (width, height)
    }

    /// Prepares all shader passes by compiling shaders and creating pipelines
    ///
    /// # Returns
    /// Result indicating success or failure
    fn prepare_all_shader_passes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Process each shader pass in the pipeline
        for (pass_index, shader_pass) in self.executable_pipeline.passes.iter().enumerate() {
            if self.log {
                println!("Preparing shader pass {}: {}", pass_index, shader_pass.id);
                println!("  Creating shader module for pass '{}' with {} chars of WGSL", shader_pass.id, shader_pass.shader.len());
            }

            // Compile the WGSL shader into a shader module
            let shader_module = self.engine.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("wgsl_shader_module_{}", shader_pass.id)),
                source: wgpu::ShaderSource::Wgsl(shader_pass.shader.clone().into()),
            });
            if self.log {
                println!("  Shader module created successfully");
            }

            // Create bind group layout describing all resources this pass needs
            let mut bind_group_layout_entries = Vec::new();

            // Add input texture bindings for reading from previous passes or source data
            for input in &shader_pass.input_textures {
                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: input.binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
            }

            // Add output texture bindings for writing shader results
            for output in &shader_pass.output_textures {
                // Determine appropriate format based on channel count
                let format = self.get_texture_format_for_channels(output.channels);
                // Map to storage texture format (must match exactly for write operations)
                let storage_format = match format {
                    wgpu::TextureFormat::R32Float => wgpu::TextureFormat::R32Float,
                    wgpu::TextureFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
                    wgpu::TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
                    wgpu::TextureFormat::R16Float => wgpu::TextureFormat::R16Float,
                    wgpu::TextureFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
                    wgpu::TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
                    _ => wgpu::TextureFormat::Rgba32Float, // Fallback to RGBA32Float
                };

                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: output.binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: storage_format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                });
            }

            // Add sampler bindings for texture filtering operations
            for sampler in &shader_pass.samplers {
                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: sampler.binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }

            // Sort by binding number to ensure consistent layout ordering
            bind_group_layout_entries.sort_by_key(|entry| entry.binding);

            // Create bind group layout describing the resource binding structure
            let bind_group_layout = self.engine.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&format!("wgsl_bind_group_layout_{}", shader_pass.id)),
                entries: &bind_group_layout_entries,
            });

            // Create pipeline layout containing the bind group layout
            let pipeline_layout = self.engine.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("wgsl_pipeline_layout_{}", shader_pass.id)),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[], // No push constants needed
            });

            // Create compute pipeline with explicit resource layout
            let compute_pipeline = self.engine.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("wgsl_compute_pipeline_{}", shader_pass.id)),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

            // Create bind group with actual texture resources bound to the pipeline
            let bind_group = self.create_bind_group_for_shader_pass(shader_pass, &compute_pipeline)?;

            // Extract physical texture IDs for output textures (for result saving)
            let output_physical_ids: Vec<u32> = shader_pass.output_textures.iter().map(|o| o.physical_id).collect();

            // Create prepared pass with all resources ready for execution
            let prepared_pass = PreparedPass {
                id: shader_pass.id.clone(),
                pipeline: compute_pipeline,
                bind_group,
                output_physical_ids,
                // Calculate actual compute dimensions based on scale factors
                compute_dimensions: (
                    (self.input_width as f64 * shader_pass.compute_scale_factors.0).floor() as u32,
                    (self.input_height as f64 * shader_pass.compute_scale_factors.1).floor() as u32,
                ),
            };

            // Add the prepared pass to the execution queue
            self.prepared_passes.push(prepared_pass);
        }

        Ok(())
    }

    /// Creates a bind group with all resources for a specific shader pass
    ///
    /// This function creates texture views for all input and output textures,
    /// retrieves the appropriate samplers, and binds them to create a complete
    /// bind group that can be used during compute pass execution.
    ///
    /// # Arguments
    /// * `shader_pass` - The shader pass requiring resource binding
    /// * `pipeline` - The compute pipeline to create bindings for
    ///
    /// # Returns
    /// A bind group with all resources bound, or an error if resources are missing
    fn create_bind_group_for_shader_pass(&self, shader_pass: &ExecutablePass, pipeline: &wgpu::ComputePipeline) -> Result<wgpu::BindGroup, Box<dyn std::error::Error>> {
        let mut bind_group_entries = Vec::new();

        if self.log {
            println!(
                "Creating bind group for pass '{}' with {} inputs, {} outputs, {} samplers",
                shader_pass.id,
                shader_pass.input_textures.len(),
                shader_pass.output_textures.len(),
                shader_pass.samplers.len()
            );
        }

        // Create texture views for all input textures (extend lifetime for bind group)
        let mut input_texture_views = Vec::new();
        for input in &shader_pass.input_textures {
            if let Some(texture) = self.physical_textures.get(&input.physical_id) {
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                input_texture_views.push(texture_view);
            } else {
                return Err(format!("Missing physical input texture: {} (ID {})", input.logical_id, input.physical_id).into());
            }
        }

        // Add input texture bindings to the bind group entries
        for (input, texture_view) in shader_pass.input_textures.iter().zip(input_texture_views.iter()) {
            if self.log {
                println!("  Adding input binding {}: {} (physical ID {})", input.binding, input.logical_id, input.physical_id);
            }
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: input.binding,
                resource: wgpu::BindingResource::TextureView(texture_view),
            });
        }

        // Create texture views for all output textures (extend lifetime for bind group)
        let mut output_texture_views = Vec::new();
        for output in &shader_pass.output_textures {
            if let Some(texture) = self.physical_textures.get(&output.physical_id) {
                let output_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                output_texture_views.push(output_view);
            } else {
                return Err(format!("Missing physical output texture: {} (ID {})", output.logical_id, output.physical_id).into());
            }
        }

        // Add output texture bindings to the bind group entries
        for (output, texture_view) in shader_pass.output_textures.iter().zip(output_texture_views.iter()) {
            if self.log {
                println!("  Adding output binding {}: {} (physical ID {})", output.binding, output.logical_id, output.physical_id);
            }
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: output.binding,
                resource: wgpu::BindingResource::TextureView(texture_view),
            });
        }

        // Add sampler bindings for texture filtering operations
        for sampler_binding in &shader_pass.samplers {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: sampler_binding.binding,
                resource: wgpu::BindingResource::Sampler(
                    self.sampler_map
                        .get(&sampler_binding.filter_mode)
                        .ok_or_else(|| format!("Sampler for filter mode {:?} not found in map", sampler_binding.filter_mode))?,
                ),
            });
        }

        // Sort bind group entries by binding number to ensure they match the layout order
        bind_group_entries.sort_by_key(|entry| entry.binding);

        if self.log {
            println!("  Total bindings created: {}", bind_group_entries.len());
            for entry in &bind_group_entries {
                println!("    Binding {}", entry.binding);
            }
        }

        // Create the final bind group with all resources bound
        let bind_group = self.engine.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("wgsl_bind_group_{}", shader_pass.id)),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &bind_group_entries,
        });

        Ok(bind_group)
    }

    /// Executes the complete pipeline and saves results to files
    ///
    /// Runs all prepared shader passes in sequence, optionally saving intermediate
    /// outputs for debugging, and saves the final result to the specified output path.
    /// This is the main execution method for file-based workflows.
    ///
    /// # Arguments
    /// * `output_path` - Path where to save the final processed image
    /// * `output_path_base` - Optional base path for saving intermediate outputs from each pass
    ///
    /// # Returns
    /// Result indicating success or failure of the pipeline execution
    pub fn execute_pipeline(&mut self, output_path: &str, output_path_base: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        if self.log {
            println!("Executing pipeline with {} prepared passes", self.prepared_passes.len());
        }

        // Execute each prepared shader pass in sequence
        for (pass_index, prepared_pass) in self.prepared_passes.iter().enumerate() {
            if self.log {
                println!("Executing pass {}: {}", pass_index, prepared_pass.id);
            }

            // Create command encoder for recording GPU commands
            let mut encoder = self.engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("wgsl_encoder_{}", prepared_pass.id)),
            });

            {
                // Begin compute pass within a scope for proper resource cleanup
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&format!("wgsl_compute_pass_{}", prepared_pass.id)),
                    timestamp_writes: None, // No GPU timing needed for this operation
                });

                // Set pipeline and bind all resources
                compute_pass.set_pipeline(&prepared_pass.pipeline);
                compute_pass.set_bind_group(0, &prepared_pass.bind_group, &[]);

                // Calculate workgroup dispatch dimensions based on compute dimensions
                let (compute_width, compute_height) = prepared_pass.compute_dimensions;
                let workgroup_x = calculate_workgroup_count(compute_width, COMPUTE_WORKGROUP_SIZE_X);
                let workgroup_y = calculate_workgroup_count(compute_height, COMPUTE_WORKGROUP_SIZE_Y);
                compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
            }

            // Submit commands to GPU queue for execution
            self.engine.queue.submit(std::iter::once(encoder.finish()));

            // Save intermediate outputs for debugging purposes if requested
            if let Some(output_path_base) = output_path_base {
                for physical_id in &prepared_pass.output_physical_ids {
                    if let Some(texture) = self.physical_textures.get(physical_id) {
                        let intermediate_path = format!("{output_path_base}_pass{}_phy{physical_id}.png", pass_index + 1);
                        save_texture_as_image_file(&self.engine.device, &self.engine.queue, texture, &intermediate_path)?;
                        if self.log {
                            println!("- Pass {pass_index} output saved to: {intermediate_path}");
                        }
                    }
                }
            }

            if self.log {
                let (output_width, output_height) = prepared_pass.compute_dimensions;
                println!("- Pass {} completed: dimensions: {}x{}", pass_index, output_width, output_height);
            }
        }

        // Save the final result using the pipeline's designated result texture
        if let Some(result_texture_id) = self.executable_pipeline.get_result_texture_id() {
            if let Some(result_texture) = self.physical_textures.get(&result_texture_id) {
                save_texture_as_image_file(&self.engine.device, &self.engine.queue, result_texture, output_path)?;
                println!("Final result saved to: {} (physical texture ID: {})", output_path, result_texture_id);
            } else {
                return Err(format!("Result texture with ID {} not found", result_texture_id).into());
            }
        } else {
            return Err("No RESULT texture found in pipeline analysis".into());
        }

        Ok(())
    }

    /// Executes the pipeline without file I/O operations for performance testing
    ///
    /// Runs all prepared shader passes in a single command buffer for optimal
    /// performance, then extracts the result image directly to memory. This method
    /// is designed for benchmarking and programmatic usage where file I/O overhead
    /// should be minimized.
    ///
    /// # Returns
    /// Tuple of (processed image, execution duration) or error
    pub fn execute_pipeline_no_io(&mut self) -> Result<(image::Rgba32FImage, std::time::Duration), Box<dyn std::error::Error>> {
        // Start timing the execution (excluding result extraction)
        let timepoint = std::time::Instant::now();

        // Create a single command encoder for all passes to minimize overhead
        let mut encoder = self.engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });

        // Execute all prepared passes within a single command buffer
        for prepared_pass in &self.prepared_passes {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("wgsl_compute_pass_{}", prepared_pass.id)),
                timestamp_writes: None,
            });

            // Set pipeline and bind resources for this pass
            compute_pass.set_pipeline(&prepared_pass.pipeline);
            compute_pass.set_bind_group(0, &prepared_pass.bind_group, &[]);

            // Calculate and dispatch workgroups for this pass
            let (compute_width, compute_height) = prepared_pass.compute_dimensions;
            let workgroup_x = calculate_workgroup_count(compute_width, COMPUTE_WORKGROUP_SIZE_X);
            let workgroup_y = calculate_workgroup_count(compute_height, COMPUTE_WORKGROUP_SIZE_Y);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // Submit all passes as a single batch to the GPU
        self.engine.queue.submit(std::iter::once(encoder.finish()));

        // Extract the final result from the designated result texture
        let result_texture_id = self
            .executable_pipeline
            .get_result_texture_id()
            .ok_or_else(|| "No RESULT texture found in pipeline analysis".to_string())?;
        let result_texture = self
            .physical_textures
            .get(&result_texture_id)
            .ok_or(format!("Result texture with ID {} not found", result_texture_id))?;
        let image = save_texture_as_image(&self.engine.device, &self.engine.queue, result_texture)?;

        // Calculate total execution time
        let elapsed = timepoint.elapsed();

        Ok((image, elapsed))
    }
}
