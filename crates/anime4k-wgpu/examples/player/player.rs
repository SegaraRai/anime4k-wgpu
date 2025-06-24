//! Core video player and rendering functionality
//!
//! This module implements the main video playback engine, including frame timing,
//! Vulkan-based video decoding, GPU-accelerated YUV-to-RGB conversion, and
//! Anime4K upscaling integration.

use super::decoder::{FrameWithPts, run_decoder};
use anime4k_wgpu::{
    PipelineExecutor,
    presets::{Anime4KPerformancePreset, Anime4KPreset},
};
use std::sync::{
    Arc,
    mpsc::{self, Receiver},
};
use vk_video::{VulkanDevice, VulkanInstance};
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

/// Number of frames to buffer between decoder and renderer
///
/// This provides a small buffer to smooth out timing variations between
/// the decoder and renderer threads. A value of 3 provides good balance
/// between latency and smooth playback.
const FRAME_BUFFER_LENGTH: usize = 3;

const YUV_COMPUTE_WORKGROUP_X: u32 = 8;
const YUV_COMPUTE_WORKGROUP_Y: u32 = 8;

/// Core video player context managing playback state and rendering pipeline
///
/// `PlayerContext` serves as the central coordinator for all video playback functionality,
/// orchestrating the interaction between the window system, video decoder, and GPU renderer.
/// It maintains synchronization between frame timing and display refresh, handles user
/// interactions like pause/resume, and manages the Anime4K upscaling pipeline.
pub struct PlayerContext {
    /// Window handle for display and event management
    ///
    /// Wrapped in `Arc` to avoid lifetime issue with `wgpu::Surface`.
    window: Arc<Window>,

    /// Video playback state and frame timing controller
    ///
    /// Manages the decoder thread communication, frame buffering, pause/resume logic,
    /// and playback timing calculations. Handles synchronization between decoded
    /// frames and display refresh timing.
    playback: VideoPlayback,

    /// GPU rendering pipeline for video frames and Anime4K processing
    ///
    /// Coordinates the complete rendering stack including YUV-to-sRGB conversion,
    /// optional Anime4K upscaling, and final screen presentation. Manages GPU
    /// resources, shaders, and render passes.
    renderer: Renderer,

    /// Flag indicating whether the current frame needs to be re-rendered
    ///
    /// Set to `true` when visual changes occur (new frame, preset change, window resize)
    /// and cleared after successful rendering. Prevents unnecessary GPU work when
    /// no visual updates are required.
    needs_redraw: bool,
}

impl PlayerContext {
    /// Creates a new player context with video decoding and rendering capabilities
    ///
    /// Initializes the complete video playback pipeline including window creation,
    /// Vulkan device setup, decoder thread spawning, and renderer initialization.
    ///
    /// # Arguments
    /// * `event_loop` - The active event loop for window management
    /// * `reader` - Input stream containing the video data
    /// * `framerate` - Target playback framerate in FPS
    /// * `start_paused` - Whether to begin playback in paused state
    ///
    /// # Returns
    /// A fully initialized player context ready for frame rendering
    pub fn new(event_loop: &ActiveEventLoop, reader: impl std::io::Read + Send + 'static, framerate: u32, start_paused: bool) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_resizable(true).with_visible(false).with_title("Anime4K-wgpu Video Player"))
                .unwrap(),
        );

        // Initialize video playback and renderer
        let (playback, surface) = VideoPlayback::new(reader, framerate, start_paused, window.clone());
        let renderer = Renderer::new(surface, &playback.vulkan_device, window.clone());

        // Set initial window size based on video dimensions
        let _ = window.request_inner_size(PhysicalSize::new(playback.current_frame.frame.size().width, playback.current_frame.frame.size().height));

        let context = Self {
            window,
            playback,
            renderer,
            needs_redraw: true, // Initial render needed
        };

        context.update_window_title();
        context.window.set_visible(true);
        context.window.focus_window();
        context.window.request_redraw();

