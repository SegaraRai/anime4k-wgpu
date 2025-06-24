//! GLSL reference implementation engine
//!
//! This module provides a reference implementation engine that processes
//! original GLSL shaders to generate reference output for verification.

use crate::wgpu_helpers::*;
use anime4k_wgpu_build::pipelines::SamplerFilterMode;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Workgroup size for compute shaders (X dimension)
const COMPUTE_WORKGROUP_SIZE_X: u32 = 8;
/// Workgroup size for compute shaders (Y dimension)
const COMPUTE_WORKGROUP_SIZE_Y: u32 = 8;
/// Default number of color components
const DEFAULT_COMPONENTS: u32 = 4;

/// Calculates the number of workgroups needed for a given size
fn calculate_workgroup_count(size: u32, workgroup_size: u32) -> u32 {
    size.div_ceil(workgroup_size)
}

/// Represents a parsed mpv-style GLSL hook
#[derive(Debug, Clone)]
pub struct MpvHook {
    /// Description from DESC directive
    desc: String,
    /// Hook point from HOOK directive
    hook: String,
    /// Input texture names from BIND directives
    bind: Vec<String>,
    /// Output texture name from SAVE directive
    save: Option<String>,
    /// Width expression from WIDTH directive
    width: Option<String>,
    /// Height expression from HEIGHT directive
    height: Option<String>,
    /// Number of components from COMPONENTS directive
    components: Option<u32>,
    /// Condition from WHEN directive
    when: Option<String>,
    /// The GLSL shader code
    glsl_code: String,
}

/// GLSL reference engine for generating reference output
///
/// Processes original GLSL shaders and executes them to generate
/// reference images for verification against WGSL implementations.
pub struct GlslReferenceEngine {
    /// The wgpu device
    device: wgpu::Device,
    /// The wgpu command queue
    queue: wgpu::Queue,
    /// Cache of compiled shader modules
    shader_cache: HashMap<String, wgpu::ShaderModule>,
}

/// Image processor that manages texture state during pipeline execution
pub struct ImageProcessor {
    /// The underlying GLSL reference engine
    engine: GlslReferenceEngine,
    /// Cache of intermediate textures created during processing
    intermediate_textures: HashMap<String, wgpu::Texture>,
}

impl MpvHook {
    /// Parses mpv hooks from GLSL source code
    ///
    /// # Arguments
    /// * `source` - The GLSL source containing mpv hook directives
    ///
    /// # Returns
    /// A vector of parsed mpv hooks
    pub fn parse_from_glsl(source: &str) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut hooks = Vec::new();
        let mut current_hook = None;
        let mut current_code = String::new();

        // Helper function to finalize and save the current hook being parsed
        let save_current_hook = |current_hook: Option<MpvHook>, current_code: &str, hooks: &mut Vec<MpvHook>| -> Option<MpvHook> {
            if let Some(mut hook) = current_hook {
                // Set the accumulated GLSL code for this hook
                hook.glsl_code = current_code.trim().to_string();
                hooks.push(hook);
            }
            None // Always return None to clear current hook
        };

        // Helper function to create a new hook with default values
        let create_new_hook = |desc: &str| -> MpvHook {
            MpvHook {
                desc: desc.to_string(),
                hook: String::new(),      // Will be set by HOOK directive
                bind: Vec::new(),         // Will be populated by BIND directives
                save: None,               // Will be set by SAVE directive
                width: None,              // Will be set by WIDTH directive
                height: None,             // Will be set by HEIGHT directive
                components: None,         // Will be set by COMPONENTS directive
                when: None,               // Will be set by WHEN directive
                glsl_code: String::new(), // Will be accumulated from non-directive lines
            }
        };

