//! Web video player using WebGPU and HTML5 Canvas
//!
//! Simplified implementation focusing on Anime4K preset controls and video handling.
//! Surface creation will be implemented when WebGPU canvas support is more mature.

use anime4k_wgpu::PipelineExecutor;
use anime4k_wgpu::presets::{Anime4KPerformancePreset, Anime4KPreset};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};
use wgpu::*;

/// WebGPU context for video processing
struct WebGPUContext {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    anime4k_executor: Option<PipelineExecutor>,
    anime4k_output_texture: Option<Texture>,
    anime4k_source_texture: Option<Texture>, // Source texture for pipeline input
    display_pipeline: RenderPipeline,
    display_bind_group_layout: BindGroupLayout, // Restored for texture binding
}

/// Web-based video player with Anime4K processing
pub struct VideoPlayer {
    canvas: HtmlCanvasElement,
    video_element: Option<HtmlVideoElement>,
    webgpu_context: Option<WebGPUContext>,

    // Settings
    current_preset: Anime4KPreset,
    current_performance_preset: Anime4KPerformancePreset,

    // Dimensions
    canvas_size: (u32, u32),
    video_size: Option<(u32, u32)>,

    // Frame management
    frame_count: u32,
}

impl VideoPlayer {
    /// Creates a new video player instance
    pub async fn new(canvas: HtmlCanvasElement) -> Self {
        let canvas_size = (canvas.width(), canvas.height());

        web_sys::console::log_1(&"VideoPlayer initialized".into());

        let mut player = Self {
            canvas,
            video_element: None,
            webgpu_context: None,
            current_preset: Anime4KPreset::Off,
            current_performance_preset: Anime4KPerformancePreset::Light,
            canvas_size,
            video_size: None,
            frame_count: 0,
        };

        // Initialize WebGPU
        web_sys::console::log_1(&"Attempting WebGPU initialization...".into());
        if let Err(e) = player.init_webgpu().await {
            web_sys::console::error_1(&format!("Failed to initialize WebGPU: {}", e).into());
            web_sys::console::log_1(&"Will use Canvas 2D fallback rendering".into());
        } else {
            web_sys::console::log_1(&"WebGPU initialization completed successfully".into());
        }

        player
    }