        context
    }

    /// Handles frame rendering and playback timing
    ///
    /// This method orchestrates the complete frame presentation pipeline:
    /// - Receives new frames from the decoder when not paused
    /// - Calculates current playback time excluding pause duration
    /// - Advances to the next frame when timing conditions are met
    /// - Triggers rendering of the current frame
    /// - Requests continued redraws for smooth playback
    pub fn handle_redraw(&mut self) {
        let mut frame_changed = false;

        // Only receive new frames when not paused
        if !self.playback.is_paused && self.playback.next_frame.is_none() {
            if let Ok(frame) = self.playback.rx.try_recv() {
                self.playback.next_frame = Some(frame);
            }
        }

        // Calculate current playback time, excluding pause duration
        let current_pause_duration = if self.playback.is_paused {
            if let Some(pause_time) = self.playback.pause_start_time {
                self.playback.total_pause_duration + (std::time::Instant::now() - pause_time)
            } else {
                self.playback.total_pause_duration
            }
        } else {
            self.playback.total_pause_duration
        };

        let current_pts = (std::time::Instant::now() - self.playback.start_timestamp) - current_pause_duration;

        // Advance to next frame if it's time and not paused
        if !self.playback.is_paused {
            if let Some(next_frame_pts) = self.playback.next_frame.as_ref().map(|f| f.pts) {
                if next_frame_pts < current_pts {
                    self.playback.current_frame = self.playback.next_frame.take().unwrap();
                    frame_changed = true;
                }
            }
        }

        // Only render if we need to redraw (frame changed, preset changed, or forced redraw)
        if self.needs_redraw || frame_changed {
            // Render the current frame
            self.renderer.render(&self.playback.current_frame.frame, &self.window).unwrap();
            self.needs_redraw = false;
        }

        // Continue the redraw loop only if video is playing
        if !self.playback.is_paused {
            self.window.request_redraw();
        }
    }

    /// Handles window resize events by updating renderer and requesting redraw
    ///
    /// # Arguments
    /// * `new_size` - The new window dimensions
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        // Resize the renderer and request redraw
        self.renderer.resize(new_size);
        self.update_window_title();
        self.request_redraw();
    }

    /// Sets the active Anime4K upscaling preset
    ///
    /// Changes the Anime4K processing mode and updates the window title.
    /// Skips processing if the preset is unchanged.
    ///
    /// # Arguments
    /// * `preset` - The Anime4K preset to activate
    pub fn set_anime4k_preset(&mut self, preset: Anime4KPreset) {
        if self.renderer.get_current_preset() == preset {
            return;
        }

        if preset == Anime4KPreset::Off {
            tracing::info!("Anime4K disabled");
        } else {
            tracing::info!(
                "Anime4K preset set to: {} (Anime4K performance preset is {})",
                preset.name(),
                self.renderer.get_current_performance_preset().name()
            );
        }

        self.renderer.set_anime4k_preset(preset);
        self.update_window_title();
        self.request_redraw();
    }

    /// Sets the active Anime4K performance preset
    ///
    /// Adjusts the computational complexity vs. quality trade-off.
    /// Skips processing if the preset is unchanged.
    ///
    /// # Arguments
    /// * `performance_preset` - The performance preset to activate
    pub fn set_anime4k_performance_preset(&mut self, performance_preset: Anime4KPerformancePreset) {
        if self.renderer.get_current_performance_preset() == performance_preset {
            return;
        }

        tracing::info!(
            "Anime4K performance preset set to: {} (Anime4K preset is {})",
            performance_preset.name(),
            self.renderer.get_current_preset().name()
        );

        self.renderer.set_anime4k_performance_preset(performance_preset);
        self.update_window_title();
        self.request_redraw();
    }

    /// Returns whether video playback is currently paused
    pub fn is_paused(&self) -> bool {
        self.playback.is_paused
    }

    /// Pauses video playback and records the pause timestamp
    ///
    /// Records the current time to accurately calculate pause duration
    /// for proper frame timing when playback resumes.
    pub fn pause(&mut self) {
        if self.playback.is_paused {
            return;
        }

        // Record when pause started for timing calculations
        self.playback.pause_start_time = Some(std::time::Instant::now());

        self.playback.is_paused = true;

        self.update_window_title();

        tracing::info!("Video paused");
    }

    /// Resumes video playback and updates timing calculations
    ///
    /// Accumulates the total pause duration to maintain proper frame
    /// timing throughout the video playback session.
    pub fn resume(&mut self) {
        if !self.playback.is_paused {
            return;
        }

        // Accumulate total pause duration when resuming
        if let Some(pause_time) = self.playback.pause_start_time {
            self.playback.total_pause_duration += std::time::Instant::now() - pause_time;
        }
        self.playback.pause_start_time = None;

        self.playback.is_paused = false;

        self.update_window_title();
        self.request_redraw();

        tracing::info!("Video resumed");
    }

    /// Request a redraw and mark that we need to re-render
    fn request_redraw(&mut self) {
        self.needs_redraw = true;
        self.window.request_redraw();
    }

    /// Updates the window title to reflect current Anime4K settings and pause state
    fn update_window_title(&self) {
        let preset_text = if self.renderer.get_current_preset() == Anime4KPreset::Off {
            "OFF"
        } else {
            &format!("{} {}", self.renderer.get_current_preset().name(), self.renderer.get_current_performance_preset().name())
        };

        let window_title = format!("Anime4K-wgpu Video Player [Anime4K {preset_text}]{}", if self.playback.is_paused { " [PAUSED]" } else { "" });
        self.window.set_title(&window_title);
    }
}