        // Helper function to process mpv directive lines (//!DIRECTIVE value)
        // Returns true if the line was a directive, false otherwise
        let process_directive = |line: &str, current_hook: &mut Option<MpvHook>| -> bool {
            if let Some(desc) = line.strip_prefix("//!DESC ").map(str::trim) {
                *current_hook = Some(create_new_hook(desc));
                true
            } else if let Some(hook_val) = line.strip_prefix("//!HOOK ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.hook = hook_val.to_string();
                }
                true
            } else if let Some(bind_val) = line.strip_prefix("//!BIND ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.bind.push(bind_val.to_string());
                }
                true
            } else if let Some(save_val) = line.strip_prefix("//!SAVE ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.save = Some(save_val.to_string());
                }
                true
            } else if let Some(width_val) = line.strip_prefix("//!WIDTH ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.width = Some(width_val.to_string());
                }
                true
            } else if let Some(height_val) = line.strip_prefix("//!HEIGHT ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.height = Some(height_val.to_string());
                }
                true
            } else if let Some(comp_val) = line.strip_prefix("//!COMPONENTS ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.components = comp_val.parse().ok();
                }
                true
            } else if let Some(when_val) = line.strip_prefix("//!WHEN ").map(str::trim) {
                if let Some(hook) = current_hook {
                    hook.when = Some(when_val.to_string());
                }
                true
            } else {
                false
            }
        };

        // Helper function to check if line should be added to code
        let should_add_to_code = |line: &str| -> bool { !line.starts_with("//") || line.starts_with("//!") };

        for line in source.lines() {
            // Check if this is a DESC directive (starts a new hook)
            if line.starts_with("//!DESC ") {
                // Save previous hook if exists
                current_hook = save_current_hook(current_hook, &current_code, &mut hooks);
                current_code.clear();
                // Process the DESC directive
                process_directive(line, &mut current_hook);
            } else if process_directive(line, &mut current_hook) {
                // Other directive processed, continue
            } else if should_add_to_code(line) && current_hook.is_some() {
                // Regular GLSL code or special comments
                current_code.push_str(&format!("{line}\n"));
            }
        }

        // Don't forget the last hook
        save_current_hook(current_hook, &current_code, &mut hooks);

        Ok(hooks)
    }

    fn calculate_output_size(&self, available_textures: &HashMap<String, wgpu::Texture>) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        // Helper function to parse mpv dimension expressions
        // Supports formats like "MAIN.w", "HOOKED.h", "intermediate.w 2 *", "MAIN.h 2 /"
        let parse_dimension_expression = |expr: &str, available_textures: &HashMap<String, wgpu::Texture>| -> Result<u32, Box<dyn std::error::Error>> {
            // Regex to match: texture_name.dimension [factor] [operator]
            let texture_dimension_regex = Regex::new(r"(\w+)\.([wh])(?:\s+(\d+)\s*([*/]))?").unwrap();
            let captures = texture_dimension_regex.captures(expr).ok_or_else(|| format!("Invalid dimension expression: '{expr}'"))?;

            let texture_name = &captures[1];
            let dimension = &captures[2];

            let (factor, op) = if let (Some(factor), Some(op)) = (captures.get(3), captures.get(4)) {
                (factor.as_str().parse::<u32>()?, op.as_str())
            } else {
                (1, "*")
            };
            if factor == 0 {
                return Err("Factor cannot be zero in dimension expression".into());
            }

            let texture = available_textures
                .get(texture_name)
                .ok_or_else(|| format!("Texture '{texture_name}' not found in available textures"))?;

            let base_size = match dimension {
                "w" => texture.size().width,
                "h" => texture.size().height,
                _ => return Err(format!("Invalid dimension '{dimension}', expected 'w' or 'h'").into()),
            };

            let result = match op {
                "*" => base_size * factor,
                "/" => base_size / factor,
                _ => return Err(format!("Invalid operator '{op}', expected '*' or '/'").into()),
            };
            Ok(result)
        };

        // Calculate width using helper function
        let width = match &self.width {
            Some(width_expr) => parse_dimension_expression(width_expr, available_textures)?,
            None => available_textures.get("MAIN").map(|t| t.size().width).ok_or("No width expression and MAIN texture not available")?,
        };

        // Calculate height using helper function
        let height = match &self.height {
            Some(height_expr) => parse_dimension_expression(height_expr, available_textures)?,
            None => available_textures.get("MAIN").map(|t| t.size().height).ok_or("No height expression and MAIN texture not available")?,
        };

        Ok((width, height))
    }

    /// Determines the appropriate texture format based on component count
    ///
    /// # Returns
    /// The appropriate texture format for this hook's output
    fn get_output_format(&self) -> wgpu::TextureFormat {
        match self.components.unwrap_or(DEFAULT_COMPONENTS) {
            1 => wgpu::TextureFormat::R32Float,        // Single component, 32-bit float
            2 => wgpu::TextureFormat::Rg32Float,       // Two components, 32-bit float
            3 | 4 => wgpu::TextureFormat::Rgba32Float, // 3 or 4 components, 32-bit float
            _ => wgpu::TextureFormat::Rgba32Float,     // Default fallback
        }
    }

    /// Converts the GLSL hook to a GLSL compute shader
    ///
    /// # Arguments
    /// * `texture_formats` - Map of texture names to their formats for binding generation
    ///
    /// # Returns
    /// A complete GLSL compute shader string
    fn convert_to_compute_shader(&self, texture_formats: &HashMap<String, wgpu::TextureFormat>) -> String {
        let mut compute_shader = String::new();

        // Add GLSL version and compute shader layout
        let header = format!("#version 450\nlayout(local_size_x = {COMPUTE_WORKGROUP_SIZE_X}, local_size_y = {COMPUTE_WORKGROUP_SIZE_Y}, local_size_z = 1) in;\n\n#define GLSL_REFERENCE_ENGINE 1\n\n");

        // Helper function to generate GLSL texture binding declarations
        // Uses different binding types based on access pattern: texture2D for inputs, image2D for outputs
        let generate_texture_binding = |binding_index: usize, texture_name: &str, access_type: &str, format: wgpu::TextureFormat| -> String {
            match access_type {
                "readonly" => {
                    // Input textures use texture2D for sampling with interpolation
                    format!("layout(binding = {binding_index}) uniform texture2D {texture_name};\n")
                }
                "writeonly" => {
                    // Output textures use image storage for direct pixel writes
                    let format_string = match format {
                        wgpu::TextureFormat::R32Float => "r32f",       // Single component 32-bit float
                        wgpu::TextureFormat::Rg32Float => "rg32f",     // Two component 32-bit float
                        wgpu::TextureFormat::Rgba32Float => "rgba32f", // Four component 32-bit float
                        _ => "rgba32f",                                // Default fallback
                    };
                    format!("layout(binding = {}, {}) uniform {} image2D {};\n", binding_index, format_string, access_type, texture_name)
                }
                _ => {
                    // Fallback case for other access types - treat as image storage
                    let format_string = match format {
                        wgpu::TextureFormat::R32Float => "r32f",
                        wgpu::TextureFormat::Rg32Float => "rg32f",
                        wgpu::TextureFormat::Rgba32Float => "rgba32f",
                        _ => "rgba32f", // Default fallback
                    };
                    format!("layout(binding = {}, {}) uniform {} image2D {};\n", binding_index, format_string, access_type, texture_name)
                }
            }
        };

        // Helper function to generate global variable declarations
        // These variables are set by the main compute function and used by the hook() function
        let generate_global_variables = |inputs: &[String]| -> String {
            let mut variables = String::new();

            // Core position and size variables that every hook needs
            variables.push_str("// Global variables accessible by hook() function\n");
            variables.push_str("ivec2 gl_pos;\n"); // Current pixel position
            variables.push_str("vec2 gl_normalized_pos;\n"); // Normalized position [0,1]
            variables.push_str("ivec2 gl_output_size;\n\n"); // Output texture dimensions

            // Per-input texture variables (e.g., MAIN_pos, HOOKED_pt, etc.)
            for input in inputs {
                variables.push_str(&format!("// Variables for {input} texture\n"));
                variables.push_str(&format!("vec2 {input}_pos;\n")); // Normalized texture coordinate
                variables.push_str(&format!("vec2 {input}_pt;\n")); // Texel size (1/width, 1/height)
                variables.push_str(&format!("ivec2 {input}_size;\n\n")); // Texture dimensions
            }

            variables
        };

        // Helper function to generate texture sampling functions for each input
        // Creates optimized sampling functions that choose between filtered and direct access
        let generate_texture_loader = |input: &str, tex_index: usize| -> String {
            let tex_name = format!("tex_{tex_index}");

            // Generate texture loader functions with smart sampling strategy:
            // - Use texelFetch for same-size textures to avoid filtering artifacts
            // - Use hardware filtering for different-size textures for better quality
            format!(
                r#"// Texture loader functions for {input} (Smart sampling strategy)
vec4 {input}_tex(vec2 pos) {{
    ivec2 input_size = textureSize({tex_name}, 0);
    ivec2 output_size = imageSize(output_tex);

    // Use direct texel fetch when dimensions match to avoid sampling artifacts
    // This preserves exact pixel values without interpolation
    if (input_size == output_size) {{
        ivec2 texel_coord = ivec2(pos * vec2(input_size));
        ivec2 clamped_coord = clamp(texel_coord, ivec2(0), input_size - 1);
        return texelFetch({tex_name}, clamped_coord, 0);
    }} else {{
        // Use hardware filtering for scaling cases
        return textureLod(sampler2D({tex_name}, g_sampler), pos, 0.0);
    }}
}}

vec4 {input}_texOff(vec2 offset) {{
    ivec2 input_size = textureSize({tex_name}, 0);
    ivec2 output_size = imageSize(output_tex);

    if (input_size == output_size) {{
        // Direct texel fetch with integer offset for same-size textures
        ivec2 base_coord = ivec2({input}_pos * vec2(input_size));
        ivec2 texel_coord = base_coord + ivec2(offset);
        ivec2 clamped_coord = clamp(texel_coord, ivec2(0), input_size - 1);
        return texelFetch({tex_name}, clamped_coord, 0);
    }} else {{
        // Convert offset to texture coordinate space and use filtering
        vec2 texel_size = 1.0 / vec2(input_size);
        vec2 texel_coord = {input}_pos + offset * texel_size;
        return textureLod(sampler2D({tex_name}, g_sampler), texel_coord, 0.0);
    }}
}}

// Extension function to force linear sampling regardless of size matching
// Useful for algorithms that specifically need interpolated values
vec4 {input}_texLinear(vec2 pos) {{
    return textureLod(sampler2D({tex_name}, g_sampler), pos, 0.0);
}}

"#
            )
        };

        // Helper function to generate variable initialization code
        // Sets up all global variables that the hook() function expects to be available
        let generate_variable_initialization = |inputs: &[String]| -> String {
            let mut init = String::new();

            // Initialize core position and size variables
            init.push_str("    // Initialize global variables\n");
            init.push_str("    gl_pos = pos;\n"); // Current pixel position
            init.push_str("    gl_normalized_pos = normalized_pos;\n"); // Normalized [0,1] position
            init.push_str("    gl_output_size = output_size;\n\n"); // Output dimensions

            // Initialize per-input texture variables
            for (i, input) in inputs.iter().enumerate() {
                let tex_name = format!("tex_{i}");
                init.push_str(&format!("    // Initialize {input} variables\n"));
                init.push_str(&format!("    {input}_pos = normalized_pos;\n")); // Same as current position
                init.push_str(&format!("    {input}_pt = vec2(1.0) / vec2(textureSize({tex_name}, 0));\n")); // Texel size
                init.push_str(&format!("    {input}_size = ivec2(textureSize({tex_name}, 0));\n\n")); // Texture dimensions
            }

            init
        };

        // Helper function to generate the main compute shader function
        // This sets up the compute shader entry point and calls the hook() function
        let generate_main_function = |inputs: &[String]| -> String {
            let variable_init = generate_variable_initialization(inputs);
            format!(
                r#"void main() {{
    // Get current pixel position from compute shader invocation
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    ivec2 output_size = imageSize(output_tex);

    // Bounds check to avoid processing pixels outside the output texture
    if (pos.x >= output_size.x || pos.y >= output_size.y) {{
        return;
    }}

    // Convert pixel position to normalized coordinates [0,1]
    // Add 0.5 offset for pixel center sampling
    vec2 normalized_pos = (vec2(pos) + vec2(0.5, 0.5)) / vec2(output_size);

{variable_init}
    // Call the original hook function to process this pixel
    vec4 result = hook();

    // Write the result to the output texture
    imageStore(output_tex, pos, result);
}}
"#
            )
        };

        // Build the complete compute shader by assembling all parts

        // 1. Add GLSL version and compute layout header
        compute_shader.push_str(&header);

        // 2. Add global variable declarations (accessible by hook function)
        compute_shader.push_str(&generate_global_variables(&self.bind));

        // 3. Generate input texture bindings
        for (i, input_name) in self.bind.iter().enumerate() {
            let texture_name = format!("tex_{i}");
            // Map HOOKED to MAIN for texture format lookup
            let texture_key = if input_name == "HOOKED" { "MAIN" } else { input_name };
            let input_format = texture_formats.get(texture_key).copied().unwrap_or(wgpu::TextureFormat::Rgba32Float);
            compute_shader.push_str(&generate_texture_binding(i, &texture_name, "readonly", input_format));
        }

        // 4. Add output texture binding (uses high binding number to avoid conflicts)
        let output_format = self.get_output_format();
        compute_shader.push_str(&generate_texture_binding(100, "output_tex", "writeonly", output_format));

        // 5. Add sampler binding for texture filtering
        compute_shader.push_str("layout(binding = 200) uniform sampler g_sampler;");
        compute_shader.push('\n');

        // 6. Generate texture loading functions that the hook() function will use
        // These must come before the original GLSL code that references them
        for (i, input) in self.bind.iter().enumerate() {
            compute_shader.push_str(&generate_texture_loader(input, i));
        }

        // 7. Add the original GLSL shader code (contains the hook() function)
        compute_shader.push_str("// Original GLSL shader code\n");
        compute_shader.push_str(&self.glsl_code);
        compute_shader.push('\n');
        compute_shader.push('\n');

        // 8. Add the main compute function that drives the execution
        compute_shader.push_str("// Main compute function\n");
        compute_shader.push_str(&generate_main_function(&self.bind));

        compute_shader
    }
}

