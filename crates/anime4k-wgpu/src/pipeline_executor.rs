//! Shader pipeline execution engine for Anime4K-wgpu
//!
//! This module contains the core pipeline execution logic that binds shader passes
//! to wgpu resources and executes them in sequence.

use crate::{ExecutablePipeline, executable_pipeline::SamplerFilterMode};

/// Compute shader workgroup size in X dimension
const COMPUTE_WORKGROUP_SIZE_X: u32 = 8;
/// Compute shader workgroup size in Y dimension
const COMPUTE_WORKGROUP_SIZE_Y: u32 = 8;

/// A pipeline bound to wgpu resources, ready for execution
#[derive(Debug)]
struct BoundPipeline {
    /// Collection of executable passes with their bound resources
    passes: Vec<BoundExecutablePass>,
}

/// A single executable pass bound to wgpu resources
#[derive(Debug)]
struct BoundExecutablePass {
    /// Human-readable name for debugging
    name: &'static str,
    /// Compute dispatch dimensions (width, height)
    compute_dimensions: (u32, u32),
    /// The wgpu compute pipeline
    compute_pipeline: wgpu::ComputePipeline,
    /// Bind group containing all resources for this pass
    bind_group: wgpu::BindGroup,
}

impl BoundPipeline {
    /// Creates a new bound pipeline from an executable pipeline
    ///
    /// Binds the pipeline to GPU resources and creates all necessary textures,
    /// samplers, and bind groups for execution.
    ///
    /// # Arguments
    /// * `pipeline` - The executable pipeline to bind
    /// * `device` - The wgpu device for resource creation
    /// * `input_texture` - The source texture for the pipeline
    ///
    /// # Returns
    /// A tuple of (bound pipeline, final output texture)
    pub fn new(pipeline: &'static ExecutablePipeline, device: &wgpu::Device, input_texture: &wgpu::Texture) -> (Self, wgpu::Texture) {
        let input_size = (input_texture.width(), input_texture.height());

        let physical_texture_map = pipeline
            .textures
            .iter()
            .map(|pt| {
                let texture = if pt.is_source {
                    // Use the input texture directly for source textures
                    input_texture.clone()
                } else {
                    device.create_texture(&wgpu::TextureDescriptor {
                        label: Some(&format!("Physical Texture {}", pt.id)),
                        size: wgpu::Extent3d {
                            width: (input_size.0 as f64 * pt.scale_factor.0.numerator as f64 / pt.scale_factor.0.denominator as f64) as u32,
                            height: (input_size.1 as f64 * pt.scale_factor.1.numerator as f64 / pt.scale_factor.1.denominator as f64) as u32,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: match pt.components {
                            1 => wgpu::TextureFormat::R32Float,
                            2 => wgpu::TextureFormat::Rg32Float,
                            _ => wgpu::TextureFormat::Rgba32Float,
                        },
                        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    })
                };
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                (pt.id, (texture, texture_view))
            })
            .collect::<std::collections::HashMap<_, _>>();

        let sampler_map = pipeline
            .samplers
            .iter()
            .map(|filter_mode| {
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some(&format!("Sampler {filter_mode:?}")),
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: match filter_mode {
                        SamplerFilterMode::Nearest => wgpu::FilterMode::Nearest,
                        SamplerFilterMode::Linear => wgpu::FilterMode::Linear,
                    },
                    min_filter: match filter_mode {
                        SamplerFilterMode::Nearest => wgpu::FilterMode::Nearest,
                        SamplerFilterMode::Linear => wgpu::FilterMode::Linear,
                    },
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    lod_min_clamp: 0.0,
                    lod_max_clamp: 0.0,
                    compare: None,
                    anisotropy_clamp: 1,
                    border_color: None,
                });
                (filter_mode.clone(), sampler)
            })
            .collect::<std::collections::HashMap<_, _>>();

        let mut passes = Vec::new();