/// Video playback state and timing management
///
/// Manages the timing and synchronization of video frame presentation,
/// coordinating between the decoder thread and the main rendering loop.
struct VideoPlayback {
    /// Shared Vulkan device for hardware video decoding
    vulkan_device: Arc<VulkanDevice>,

    /// Channel receiver for frames from the decoder thread
    rx: Receiver<FrameWithPts>,
    /// The currently displayed frame
    current_frame: FrameWithPts,
    /// The next frame waiting to be displayed
    next_frame: Option<FrameWithPts>,

    /// Timestamp when video playback started
    start_timestamp: std::time::Instant,
    /// Current pause state
    is_paused: bool,
    /// Timestamp when the current pause began (if paused)
    pause_start_time: Option<std::time::Instant>,
    /// Total accumulated pause time for timing calculations
    total_pause_duration: std::time::Duration,
}

impl VideoPlayback {
    /// Creates a new video playback state and initializes hardware decoding
    ///
    /// Sets up the complete video processing pipeline including Vulkan device creation,
    /// decoder thread spawning, and initial frame reception. Creates the wgpu surface
    /// for rendering output.
    ///
    /// # Arguments
    /// * `reader` - Input stream containing video data
    /// * `framerate` - Target playback framerate
    /// * `start_paused` - Whether to begin in paused state
    /// * `window` - Window handle for surface creation
    ///
    /// # Returns
    /// A tuple containing the initialized playback state and wgpu surface
    pub fn new(reader: impl std::io::Read + Send + 'static, framerate: u32, start_paused: bool, window: Arc<Window>) -> (Self, wgpu::Surface<'static>) {
        // Initialize Vulkan instance for video decoding and graphics
        let vulkan_instance = VulkanInstance::new().unwrap();

        // Create wgpu surface for rendering to the window
        let surface = vulkan_instance.wgpu_instance().create_surface(window).unwrap();

        // Create Vulkan device with required features for video and graphics
        let vulkan_device = vulkan_instance.create_device(wgpu::Features::FLOAT32_FILTERABLE, wgpu::Limits::default(), Some(&surface)).unwrap();

        // Create a bounded channel for frame communication between threads
        let (tx, rx) = mpsc::sync_channel(FRAME_BUFFER_LENGTH);
        let vulkan_device_clone = vulkan_device.clone();

        // Spawn decoder thread for hardware video decoding
        std::thread::spawn(move || {
            run_decoder(tx, framerate, vulkan_device_clone, reader);
        });

        let initial_frame = rx.recv().unwrap();
        let start_timestamp = std::time::Instant::now();

        (
            Self {
                vulkan_device,

                rx,
                current_frame: initial_frame,
                next_frame: None,

                start_timestamp,
                is_paused: start_paused,
                pause_start_time: if start_paused { Some(start_timestamp) } else { None },
                total_pause_duration: std::time::Duration::ZERO,
            },
            surface,
        )
    }
}

/// Vertex data structure for rendering geometry
///
/// Represents a single vertex with 3D position and 2D texture coordinates.
/// Used for rendering full-screen quads in both YUV-to-RGB and RGB-to-screen passes.
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct Vertex {
    /// 3D position coordinates (x, y, z)
    position: [f32; 3],
    /// 2D texture coordinates (u, v)
    texture_coords: [f32; 2],
}