    /// Loads a video from URL
    pub fn load_video(&mut self, video_url: &str) {
        let document = web_sys::window().unwrap().document().unwrap();
        let video = document.create_element("video").unwrap().dyn_into::<HtmlVideoElement>().unwrap();

        video.set_src(video_url);
        video.set_cross_origin(Some("anonymous"));
        video.set_muted(true); // Required for autoplay in most browsers
        video.set_loop(true);
        video.set_controls(true);

        // Set up video event handlers for metadata loading
        let video_clone = video.clone();
        let loadedmetadata_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let width = video_clone.video_width();
            let height = video_clone.video_height();
            web_sys::console::log_1(&format!("Video loaded: {}x{}", width, height).into());
        }) as Box<dyn FnMut(_)>);

        video.set_onloadedmetadata(Some(loadedmetadata_closure.as_ref().unchecked_ref()));
        loadedmetadata_closure.forget();

        // Add the video element to the page for debugging
        let container = document.get_element_by_id("video-container").unwrap_or_else(|| {
            let container = document.create_element("div").unwrap();
            container.set_id("video-container");
            document.body().unwrap().append_child(&container).unwrap();
            container
        });

        // Clear existing videos and add new one
        container.set_inner_html("");
        container.append_child(&video).unwrap();
        self.video_element = Some(video);

        web_sys::console::log_1(&"Video element created and added to DOM".into());
    }

    /// Starts video playback
    pub fn play(&mut self) {
        if let Some(ref video) = self.video_element {
            let _ = video.play();
            web_sys::console::log_1(&"Video play started".into());
        }
    }

    /// Pauses video playback
    pub fn pause(&mut self) {
        if let Some(ref video) = self.video_element {
            video.pause().unwrap();
            web_sys::console::log_1(&"Video paused".into());
        }
    }

    /// Renders the current video frame using WebGPU
    pub fn render(&mut self) {
        // Simple frame limiting - only render every 4th frame to reduce load
        self.frame_count += 1;
        if self.frame_count % 4 != 0 {
            return; // Skip 3 out of 4 frames
        }

        // Log every 60 rendered frames to reduce spam
        if self.frame_count % 240 == 0 {
            web_sys::console::log_1(&format!("Frame {}: Rendering...", self.frame_count).into());
        }

        if self.webgpu_context.is_none() {
            web_sys::console::log_1(&"WebGPU not initialized, using fallback".into());
            self.render_fallback();
            return;
        }

        // Try WebGPU rendering first
        if let Err(e) = self.process_video_frame() {
            // Fallback to Canvas 2D if WebGPU fails
            web_sys::console::error_1(&format!("WebGPU rendering failed: {}", e).into());
            self.render_fallback();
        }
    }

    /// Fallback rendering using Canvas 2D
    fn render_fallback(&mut self) {
        // Get canvas 2D context for simple rendering
        let context = self.canvas.get_context("2d").unwrap().unwrap().dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap();

        // Clear canvas with black background
        context.set_fill_style(&wasm_bindgen::JsValue::from("black"));
        context.fill_rect(0.0, 0.0, self.canvas_size.0 as f64, self.canvas_size.1 as f64);

        // Draw video if available
        if let Some(ref video) = self.video_element {
            if video.ready_state() >= 2 {
                let video_width = video.video_width() as f64;
                let video_height = video.video_height() as f64;
                let canvas_width = self.canvas_size.0 as f64;
                let canvas_height = self.canvas_size.1 as f64;

                // Calculate scale to fit video in canvas while maintaining aspect ratio
                let scale_x = canvas_width / video_width;
                let scale_y = canvas_height / video_height;
                let scale = scale_x.min(scale_y);

                let scaled_width = video_width * scale;
                let scaled_height = video_height * scale;
                let x = (canvas_width - scaled_width) / 2.0;
                let y = (canvas_height - scaled_height) / 2.0;

                // Draw the video frame
                let _ = context.draw_image_with_html_video_element_and_dw_and_dh(video, x, y, scaled_width, scaled_height);
            }
        }
    }

    /// Sets the Anime4K preset
    pub fn set_anime4k_preset(&mut self, preset: Anime4KPreset) {
        self.current_preset = preset;
        web_sys::console::log_1(&format!("Anime4K preset changed to: {:?}", preset).into());

        // Recreate Anime4K pipeline with new preset and current frame size
        self.recreate_pipeline_if_needed();
    }

    /// Sets the Anime4K performance preset
    pub fn set_anime4k_performance_preset(&mut self, preset: Anime4KPerformancePreset) {
        self.current_performance_preset = preset;
        web_sys::console::log_1(&format!("Anime4K performance preset changed to: {:?}", preset).into());

        // Recreate Anime4K pipeline with new performance preset and current frame size
        self.recreate_pipeline_if_needed();
    }

    /// Initialize WebGPU context
    async fn init_webgpu(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"Initializing WebGPU...".into());

        // Create WebGPU instance
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::GL | Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // Create surface from canvas
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
            .map_err(|e| format!("Failed to create surface from canvas: {:?}", e))?;

        // Request adapter
        let adapter_result = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter_result {
            Ok(adapter) => adapter,
            Err(_) => return Err("Failed to find suitable adapter".to_string()),
        };

        web_sys::console::log_1(&format!("Using adapter: {:?}", adapter.get_info()).into());

        // Request device with float32-filterable feature for Rgba32Float textures
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Anime4K Device"),
                required_features: Features::FLOAT32_FILTERABLE, // Enable filterable float32 textures
                required_limits: Limits::downlevel_webgl2_defaults(),
                memory_hints: MemoryHints::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Failed to create device: {:?}", e))?;

        // Configure surface
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: self.canvas_size.0,
            height: self.canvas_size.1,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        web_sys::console::log_1(&format!("Surface config: {}x{}, format: {:?}", surface_config.width, surface_config.height, surface_config.format).into());
        surface.configure(&device, &surface_config);

        // Create simple display shader to render Anime4K output to screen
        web_sys::console::log_1(&"Creating display shader...".into());
        let shader_source = include_str!("shaders/display.wgsl");
        web_sys::console::log_1(&format!("Shader source length: {} characters", shader_source.len()).into());

        let display_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });
        web_sys::console::log_1(&"Display shader module created successfully".into());
        // Create bind group layout for display (restored)
        let display_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Display Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create display pipeline layout (now with bind groups restored)
        let display_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Display Pipeline Layout"),
            bind_group_layouts: &[&display_bind_group_layout], // Restored bind group
            push_constant_ranges: &[],
        });

        // Create display render pipeline
        web_sys::console::log_1(&"Creating display render pipeline...".into());
        let display_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Display Render Pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: VertexState {
                module: &display_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &display_shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        web_sys::console::log_1(&format!("Display render pipeline created successfully, surface format: {:?}", surface_config.format).into());

        let mut webgpu_context = WebGPUContext {
            device,
            queue,
            surface,
            anime4k_executor: None,
            anime4k_output_texture: None,
            anime4k_source_texture: None,
            display_pipeline,
            display_bind_group_layout, // Restored
        };

        // Initialize Anime4K pipeline if preset is not Off
        if self.current_preset != Anime4KPreset::Off {
            let frame_size = self.video_size.unwrap_or((512, 512));
            if let Err(e) = Self::create_anime4k_pipeline_for_context_with_size(self.current_preset, self.current_performance_preset, frame_size, &mut webgpu_context) {
                web_sys::console::error_1(&format!("Failed to create initial Anime4K pipeline: {}", e).into());
            }
        }

        self.webgpu_context = Some(webgpu_context);

        web_sys::console::log_1(&"WebGPU initialized successfully".into());
        Ok(())
    }

    /// Creates or updates the Anime4K pipeline with current preset settings and frame size (static version)
    fn create_anime4k_pipeline_for_context_with_size(preset: Anime4KPreset, performance_preset: Anime4KPerformancePreset, frame_size: (u32, u32), webgpu: &mut WebGPUContext) -> Result<(), String> {
        if preset == Anime4KPreset::Off {
            webgpu.anime4k_executor = None;
            webgpu.anime4k_output_texture = None;
            webgpu.anime4k_source_texture = None;
            web_sys::console::log_1(&"Anime4K pipeline disabled".into());
            return Ok(());
        }

        let (width, height) = frame_size;
        web_sys::console::log_1(&format!("Creating Anime4K pipeline for {}x{} frame", width, height).into());

        // Create input texture for pipeline creation with actual video frame size
        let input_texture = webgpu.device.create_texture(&TextureDescriptor {
            label: Some("Anime4K Source Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create executable pipelines using the current presets
        let pipelines = preset.create_pipelines(performance_preset, 2.0);

        if pipelines.is_empty() {
            return Ok(());
        }

        // Create pipeline executor
        let (executor, output_texture) = PipelineExecutor::new(&pipelines, &webgpu.device, &input_texture);

        webgpu.anime4k_executor = Some(executor);
        webgpu.anime4k_output_texture = Some(output_texture);
        webgpu.anime4k_source_texture = Some(input_texture); // Store the source texture

        web_sys::console::log_1(&format!("Anime4K pipeline created with {} passes for {}x{} frame", pipelines.len(), width, height).into());
        Ok(())
    }

    /// Recreates the Anime4K pipeline if needed (when preset or video size changes)
    fn recreate_pipeline_if_needed(&mut self) {
        let frame_size = self.video_size.unwrap_or((512, 512));
        let preset = self.current_preset;
        let performance_preset = self.current_performance_preset;

        web_sys::console::log_1(&format!("Recreating pipeline for preset {:?} with frame size {}x{}", preset, frame_size.0, frame_size.1).into());

        if let Some(ref mut webgpu) = self.webgpu_context {
            if let Err(e) = Self::create_anime4k_pipeline_for_context_with_size(preset, performance_preset, frame_size, webgpu) {
                web_sys::console::error_1(&format!("Failed to recreate Anime4K pipeline: {}", e).into());
            } else {
                web_sys::console::log_1(&"Pipeline recreation completed successfully".into());
            }
        } else {
            web_sys::console::error_1(&"No WebGPU context available for pipeline recreation".into());
        }
    }

    /// Copies input texture to the pipeline's source texture for processing
    fn copy_input_to_pipeline_source(&self, encoder: &mut CommandEncoder, input_texture: &Texture, webgpu: &WebGPUContext) -> Result<(), String> {
        let Some(ref source_texture) = webgpu.anime4k_source_texture else {
            return Err("No pipeline source texture available".to_string());
        };

        // Get the dimensions of both textures
        let input_size = (input_texture.width(), input_texture.height());
        let source_size = (source_texture.width(), source_texture.height());

        web_sys::console::log_1(&format!("Copying texture: input {}x{} to source {}x{}", input_size.0, input_size.1, source_size.0, source_size.1).into());

        // Check if dimensions match exactly
        if input_size != source_size {
            web_sys::console::log_1(
                &format!(
                    "Texture size mismatch: input {}x{} vs source {}x{} - this indicates pipeline needs recreation",
                    input_size.0, input_size.1, source_size.0, source_size.1
                )
                .into(),
            );
            return Err(format!("Texture size mismatch: input {}x{} vs source {}x{}", input_size.0, input_size.1, source_size.0, source_size.1));
        }

        // Copy the input texture to the source texture
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: input_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: source_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: input_size.0,
                height: input_size.1,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    /// Process video frame with Anime4K
    fn process_video_frame(&mut self) -> Result<(), String> {
        // Check for video size changes first
        let mut size_changed = false;
        if let Some(ref video) = self.video_element {
            if video.ready_state() >= 2 {
                let current_width = video.video_width();
                let current_height = video.video_height();

                if current_width > 0 && current_height > 0 {
                    let new_size = (current_width, current_height);
                    if self.video_size != Some(new_size) {
                        web_sys::console::log_1(&format!("Video size change detected: {:?} -> {:?}", self.video_size, Some(new_size)).into());
                        self.video_size = Some(new_size);
                        size_changed = true;
                    }
                }
            }
        }

        // Recreate pipeline if size changed
        if size_changed {
            self.recreate_pipeline_if_needed();
        }

        // Take ownership of webgpu context temporarily to avoid borrowing conflicts
        let webgpu_context = self.webgpu_context.take().ok_or("WebGPU not initialized")?;

        // Create input texture for processing
        let input_texture = if let Some(ref video) = self.video_element {
            if video.ready_state() >= 2 {
                match self.create_video_texture(&webgpu_context, video) {
                    Ok(texture) => texture,
                    Err(_) => {
                        // Return context and skip this frame
                        self.webgpu_context = Some(webgpu_context);
                        return Ok(());
                    }
                }
            } else {
                // Video not ready, return context and skip this frame
                self.webgpu_context = Some(webgpu_context);
                return Ok(());
            }
        } else {
            // No video loaded, create a simple test pattern
            self.create_simple_test_texture(&webgpu_context)?
        };

        // Determine what texture to display
        let display_texture = if self.current_preset != Anime4KPreset::Off {
            // Run Anime4K pipeline if preset is enabled
            if let (Some(executor), Some(_source_texture)) = (&webgpu_context.anime4k_executor, &webgpu_context.anime4k_source_texture) {
                // Create command encoder for Anime4K processing
                let mut encoder = webgpu_context.device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Anime4K Processing Encoder"),
                });

                // Copy input texture to pipeline source if sizes match
                if let Err(e) = self.copy_input_to_pipeline_source(&mut encoder, &input_texture, &webgpu_context) {
                    web_sys::console::log_1(&format!("Failed to copy input to pipeline source: {}, using input directly", e).into());
                    input_texture
                } else {
                    // Execute Anime4K pipeline
                    executor.pass(&mut encoder);

                    // Submit processing commands
                    webgpu_context.queue.submit(std::iter::once(encoder.finish()));

                    // Use Anime4K output texture for display
                    webgpu_context.anime4k_output_texture.as_ref().unwrap().clone()
                }
            } else {
                web_sys::console::log_1(&"Anime4K pipeline not available, using input texture".into());
                input_texture
            }
        } else {
            // No Anime4K processing, use input directly
            input_texture
        };

        web_sys::console::log_1(
            &format!(
                "Using {} texture for display",
                if self.current_preset != Anime4KPreset::Off && webgpu_context.anime4k_executor.is_some() {
                    "Anime4K processed"
                } else {
                    "input"
                }
            )
            .into(),
        );

        // Get surface texture
        let output = webgpu_context.surface.get_current_texture().map_err(|e| format!("Failed to get surface texture: {:?}", e))?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());

        // Create command encoder for final display rendering
        let mut display_encoder = webgpu_context.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Display Render Encoder"),
        });

        let display_texture_view = display_texture.create_view(&TextureViewDescriptor::default());

        // Create sampler for display
        let sampler = webgpu_context.device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for display (restored)
        let bind_group = webgpu_context.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Display Bind Group"),
            layout: &webgpu_context.display_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&display_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Debug: Log which texture we're about to display (reduced frequency)
        if self.frame_count % 240 == 0 {
            web_sys::console::log_1(&"Rendering frame...".into());
        }

        // Render final result to screen
        {
            let mut render_pass = display_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Display Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), // Black clear color
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Reduced logging for performance
            if self.frame_count % 240 == 0 {
                web_sys::console::log_1(&"Display render pass executing".into());
            }

            render_pass.set_pipeline(&webgpu_context.display_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Full-screen triangle
        }

        // Submit commands
        webgpu_context.queue.submit(std::iter::once(display_encoder.finish()));
        output.present();

        // Return the context
        self.webgpu_context = Some(webgpu_context);

        Ok(())
    }

    /// Create video texture from HTML video element by extracting current frame
    fn create_video_texture(&self, webgpu: &WebGPUContext, video: &HtmlVideoElement) -> Result<Texture, String> {
        let video_width = video.video_width();
        let video_height = video.video_height();

        if video_width == 0 || video_height == 0 {
            return Err("Video has invalid dimensions".to_string());
        }

        // Check if video is actually playing and has data
        if video.ready_state() < 2 {
            return Err("Video not ready for frame extraction".to_string());
        }

        web_sys::console::log_1(&format!("Extracting video frame: {}x{}", video_width, video_height).into());

        // Create a temporary canvas to extract video frame
        let document = web_sys::window().unwrap().document().unwrap();
        let temp_canvas = document.create_element("canvas").unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

        temp_canvas.set_width(video_width);
        temp_canvas.set_height(video_height);

        let context = temp_canvas.get_context("2d").unwrap().unwrap().dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap();

        // Draw the current video frame to the canvas
        context
            .draw_image_with_html_video_element_and_dw_and_dh(video, 0.0, 0.0, video_width as f64, video_height as f64)
            .map_err(|_| "Failed to draw video frame to canvas")?;

        // Get the pixel data from the canvas
        let image_data = context
            .get_image_data(0.0, 0.0, video_width as f64, video_height as f64)
            .map_err(|_| "Failed to get image data from canvas")?;

        let pixel_data = image_data.data();

        // Convert RGBA8 pixel data to RGBA32Float format for GPU
        let mut float_data = Vec::with_capacity((video_width * video_height * 16) as usize);

        for i in (0..pixel_data.len()).step_by(4) {
            // Convert u8 (0-255) to f32 (0.0-1.0)
            let r = pixel_data[i] as f32 / 255.0;
            let g = pixel_data[i + 1] as f32 / 255.0;
            let b = pixel_data[i + 2] as f32 / 255.0;
            let a = pixel_data[i + 3] as f32 / 255.0;

            // Convert to little-endian bytes
            float_data.extend_from_slice(&r.to_le_bytes());
            float_data.extend_from_slice(&g.to_le_bytes());
            float_data.extend_from_slice(&b.to_le_bytes());
            float_data.extend_from_slice(&a.to_le_bytes());
        }

        // Create texture with correct format for Anime4K pipeline
        let texture = webgpu.device.create_texture(&TextureDescriptor {
            label: Some("Video Frame Texture"),
            size: Extent3d {
                width: video_width,
                height: video_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC, // Added COPY_SRC
            view_formats: &[],
        });

        // Upload the video frame data to the texture
        webgpu.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &float_data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(video_width * 16), // 4 components * 4 bytes per f32
                rows_per_image: Some(video_height),
            },
            Extent3d {
                width: video_width,
                height: video_height,
                depth_or_array_layers: 1,
            },
        );

        web_sys::console::log_1(&format!("Video frame extracted and uploaded to GPU: {}x{} pixels", video_width, video_height).into());
        Ok(texture)
    }

    /// Create a simple test texture when no video is loaded
    fn create_simple_test_texture(&self, webgpu: &WebGPUContext) -> Result<Texture, String> {
        let (width, height) = (512, 512);

        let texture = webgpu.device.create_texture(&TextureDescriptor {
            label: Some("Simple Test Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Create simple gradient pattern
        let mut data = Vec::with_capacity((width * height * 16) as usize);
        for y in 0..height {
            for x in 0..width {
                let r = x as f32 / width as f32;
                let g = y as f32 / height as f32;
                let b = 0.5f32;
                let a = 1.0f32;

                data.extend_from_slice(&r.to_le_bytes());
                data.extend_from_slice(&g.to_le_bytes());
                data.extend_from_slice(&b.to_le_bytes());
                data.extend_from_slice(&a.to_le_bytes());
            }
        }

        webgpu.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 16),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(texture)
    }
}