impl GlslReferenceEngine {
    /// Creates a new GLSL reference engine instance
    ///
    /// Initializes wgpu with high-performance settings and the required features
    /// for processing GLSL shaders. This includes float texture filtering support
    /// and elevated storage texture limits for complex shader pipelines.
    ///
    /// # Returns
    /// A new engine instance or an error if initialization fails
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create wgpu instance with all available backends for maximum compatibility
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request a high-performance adapter optimized for compute operations
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None, // No surface needed for compute-only operations
                force_fallback_adapter: false,
            })
            .await?;

        // Request device with features required for GLSL shader processing
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("GLSL Reference Engine"),
                required_features: wgpu::Features::FLOAT32_FILTERABLE,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await?;

        Ok(Self {
            device,
            queue,
            shader_cache: HashMap::new(), // Initialize empty cache for compiled shaders
        })
    }

    /// Creates or retrieves a cached shader module from GLSL source
    ///
    /// # Arguments
    /// * `name` - Unique name for caching the shader
    /// * `source` - GLSL compute shader source code
    ///
    /// # Returns
    /// A compiled shader module
    fn create_shader_module(&mut self, name: &str, source: &str) -> Result<wgpu::ShaderModule, wgpu::Error> {
        // Check cache first to avoid recompiling identical shaders
        if !self.shader_cache.contains_key(name) {
            // Compile GLSL source to a shader module
            let shader_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("glsl_shader_module_{name}")),
                source: wgpu::ShaderSource::Glsl {
                    shader: source.into(),
                    stage: wgpu::naga::ShaderStage::Compute, // Always compute for our use case
                    defines: Default::default(),
                },
            });
            // Cache the compiled shader for potential reuse
            self.shader_cache.insert(name.to_string(), shader_module);
        }

        // Return cached shader module
        Ok(self.shader_cache.get(name).unwrap().clone())
    }
}