/// Uniform buffer data for scale and offset transformations
///
/// Used in the final rendering pass to scale and position the video content
/// within the window, maintaining aspect ratio and centering the image.
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct ScaleUniforms {
    /// Scale factors for width and height
    scale: [f32; 2],
    /// Offset values for centering (currently unused, always [0,0])
    offset: [f32; 2],
}

impl Vertex {
    /// Vertex attribute descriptors for wgpu
    const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    /// Complete vertex buffer layout descriptor
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: Self::ATTRIBUTES,
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
    };
}

/// Full-screen quad vertices in normalized device coordinates
///
/// Creates a quad that covers the entire screen from (-1,-1) to (1,1)
/// with texture coordinates from (0,0) to (1,1).
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, 1.0, 0.0], // Top-left
        texture_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0], // Bottom-left
        texture_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0], // Bottom-right
        texture_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0], // Top-right
        texture_coords: [1.0, 0.0],
    },
];

/// Triangle indices for the full-screen quad
///
/// Defines two triangles that form a complete quad using the vertices above.
const INDICES: &[u16] = &[0, 1, 3, 1, 2, 3];

/// Background color for areas not covered by video content
const BACKGROUND_COLOR: wgpu::Color = wgpu::Color::BLACK;

/// Main renderer structure managing the complete video rendering pipeline
///
/// Handles three main rendering stages:
/// 1. YUV to sRGB conversion from decoded video frames
/// 2. Optional Anime4K upscaling processing
/// 3. Final scaling and rendering to screen
struct Renderer {
    // Core wgpu resources
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_configuration: wgpu::SurfaceConfiguration,

    // YUV to sRGB conversion pipeline resources
    yuv_sampler: wgpu::Sampler,
    yuv_pipeline: wgpu::ComputePipeline,

    // sRGB to Screen rendering pipeline resources
    rgb_sampler: wgpu::Sampler,
    rgb_vertex_buffer: wgpu::Buffer,
    rgb_index_buffer: wgpu::Buffer,
    rgb_uniform_buffer: wgpu::Buffer,
    rgb_pipeline: wgpu::RenderPipeline,

    // Intermediate sRGB texture between YUV conversion and Anime4K processing
    rgb_texture: Option<wgpu::Texture>,

    // Anime4K upscaling pipeline and its output texture
    anime4k_pipeline: Option<(PipelineExecutor, wgpu::Texture)>,
    current_preset: Anime4KPreset,
    current_performance_preset: Anime4KPerformancePreset,

    // Video dimensions for pipeline setup
    video_dimensions: (u32, u32),
}