        for shader_pass in pipeline.passes.iter() {
            let compute_dimensions = (
                (input_size.0 as f64 * shader_pass.compute_scale_factors.0).floor() as u32,
                (input_size.1 as f64 * shader_pass.compute_scale_factors.1).floor() as u32,
            );
            let skip_bound_check = compute_dimensions.0 % COMPUTE_WORKGROUP_SIZE_X == 0 && compute_dimensions.1 % COMPUTE_WORKGROUP_SIZE_Y == 0;

            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(shader_pass.name),
                source: wgpu::ShaderSource::Wgsl(shader_pass.shader.into()),
            });

            // Create explicit bind group layout based on the pass requirements
            let mut bind_group_layout_entries = Vec::new();

            // Add input texture bindings
            for input in shader_pass.input_textures {
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

            // Add output texture bindings
            for output in shader_pass.output_textures {
                let storage_format = physical_texture_map.get(&output.physical_texture_id).unwrap().0.format();
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

            // Add sampler bindings
            for sampler in shader_pass.samplers {
                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: sampler.binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }

            // Sort by binding number
            bind_group_layout_entries.sort_by_key(|entry| entry.binding);

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(shader_pass.name),
                entries: &bind_group_layout_entries,
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(shader_pass.name),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            // Create compute pipeline with explicit layout
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(shader_pass.name),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: if skip_bound_check { Some("main_unchecked") } else { Some("main") },
                compilation_options: Default::default(),
                cache: None,
            });

            // Create bind group using the analyzed texture bindings
            let mut bind_group_entries = Vec::new();

            for input in shader_pass.input_textures {
                let (_, texture_view) = physical_texture_map.get(&input.physical_texture_id).unwrap();
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: input.binding,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                });
            }

            for output in shader_pass.output_textures {
                let (_, texture_view) = physical_texture_map.get(&output.physical_texture_id).unwrap();
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: output.binding,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                });
            }

            for sampler in shader_pass.samplers {
                let sampler_resource = sampler_map.get(&sampler.filter_mode).unwrap();
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: sampler.binding,
                    resource: wgpu::BindingResource::Sampler(sampler_resource),
                });
            }

            bind_group_entries.sort_by_key(|entry| entry.binding);

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(shader_pass.name),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &bind_group_entries,
            });

            passes.push(BoundExecutablePass {
                name: shader_pass.name,
                compute_dimensions,
                compute_pipeline: pipeline,
                bind_group,
            });
        }

        let output_texture = physical_texture_map
            .get(&pipeline.passes.last().unwrap().output_textures.first().unwrap().physical_texture_id)
            .unwrap()
            .0
            .clone();

        (BoundPipeline { passes }, output_texture)
    }

    /// Executes all passes in this pipeline
    ///
    /// # Arguments
    /// * `encoder` - The command encoder to record commands into
    pub fn pass(&self, encoder: &mut wgpu::CommandEncoder) {
        for pass in self.passes.iter() {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(pass.name),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pass.compute_pipeline);
            compute_pass.set_bind_group(0, &pass.bind_group, &[]);

            let (compute_width, compute_height) = pass.compute_dimensions;
            let workgroup_x = compute_width.div_ceil(COMPUTE_WORKGROUP_SIZE_X);
            let workgroup_y = compute_height.div_ceil(COMPUTE_WORKGROUP_SIZE_Y);

            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }
    }
}

/// A complete shader pipeline consisting of multiple bound pipelines
///
/// Manages the execution of a series of shader pipelines in sequence,
/// handling resource management and intermediate texture passing.
#[derive(Debug)]
pub struct PipelineExecutor {
    /// Collection of bound pipelines to execute in sequence
    bound_pipelines: Vec<BoundPipeline>,
}

impl PipelineExecutor {
    /// Creates a new shader pipeline from executable pipelines
    ///
    /// Binds all pipelines to GPU resources and chains them together so that
    /// the output of one pipeline becomes the input of the next.
    ///
    /// # Arguments
    /// * `executable_pipeline` - Array of executable pipelines to chain together
    /// * `device` - The wgpu device for resource creation
    /// * `source_texture` - The initial input texture
    ///
    /// # Returns
    /// A tuple of (pipeline executor, final output texture)
    pub fn new(executable_pipeline: &[&'static ExecutablePipeline], device: &wgpu::Device, source_texture: &wgpu::Texture) -> (Self, wgpu::Texture) {
        let mut bound_pipelines = Vec::new();
        let mut current_input_texture = source_texture.clone();

        for pipeline in executable_pipeline {
            let (bound_pipeline, output_texture) = BoundPipeline::new(pipeline, device, &current_input_texture);
            current_input_texture = output_texture;

            bound_pipelines.push(bound_pipeline);
        }

        (Self { bound_pipelines }, current_input_texture)
    }

    /// Executes the entire shader pipeline
    ///
    /// # Arguments
    /// * `encoder` - The command encoder to record commands into
    pub fn pass(&self, encoder: &mut wgpu::CommandEncoder) {
        for bound_pipeline in &self.bound_pipelines {
            bound_pipeline.pass(encoder);
        }
    }
}