impl ImageProcessor {
    /// Creates a new image processor instance
    ///
    /// # Arguments
    /// * `engine` - The GLSL reference engine to use for processing
    ///
    /// # Returns
    /// A new image processor with empty texture cache
    pub fn new(engine: GlslReferenceEngine) -> Self {
        Self {
            engine,
            intermediate_textures: HashMap::new(), // Empty cache for pipeline textures
        }
    }

    /// Initializes the pipeline texture state with the input image
    ///
    /// Sets up the initial HOOKED and MAIN textures that hooks can reference.
    /// Both textures initially point to the same input texture.
    ///
    /// # Arguments
    /// * `input_texture` - The source input texture for the pipeline
    pub fn initialize_pipeline_textures(&mut self, input_texture: wgpu::Texture) {
        // HOOKED refers to the texture being processed in the current context
        self.intermediate_textures.insert("HOOKED".to_string(), input_texture.clone());
        // MAIN is the primary image being processed through the pipeline
        self.intermediate_textures.insert("MAIN".to_string(), input_texture);
    }

    /// Builds a mapping of texture names to their GPU formats
    ///
    /// Analyzes the shader pipeline to determine appropriate texture formats
    /// for each intermediate texture based on component counts.
    ///
    /// # Arguments
    /// * `hooks` - The shader hooks that define the pipeline
    ///
    /// # Returns
    /// A mapping from texture names to wgpu texture formats
    pub fn build_texture_format_map(&self, hooks: &[MpvHook]) -> HashMap<String, wgpu::TextureFormat> {
        let mut texture_formats = HashMap::new();

        // Initialize with default RGBA format for input textures
        texture_formats.insert("MAIN".to_string(), wgpu::TextureFormat::Rgba32Float);
        texture_formats.insert("HOOKED".to_string(), wgpu::TextureFormat::Rgba32Float);

        // Process hooks in execution order to build dependency chain
        // Each SAVE directive updates the format for that texture name
        for hook in hooks {
            // Determine where this hook saves its output (defaults to MAIN)
            let save_name = hook.save.as_deref().unwrap_or("MAIN");

            // Get the output format for this hook based on its component count
            let format = hook.get_output_format();
            texture_formats.insert(save_name.to_string(), format);

            // If this hook modifies MAIN, update HOOKED to match
            // since HOOKED typically refers to the current state of MAIN
            if save_name == "MAIN" {
                texture_formats.insert("MAIN".to_string(), format);
                texture_formats.insert("HOOKED".to_string(), format);
            }
        }

        texture_formats
    }