impl Renderer {
    /// Creates a new renderer instance with all necessary GPU resources
    ///
    /// Initializes the complete rendering pipeline including YUV-to-RGB conversion,
    /// Anime4K processing capability, and final screen rendering. Sets up all
    /// shaders, buffers, samplers, and pipeline states.
    ///
    /// # Arguments
    /// * `surface` - The wgpu surface to render to
    /// * `vulkan_device` - Vulkan device wrapper for GPU access
    /// * `window` - The window being rendered to
    ///
    /// # Returns
    /// A fully initialized renderer ready for frame rendering
    fn new(surface: wgpu::Surface<'static>, vulkan_device: &VulkanDevice, window: Arc<Window>) -> Self {
        // Get wgpu device and queue from Vulkan wrapper
        let device = vulkan_device.wgpu_device();
        let queue = vulkan_device.wgpu_queue();
        let size = window.inner_size();

        // Configure surface for rendering
        let surface_capabilities = surface.get_capabilities(&vulkan_device.wgpu_adapter());
        let surface_texture_format = surface_capabilities.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_capabilities.formats[0]);

        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            width: size.width,
            height: size.height,
            format: surface_texture_format,
            view_formats: vec![surface_texture_format, surface_texture_format.remove_srgb_suffix()],
            alpha_mode: surface_capabilities.alpha_modes[0],
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_configuration);

        // Create shared vertex and index buffers for full-screen quad rendering
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(VERTICES),
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(INDICES),
        });

        // Set up YUV to sRGB conversion pipeline
        let yuv_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("YUV bind group layout"),
            entries: &[
                // Y plane texture (luminance)
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                },
                // UV plane texture (chrominance)
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                },
                // Texture sampler for UV plane
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                },
                // Output texture
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                },
            ],
        });

        // Create linear sampler for YUV texture sampling
        let yuv_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("YUV sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create YUV conversion pipeline layout and shaders
        let yuv_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("YUV pipeline layout"),
            bind_group_layouts: &[&yuv_bind_group_layout],
            push_constant_ranges: &[],
        });

        let yuv_shader_module = device.create_shader_module(wgpu::include_wgsl!("yuv_to_srgb.wgsl"));

        // Create YUV to sRGB compute pipeline
        let yuv_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("YUV to sRGB compute pipeline"),
            layout: Some(&yuv_pipeline_layout),
            module: &yuv_shader_module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

        // Set up RGB to Screen rendering pipeline
        let rgb_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RGB bind group layout"),
            entries: &[
                // RGB texture input (from YUV conversion or Anime4K output)
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
                // Scale and offset uniform buffer for aspect ratio correction
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                },
                // Texture sampler for final rendering
                wgpu::BindGroupLayoutEntry {
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
            ],
        });

        // Create linear sampler for final RGB rendering
        let rgb_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RGB sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create uniform buffer for scale and offset values
        let rgb_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RGB scale uniform buffer"),
            size: std::mem::size_of::<ScaleUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create RGB rendering pipeline layout and shaders
        let rgb_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RGB pipeline layout"),
            bind_group_layouts: &[&rgb_bind_group_layout],
            push_constant_ranges: &[],
        });

        let rgb_shader_module = device.create_shader_module(wgpu::include_wgsl!("srgb_to_screen.wgsl"));

        // Create RGB to screen render pipeline
        let rgb_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sRGB to Screen pipeline"),
            layout: Some(&rgb_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &rgb_shader_module,
                buffers: &[Vertex::LAYOUT],
                compilation_options: Default::default(),
                entry_point: None,
            },
            fragment: Some(wgpu::FragmentState {
                module: &rgb_shader_module,
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format.remove_srgb_suffix(), // Linear format for proper color space handling
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
                entry_point: None,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            depth_stencil: None,
        });

        Self {
            surface,
            device,
            queue,
            surface_configuration,
            yuv_sampler,
            yuv_pipeline,
            rgb_sampler,
            rgb_vertex_buffer: vertex_buffer,
            rgb_index_buffer: index_buffer,
            rgb_uniform_buffer,
            rgb_pipeline,
            rgb_texture: None,
            anime4k_pipeline: None,
            current_preset: Anime4KPreset::Off,
            current_performance_preset: Anime4KPerformancePreset::Light,
            video_dimensions: (size.width, size.height),
        }
    }

    /// Handles window resize events
    ///
    /// Updates the surface configuration and recreates the Anime4K pipeline
    /// with new target dimensions to maintain proper scaling.
    ///
    /// # Arguments
    /// * `size` - New window size in physical pixels
    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            // Update surface configuration with new dimensions
            self.surface_configuration.width = size.width;
            self.surface_configuration.height = size.height;
            self.surface.configure(&self.device, &self.surface_configuration);

            // Recreate Anime4K pipeline with new target dimensions
            self.update_anime4k_pipeline(self.video_dimensions.0, self.video_dimensions.1);
        }
    }

    /// Creates an intermediate RGB texture for YUV conversion output
    ///
    /// This texture serves as the input to the Anime4K pipeline and uses
    /// high precision RGBA32Float format to preserve image quality.
    ///
    /// # Arguments
    /// * `width` - Texture width in pixels
    /// * `height` - Texture height in pixels
    fn create_rgb_texture(&mut self, width: u32, height: u32) {
        self.rgb_texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RGB intermediate texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float, // High precision for Anime4K processing
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }));
    }

    /// Updates or creates the Anime4K processing pipeline for new video dimensions
    ///
    /// Reconfigures the Anime4K shader pipeline when video dimensions change or when
    /// switching between presets. Calculates appropriate scale factors and creates
    /// optimized GPU pipelines for the current video resolution and target window size.
    fn update_anime4k_pipeline(&mut self, video_width: u32, video_height: u32) {
        tracing::debug!(
            "Setting up Anime4K for {video_width}x{video_height} (target={}x{}), current_preset={}, anime4k_pipeline={}",
            self.surface_configuration.width,
            self.surface_configuration.height,
            self.current_preset.name(),
            self.anime4k_pipeline.is_some(),
        );

        // Disable pipeline if Anime4K is turned off
        if self.current_preset == Anime4KPreset::Off {
            self.anime4k_pipeline = None;
            return;
        }

        let target_width = self.surface_configuration.width;
        let target_height = self.surface_configuration.height;

        if let Some(rgb_texture) = &self.rgb_texture {
            // Calculate target scale factor to fit video in window
            let target_scale_factor = (target_width as f64 / video_width as f64).max(target_height as f64 / video_height as f64);

            // Create Anime4K pipelines with appropriate settings
            let pipelines = self.current_preset.create_pipelines(self.current_performance_preset, target_scale_factor);

            // Initialize the Anime4K shader pipeline
            let (pipeline, output_texture) = PipelineExecutor::new(&pipelines, &self.device, rgb_texture);

            self.anime4k_pipeline = Some((pipeline, output_texture));
        }
    }

    /// Calculates scale and offset values for aspect ratio-preserving video display
    ///
    /// Computes the scale factors needed to fit the video within the window
    /// while maintaining aspect ratio and centering the image.
    ///
    /// # Arguments
    /// * `video_width` - Video width in pixels
    /// * `video_height` - Video height in pixels
    ///
    /// # Returns
    /// Scale uniform data for the final rendering pass
    fn calculate_scale_and_offset(&self, video_width: u32, video_height: u32) -> ScaleUniforms {
        let window_width = self.surface_configuration.width as f32;
        let window_height = self.surface_configuration.height as f32;
        let video_width = video_width as f32;
        let video_height = video_height as f32;

        let window_aspect = window_width / window_height;
        let video_aspect = video_width / video_height;

        let (scale_x, scale_y) = if video_aspect > window_aspect {
            // Video is wider than window, fit to width
            let scale = window_width / video_width;
            (1.0, (video_height * scale) / window_height)
        } else {
            // Video is taller than window, fit to height
            let scale = window_height / video_height;
            ((video_width * scale) / window_width, 1.0)
        };

        ScaleUniforms {
            scale: [scale_x, scale_y],
            offset: [0.0, 0.0], // Center the video (offset currently unused in shader)
        }
    }

    /// Renders a complete frame through the three-stage pipeline
    ///
    /// Executes the full rendering pipeline:
    /// 1. Converts YUV420 frame to sRGB in intermediate texture
    /// 2. Optionally applies Anime4K upscaling if enabled
    /// 3. Renders final result to screen with proper scaling and aspect ratio
    ///
    /// # Arguments
    /// * `frame` - The YUV420 video frame texture to render
    /// * `window` - The window being rendered to
    ///
    /// # Returns
    /// Result indicating rendering success or surface error
    fn render(&mut self, frame: &wgpu::Texture, window: &Window) -> Result<(), wgpu::SurfaceError> {
        let video_width = frame.width();
        let video_height = frame.height();

        // Create or update RGB intermediate texture if needed
        if self.rgb_texture.is_none() || self.video_dimensions != (video_width, video_height) {
            self.create_rgb_texture(video_width, video_height);
            self.video_dimensions = (video_width, video_height);

            // Update Anime4K pipeline with new video dimensions
            self.update_anime4k_pipeline(video_width, video_height);
        }

        let device = &self.device;
        let surface = self.surface.get_current_texture()?;
        let surface_view = surface.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(surface.texture.format().remove_srgb_suffix()), // Linear format for proper color handling
            ..Default::default()
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        // Stage 1: Convert YUV420 to sRGB
        if let Some(rgb_texture) = &self.rgb_texture {
            let rgb_texture_view = rgb_texture.create_view(&Default::default());

            // Create bind group for YUV input textures, sampler, and output texture
            let yuv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("YUV bind group"),
                layout: &self.yuv_pipeline.get_bind_group_layout(0),
                entries: &[
                    // Y plane (luminance) as single-component texture
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&frame.create_view(&wgpu::TextureViewDescriptor {
                            label: Some("Y texture"),
                            format: Some(wgpu::TextureFormat::R8Unorm),
                            aspect: wgpu::TextureAspect::Plane0,
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            ..Default::default()
                        })),
                    },
                    // UV plane (chrominance) as two-component texture
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&frame.create_view(&wgpu::TextureViewDescriptor {
                            label: Some("UV texture"),
                            format: Some(wgpu::TextureFormat::Rg8Unorm),
                            aspect: wgpu::TextureAspect::Plane1,
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            ..Default::default()
                        })),
                    },
                    // Linear sampler for UV texture sampling
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.yuv_sampler),
                    },
                    // Output RGB texture
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&rgb_texture_view),
                    },
                ],
            });

            // Execute YUV to sRGB conversion compute pass
            {
                let mut yuv_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("YUV to sRGB compute pass"),
                    timestamp_writes: None,
                });

                yuv_pass.set_pipeline(&self.yuv_pipeline);
                yuv_pass.set_bind_group(0, &yuv_bind_group, &[]);
                yuv_pass.dispatch_workgroups(video_width.div_ceil(YUV_COMPUTE_WORKGROUP_X), video_height.div_ceil(YUV_COMPUTE_WORKGROUP_Y), 1);
            }

            // Stage 2: Apply Anime4K processing if enabled
            let texture_to_render = if let Some((ref pipeline, ref output_texture)) = self.anime4k_pipeline {
                // Execute Anime4K compute shaders
                pipeline.pass(&mut command_encoder);
                output_texture
            } else {
                // Use original RGB texture without Anime4K processing
                rgb_texture
            };

            // Stage 3: Render final result to screen with proper scaling
            let final_width = texture_to_render.width();
            let final_height = texture_to_render.height();
            let scale_uniforms = self.calculate_scale_and_offset(final_width, final_height);

            // Update uniform buffer with current scale values
            self.queue.write_buffer(&self.rgb_uniform_buffer, 0, bytemuck::cast_slice(&[scale_uniforms]));

            // Create bind group for final rendering pass
            let rgb_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RGB bind group"),
                layout: &self.rgb_pipeline.get_bind_group_layout(0),
                entries: &[
                    // Final texture to render (RGB or Anime4K output)
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_to_render.create_view(&Default::default())),
                    },
                    // Scale and offset uniforms
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.rgb_uniform_buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                    // Linear sampler for final rendering
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.rgb_sampler),
                    },
                ],
            });

            // Execute final render pass to screen
            {
                let mut rgb_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sRGB to screen pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

                rgb_pass.set_pipeline(&self.rgb_pipeline);
                rgb_pass.set_bind_group(0, &rgb_bind_group, &[]);
                rgb_pass.set_vertex_buffer(0, self.rgb_vertex_buffer.slice(..));
                rgb_pass.set_index_buffer(self.rgb_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rgb_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
            }
        }

        // Submit all commands to GPU and present the frame
        self.queue.submit(Some(command_encoder.finish()));
        window.pre_present_notify();
        surface.present();

        Ok(())
    }

    /// Sets the current Anime4K preset and updates the pipeline
    ///
    /// Changes the Anime4K processing mode and recreates the shader pipeline
    /// with the new settings. Updates console output to show current state.
    ///
    /// # Arguments
    /// * `preset` - The new Anime4K preset to use
    pub fn set_anime4k_preset(&mut self, preset: Anime4KPreset) {
        if self.current_preset == preset {
            return;
        }

        self.current_preset = preset;

        // Recreate pipeline with new preset
        self.update_anime4k_pipeline(self.video_dimensions.0, self.video_dimensions.1);
    }

    /// Sets the current Anime4K performance preset and updates the pipeline
    ///
    /// Changes the computational complexity/quality trade-off and recreates
    /// the shader pipeline with the new performance settings.
    ///
    /// # Arguments
    /// * `performance_preset` - The new performance preset to use
    pub fn set_anime4k_performance_preset(&mut self, performance_preset: Anime4KPerformancePreset) {
        if self.current_performance_preset == performance_preset {
            return;
        }

        self.current_performance_preset = performance_preset;

        // Recreate pipeline with new performance preset
        self.update_anime4k_pipeline(self.video_dimensions.0, self.video_dimensions.1);
    }

    /// Returns the current Anime4K preset
    pub fn get_current_preset(&self) -> Anime4KPreset {
        self.current_preset
    }

    /// Returns the current Anime4K performance preset
    pub fn get_current_performance_preset(&self) -> Anime4KPerformancePreset {
        self.current_performance_preset
    }
}