    /// Creates a complete bind group for a shader hook with all required resources
    ///
    /// This creates texture views for all input textures specified in the hook's BIND
    /// directives, plus the output texture and a linear sampler. Uses a fixed binding
    /// layout: inputs at bindings 0-N, output at binding 100, sampler at binding 200.
    ///
    /// # Arguments
    /// * `hook` - The mpv hook requiring resource binding
    /// * `hook_index` - Index of this hook for debugging labels
    /// * `output_texture` - The texture where this hook writes its output
    /// * `pipeline` - The compute pipeline to create bindings for
    ///
    /// # Returns
    /// A bind group with all resources bound, or an error if textures are missing
    fn create_hook_bind_group(&self, hook: &MpvHook, hook_index: usize, output_texture: &wgpu::Texture, pipeline: &wgpu::ComputePipeline) -> Result<wgpu::BindGroup, Box<dyn std::error::Error>> {
        let mut bind_group_entries = Vec::new();
        let mut texture_views = Vec::new(); // Store texture views to extend their lifetime
        // Create a linear sampler for texture filtering operations
        let sampler = create_sampler(&self.engine.device, SamplerFilterMode::Linear);

        // Process all input textures specified in BIND directives
        for input_name in &hook.bind {
            // Map HOOKED to MAIN since they typically refer to the same texture
            let texture_key = if input_name == "HOOKED" { "MAIN" } else { input_name };
            if let Some(texture) = self.intermediate_textures.get(texture_key) {
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                texture_views.push(texture_view);
            } else {
                return Err(format!("Missing texture for input: {}", input_name).into());
            }
        }

        // Add input texture bindings (sequential binding numbers starting from 0)
        for (i, texture_view) in texture_views.iter().enumerate() {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(texture_view),
            });
        }

        // Add output texture binding at fixed binding 100 (storage image)
        let output_texture_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: 100,
            resource: wgpu::BindingResource::TextureView(&output_texture_view),
        });

        // Add sampler binding at fixed binding 200
        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: 200,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

        // Create the final bind group with all resources
        let bind_group = self.engine.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("glsl_bind_group_{hook_index}")),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &bind_group_entries,
        });

        Ok(bind_group)
    }

    /// Builds a texture format map based on currently available intermediate textures
    ///
    /// This creates a format map using the actual formats of textures currently stored
    /// in the intermediate texture cache. This is used for shader generation to ensure
    /// input texture formats match what's actually available.
    ///
    /// # Returns
    /// Map from texture names to their current actual formats
    fn build_dynamic_texture_format_map(&self) -> HashMap<String, wgpu::TextureFormat> {
        let mut texture_formats = HashMap::new();

        // Extract actual formats from currently allocated intermediate textures
        // This ensures shader generation uses the correct input formats
        for (name, texture) in &self.intermediate_textures {
            texture_formats.insert(name.clone(), texture.format());
        }

        // Note: Output format is determined by the hook's component count,
        // but input formats must match existing intermediate textures

        texture_formats
    }

    /// Executes a compute pass for a single hook
    ///
    /// Creates a command encoder, sets up the compute pass with the pipeline and
    /// bind group, calculates appropriate workgroup dimensions, and submits the
    /// commands for execution on the GPU.
    ///
    /// # Arguments
    /// * `pipeline` - The compute pipeline to execute
    /// * `bind_group` - The bind group containing all resources
    /// * `output_width` - Width of the output texture in pixels
    /// * `output_height` - Height of the output texture in pixels
    /// * `hook_index` - Index of this hook for debugging labels
    fn execute_compute_pass(&self, pipeline: &wgpu::ComputePipeline, bind_group: &wgpu::BindGroup, output_width: u32, output_height: u32, hook_index: usize) {
        // Create command encoder for recording GPU commands
        let mut encoder = self.engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("glsl_encoder_{hook_index}")),
        });

        {
            // Begin compute pass within a scope to ensure proper resource cleanup
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("glsl_compute_pass_{hook_index}")),
                timestamp_writes: None, // No GPU timing needed for verification
            });

            // Set the compute pipeline and bind all resources
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            // Calculate workgroup dispatch dimensions based on output size
            // Each workgroup processes WORKGROUP_SIZE_X × WORKGROUP_SIZE_Y pixels
            let workgroup_x = calculate_workgroup_count(output_width, COMPUTE_WORKGROUP_SIZE_X);
            let workgroup_y = calculate_workgroup_count(output_height, COMPUTE_WORKGROUP_SIZE_Y);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // Submit the recorded commands to the GPU queue for execution
        self.engine.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Updates the intermediate texture cache after processing a hook
    ///
    /// Stores the hook's output texture under the name specified by its SAVE directive.
    /// If the hook saves to MAIN, also updates HOOKED to point to the same texture
    /// since HOOKED typically refers to the current state of the main image.
    ///
    /// # Arguments
    /// * `hook` - The hook that was just processed
    /// * `output_texture` - The output texture produced by the hook
    fn update_intermediate_textures(&mut self, hook: &MpvHook, output_texture: wgpu::Texture) {
        // Determine where to save the output (defaults to MAIN if no SAVE directive)
        let save_name = hook.save.as_deref().unwrap_or("MAIN");

        // If saving to MAIN, update HOOKED to match since they represent the same image
        if save_name == "MAIN" {
            self.intermediate_textures.insert("HOOKED".to_string(), output_texture.clone());
        }

        // Store the output texture under its save name for future hooks to reference
        self.intermediate_textures.insert(save_name.to_string(), output_texture);
    }

    /// Processes a single mpv hook through the complete shader pipeline
    ///
    /// This performs the full processing pipeline for one hook: calculates output
    /// dimensions, creates output texture, converts GLSL to compute shader, compiles
    /// the shader, creates pipeline and bind group, executes on GPU, and updates
    /// the intermediate texture cache.
    ///
    /// # Arguments
    /// * `hook` - The mpv hook to process
    /// * `hook_index` - Index of this hook for debugging and labeling
    /// * `output_path` - Optional path to save intermediate output for debugging
    /// * `log` - Whether to enable debug logging
    ///
    /// # Returns
    /// Result indicating success or failure of the processing
    pub fn process_single_hook(&mut self, hook: &MpvHook, hook_index: usize, output_path: Option<&str>, log: bool) -> Result<(), Box<dyn std::error::Error>> {
        if log {
            println!("Processing hook {}: {}", hook_index, hook.desc);
        }

        // Calculate output dimensions based on hook's WIDTH/HEIGHT directives
        let (output_width, output_height) = hook.calculate_output_size(&self.intermediate_textures)?;

        // Create output texture with appropriate format based on component count
        let output_format = hook.get_output_format();
        let output_texture = create_texture(&self.engine.device, output_width, output_height, output_format, TEXTURE_USAGE_STORAGE);

        // Build texture format map using actual formats of current intermediate textures
        let texture_formats = self.build_dynamic_texture_format_map();

        // Convert the mpv-style GLSL hook to a compute shader with proper bindings
        let compute_shader_source = hook.convert_to_compute_shader(&texture_formats);

        // Compile the compute shader into a shader module (with caching)
        let shader_name = format!("hook_{hook_index}");
        let shader_module = self.engine.create_shader_module(&shader_name, &compute_shader_source)?;

        // Create compute pipeline with automatic layout inference
        let pipeline = self.engine.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("glsl_compute_pipeline_{hook_index}")),
            layout: None, // Let wgpu infer the layout from the shader
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create bind group with all required resources (inputs, output, sampler)
        let bind_group = self.create_hook_bind_group(hook, hook_index, &output_texture, &pipeline)?;

        // Execute the compute shader on the GPU
        self.execute_compute_pass(&pipeline, &bind_group, output_width, output_height, hook_index);

        // Save intermediate output for debugging if requested
        if let Some(output_path) = output_path {
            save_texture_as_image_file(&self.engine.device, &self.engine.queue, &output_texture, output_path)?;
            if log {
                println!("- Pass {hook_index} output saved to: {output_path}");
            }
        }

        // Update intermediate texture cache with the hook's output
        self.update_intermediate_textures(hook, output_texture);

        if log {
            println!("- Hook {hook_index} completed: {output_width}x{output_height}");
        }

        Ok(())
    }

    /// Processes a complete shader pipeline from file with I/O operations
    ///
    /// Loads a GLSL shader file, parses all mpv hooks, loads the input image,
    /// processes each hook in sequence, and saves the final result. Optionally
    /// saves intermediate outputs from each hook for debugging.
    ///
    /// # Arguments
    /// * `shader_path` - Path to the GLSL shader file containing mpv hooks
    /// * `input_path` - Path to the input image file
    /// * `output_path` - Path where to save the final processed image
    /// * `save_intermediate_textures` - Whether to save intermediate outputs from each hook
    ///
    /// # Returns
    /// Result indicating success or failure of the entire pipeline
    pub fn process_shader_pipeline(&mut self, shader_path: &str, input_path: &str, output_path: &str, save_intermediate_textures: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Load shader file and parse all mpv hooks from the GLSL source
        let shader_source = fs::read_to_string(shader_path)?;
        let hooks = MpvHook::parse_from_glsl(&shader_source)?;

        // Log information about the discovered hooks
        println!("Found {} hooks in shader", hooks.len());
        for (i, hook) in hooks.iter().enumerate() {
            println!("- Hook {i}: {} ({})", hook.desc, hook.hook);
        }

        // Load input image
        let input_texture = load_image_file_as_texture(&self.engine.device, &self.engine.queue, input_path)?;

        // Initialize the pipeline texture state with the input image
        self.initialize_pipeline_textures(input_texture);

        // Process each hook in sequence, maintaining texture state between hooks
        let output_path_base = Path::new(output_path).with_extension("").to_str().unwrap().to_string();
        for (hook_index, hook) in hooks.iter().enumerate() {
            // Generate intermediate output path if debugging is enabled
            let pass_output_path = if save_intermediate_textures {
                Some(format!("{output_path_base}_hook{}.png", hook_index + 1))
            } else {
                None
            };
            self.process_single_hook(hook, hook_index, pass_output_path.as_deref(), true)?;
        }

        // Save the final result from the MAIN texture
        if let Some(final_texture) = self.intermediate_textures.get("MAIN") {
            save_texture_as_image_file(&self.engine.device, &self.engine.queue, final_texture, output_path)?;
            println!("Final result saved to: {output_path}");
        } else {
            return Err("No final output texture found".into());
        }

        Ok(())
    }

    /// Processes a shader pipeline from memory without file I/O operations
    ///
    /// This version operates purely in memory for performance testing and
    /// programmatic usage. Takes shader source and input image directly,
    /// processes the pipeline, and returns the result image with timing data.
    ///
    /// # Arguments
    /// * `shader_source` - GLSL shader source containing mpv hooks
    /// * `input_image` - Input image to process
    ///
    /// # Returns
    /// Tuple of (processed image, processing duration) or error
    pub fn process_shader_pipeline_no_io(&mut self, shader_source: &str, input_image: &image::DynamicImage) -> Result<(image::Rgba32FImage, std::time::Duration), Box<dyn std::error::Error>> {
        // Parse mpv hooks from the provided GLSL source
        let hooks = MpvHook::parse_from_glsl(shader_source)?;

        // Convert input image to GPU texture
        let input_texture = load_image_as_texture(&self.engine.device, &self.engine.queue, input_image)?;

        // Initialize pipeline texture state with the input image
        self.initialize_pipeline_textures(input_texture);

        // Start timing the actual processing (excluding setup)
        let timepoint = std::time::Instant::now();

        // Process each hook in sequence without saving intermediate outputs
        for (hook_index, hook) in hooks.iter().enumerate() {
            self.process_single_hook(hook, hook_index, None, false)?;
        }

        // Extract the final result from the MAIN texture
        let image = if let Some(final_texture) = self.intermediate_textures.get("MAIN") {
            save_texture_as_image(&self.engine.device, &self.engine.queue, final_texture)?
        } else {
            return Err("No final output texture found".into());
        };

        // Calculate total processing time
        let elapsed = timepoint.elapsed();

        Ok((image, elapsed))
    }
}

/// Analyzes a GLSL shader file and displays detailed pipeline information
///
/// This function provides detailed analysis of Anime4K GLSL shaders, including
/// hook structure, texture dependencies, and pipeline organization. Useful for
/// debugging and understanding shader behavior.
///
/// # Arguments
/// * `shader_path` - Path to the GLSL shader file to analyze
///
/// # Returns
/// Result indicating success or failure of the analysis
pub async fn analyze_shader(shader_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Shader Analysis Mode ===");
    println!("Analyzing shader: {shader_path}");

    // Verify the shader file exists before attempting to read it
    if !Path::new(shader_path).exists() {
        return Err(format!("Shader file not found: {shader_path}").into());
    }

    // Load and parse all mpv hooks from the shader file
    let shader_content = fs::read_to_string(shader_path)?;
    let hooks = MpvHook::parse_from_glsl(&shader_content)?;

    println!("\nFound {} hooks in the shader:", hooks.len());

    // Display detailed information for each hook
    for (i, hook) in hooks.iter().enumerate() {
        println!("\n--- Hook {i} ---");
        println!("Description: {}", hook.desc);
        println!("Hook Point: {}", hook.hook);
        println!("Bindings: {:?}", hook.bind);

        // Show optional directives if present
        if let Some(save) = &hook.save {
            println!("Save As: {save}");
        }

        if let Some(components) = hook.components {
            println!("Components: {components}");
        }

        // Display the original GLSL code
        println!("\nOriginal GLSL Code:");
        println!("{}", hook.glsl_code);

        // Show the converted compute shader with standard format assumptions
        println!("\n--- Standard Compute Shader ---");
        let mut demo_formats = HashMap::new();
        demo_formats.insert("MAIN".to_string(), wgpu::TextureFormat::Rgba32Float);
        demo_formats.insert("HOOKED".to_string(), wgpu::TextureFormat::Rgba32Float);
        // Add RGBA32Float format for any additional input textures
        for input in &hook.bind {
            if input != "HOOKED" && input != "MAIN" {
                demo_formats.insert(input.clone(), wgpu::TextureFormat::Rgba32Float);
            }
        }
        let standard_shader = hook.convert_to_compute_shader(&demo_formats);
        println!("{standard_shader}");

        println!("\n{}", "=".repeat(80));
    }

    println!("\n=== Analysis Complete ===");
    Ok(())
}
