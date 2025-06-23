//! GLSL to WGSL conversion for CNN shaders
//!
//! This module handles the conversion of mpv-style GLSL hooks used in the original
//! Anime4K implementation to WGSL compute shaders for wgpu.

use std::collections::HashMap;

use regex::Regex;

/// Workgroup size for 2D convolution compute shaders (X dimension)
const COMPUTE_WORKGROUP_SIZE_X: u32 = 8;
/// Workgroup size for 2D convolution compute shaders (Y dimension)
const COMPUTE_WORKGROUP_SIZE_Y: u32 = 8;

/// Type of shader stage in the CNN pipeline
///
/// Anime4K uses different types of processing stages, each requiring
/// different shader generation approaches.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConvolutionStageType {
    /// Convolutional layer that applies learned filters to input textures
    ///
    /// These stages perform the core CNN operations including:
    /// - Feature extraction through convolution
    /// - Non-linear activation (ReLU)
    /// - Bias addition
    /// - Residual connections
    Conv,
    /// Depth to space layer for upscaling
    ///
    /// These stages rearrange channels to increase spatial resolution,
    /// typically used as the final upscaling step in the pipeline.
    DepthToSpace,
}

/// Represents a parsed mpv-style shader hook
///
/// mpv hooks are GLSL shaders with special comment directives that define
/// their behavior and integration into the rendering pipeline. This struct
/// contains all the parsed information needed to convert the hook to WGSL.
#[derive(Debug, Clone)]
pub struct MpvHook {
    /// Human-readable description from DESC directive
    name: String,
    /// Upscaling factor (1, 2, 3, or 4) relative to the source texture
    scale_factor: u32,
    /// Whether the hook needs a texture sampler for interpolated access
    needs_sampler: bool,
    /// Whether the hook needs bounds checking for texture coordinates
    needs_bound: bool,
    /// Input texture names from BIND directives
    inputs: Vec<String>,
    /// Output texture name from SAVE directive
    output: String,
    /// The type of shader stage (Conv or DepthToSpace)
    r#type: ConvolutionStageType,
    /// The original GLSL shader code (without comment directives)
    code: String,
}

impl MpvHook {
    /// Splits GLSL source code into individual mpv hook sections
    ///
    /// Parses mpv-style GLSL shader source and separates it into individual
    /// processing hooks, each beginning with a DESC directive. This allows
    /// multi-pass CNN shaders to be processed as separate compute stages.
    ///
    /// # Arguments
    /// * `source` - Complete GLSL source code containing multiple mpv hooks
    ///
    /// # Returns
    /// Vector of individual hook source code strings
    pub fn parse_mpv_hooks(source: &str) -> Vec<String> {
        let mut hooks = Vec::new();
        let mut current_hook = String::new();

        for line in source.lines() {
            if line.starts_with("//!DESC ") {
                if !current_hook.is_empty() {
                    hooks.push(current_hook);
                }
                current_hook = format!("{line}\n");
                continue;
            }

            if !current_hook.is_empty() {
                current_hook.push_str(&format!("{line}\n"));
            }
        }

        if !current_hook.is_empty() {
            hooks.push(current_hook);
        }

        hooks
    }

    /// Creates a default scale factor mapping for texture names
    ///
    /// This initializes a mapping that tracks the scale factors of different textures
    /// throughout the CNN pipeline. All base textures start with scale factor 1.
    ///
    /// # Returns
    /// A HashMap mapping texture names to their scale factors:
    /// - "MAIN": 1 (the main input texture)
    /// - "HOOKED": 1 (the hooked texture in mpv terminology)
    /// - "source": 1 (the source texture)
    pub fn new_scale_factor_map() -> HashMap<String, u32> {
        let mut scale_factor_map = HashMap::new();
        scale_factor_map.insert("MAIN".to_string(), 1);
        scale_factor_map.insert("HOOKED".to_string(), 1);
        scale_factor_map.insert("source".to_string(), 1);
        scale_factor_map
    }

    /// Creates a new MpvHook from GLSL source code
    ///
    /// Parses an mpv-style GLSL hook and extracts all the metadata including:
    /// - Description from //!DESC directive
    /// - Scale factors from //!WIDTH and //!HEIGHT directives
    /// - Input textures from //!BIND directives
    /// - Output texture from //!SAVE directive
    /// - Hook target from //!HOOK directive
    /// - Component count from //!COMPONENTS directive
    ///
    /// The method also determines whether the hook needs texture sampling or bounds
    /// checking based on the scale factors of input and output textures.
    ///
    /// # Arguments
    /// * `source` - The GLSL source code containing mpv directives and shader code
    /// * `scale_factor_map` - Mutable mapping of texture names to scale factors, updated with new outputs
    ///
    /// # Returns
    /// A parsed MpvHook ready for conversion to WGSL
    ///
    /// # Errors
    /// Returns an error if:
    /// - Required directives are missing or malformed
    /// - Scale factors are inconsistent between WIDTH and HEIGHT
    /// - Referenced textures are not found in the scale factor map
    /// - Unsupported hook types or component counts are used
    pub fn new(source: &str, scale_factor_map: &mut HashMap<String, u32>) -> Result<Self, std::boxed::Box<dyn std::error::Error>> {
        let mut name = String::new();
        let mut scale_factor = 0;
        let mut inputs = Vec::new();
        let mut output = String::new();
        let mut code = String::new();

        let scale_factor_re = Regex::new(r"^//!(?:WIDTH|HEIGHT) (\w+)\.[wh](?: (\d+) \*)?$").unwrap();

        for line in source.lines() {
            if let Some(content) = line.strip_prefix("//!DESC ").map(str::trim) {
                name = content.to_string();
            } else if line.starts_with("//!WIDTH ") || line.starts_with("//!HEIGHT ") {
                let current_match = scale_factor_re
                    .captures(line)
                    .ok_or_else(|| std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid scale factor line")))?;
                let base_texture_name = &current_match[1];
                let ratio = current_match.get(2).map(|m| m.as_str().parse::<u32>().unwrap()).unwrap_or(1);
                let base_texture_scale_factor = scale_factor_map
                    .get(base_texture_name)
                    .ok_or_else(|| std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unknown base texture name")))?;
                let current_scale_factor = base_texture_scale_factor * ratio;
                if scale_factor == 0 {
                    scale_factor = current_scale_factor;
                } else if scale_factor != current_scale_factor {
                    return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Inconsistent scale factors")));
                }
            } else if let Some(content) = line.strip_prefix("//!BIND ").map(str::trim) {
                let name = if content == "MAIN" { "source" } else { content };
                inputs.push(name.to_string());
            } else if let Some(content) = line.strip_prefix("//!SAVE ").map(str::trim) {
                let name = if content == "MAIN" { "dest" } else { content };
                output = name.to_string();
            } else if let Some(content) = line.strip_prefix("//!HOOK ").map(str::trim) {
                if content != "MAIN" {
                    return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported hook type")));
                }
            } else if let Some(content) = line.strip_prefix("//!COMPONENTS ").map(str::trim) {
                if content != "4" {
                    return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported number of components")));
                }
            } else if line.starts_with("//!WHEN ") {
                // ignore
            } else {
                code.push_str(&format!("{line}\n"));
            }
        }

        let r#type = if name.contains("-Conv-") {
            Ok(ConvolutionStageType::Conv)
        } else if name.contains("-Depth-to-Space") {
            Ok(ConvolutionStageType::DepthToSpace)
        } else {
            Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unknown hook type")))
        }?;

        if name.is_empty() {
            return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No name specified")));
        }

        if inputs.is_empty() {
            return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No inputs specified")));
        }

        if output.is_empty() {
            return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No output specified")));
        }

        if scale_factor == 0 {
            return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No scale factor specified")));
        }

        let (needs_sampler, needs_bound) = {
            let mut needs_sampler = false;
            let mut needs_bound = false;
            for input in &inputs {
                if let Some(input_scale_factor) = scale_factor_map.get(input).copied() {
                    if input_scale_factor == scale_factor {
                        needs_bound = true;
                    } else {
                        needs_sampler = true;
                    }
                } else {
                    return Err(std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown input texture: {input}"))));
                }
            }
            (needs_sampler, needs_bound)
        };

        scale_factor_map.insert(output.clone(), scale_factor);

        Ok(Self {
            name,
            scale_factor,
            needs_sampler,
            needs_bound,
            inputs,
            output,
            r#type,
            code,
        })
    }
}

/// WGSL shader stage type variants
///
/// Represents the different types of shader stages that can be generated
/// from mpv hooks, each requiring different code generation approaches.
#[derive(Debug, Clone)]
pub enum WgslStageShaderType {
    /// Convolutional layer with custom WGSL code
    Conv {
        /// The translated WGSL compute shader code
        code: String,
    },
    /// Depth-to-space upscaling layer with fixed algorithm
    DepthToSpace {
        /// Number of input channels to process
        channel_count: u32,
    },
}

/// A WGSL compute shader stage converted from an mpv hook
///
/// Contains all the information needed to generate a complete WGSL compute shader
/// pass, including bindings, scale factors, and the shader code itself.
#[derive(Debug, Clone)]
pub struct WgslStageShader {
    /// Original mpv hook that this shader was converted from
    pub source: MpvHook,
    /// Generated name for this shader stage
    pub name: String,
    /// Type and content of the WGSL shader
    pub r#type: WgslStageShaderType,
    /// Input texture bindings (binding_index, texture_name)
    pub inputs: Vec<(u32, String)>,
    /// Output texture binding (binding_index, texture_name)
    pub output: (u32, String),
    /// Optional sampler binding index for texture sampling
    pub sampler: Option<u32>,
    /// Scale factor as a string for code generation
    pub scale_factor: String,
}

impl WgslStageShader {
    /// Creates a new WGSL stage shader from an mpv hook
    ///
    /// Converts an mpv hook into a WGSL compute shader stage by:
    /// - Determining the appropriate shader type (Conv or DepthToSpace)
    /// - Assigning binding indices for inputs, outputs, and samplers
    /// - Normalizing texture names (source -> SOURCE, dest -> RESULT)
    /// - Translating GLSL code to WGSL for convolutional layers
    ///
    /// # Arguments
    /// * `source` - The parsed mpv hook to convert
    /// * `scale_factor_map` - Mapping of texture names to their scale factors
    ///
    /// # Returns
    /// A WGSL shader stage ready for code generation
    ///
    /// # Errors
    /// Returns an error if GLSL to WGSL translation fails for convolutional layers
    pub fn new(source: MpvHook, scale_factor_map: &HashMap<String, u32>) -> Result<Self, std::boxed::Box<dyn std::error::Error>> {
        let name = if source.output == "dest" { "result".to_string() } else { source.output.clone() };
        let r#type = match source.r#type {
            ConvolutionStageType::Conv => WgslStageShaderType::Conv {
                code: Self::convert_conv_hook_code(&source, scale_factor_map)?,
            },
            ConvolutionStageType::DepthToSpace => {
                let channel_count = source.inputs.len() as u32;
                WgslStageShaderType::DepthToSpace { channel_count }
            }
        };
        let inputs: Vec<_> = source
            .inputs
            .iter()
            .enumerate()
            .map(|(i, input)| (i as u32, if input == "source" { "SOURCE".to_string() } else { input.clone() }))
            .collect();
        let output = (inputs.len() as u32, if source.output == "dest" { "RESULT".to_string() } else { name.clone() });
        let sampler = if source.scale_factor > 1 { Some(inputs.len() as u32 + 1) } else { None };
        let scale_factor = if source.scale_factor > 1 { format!("{}", source.scale_factor) } else { "1".to_string() };
        Ok(Self {
            name,
            r#type,
            source,
            inputs,
            output,
            sampler,
            scale_factor,
        })
    }

    /// Translates GLSL convolutional hook code to WGSL compute shader
    ///
    /// This is the core translation function that converts mpv-style GLSL code
    /// to WGSL compute shaders. It handles:
    ///
    /// - **Texture bindings**: Converts GLSL texture declarations to WGSL bindings
    /// - **Macro expansion**: Translates GO and G macros to WGSL functions
    /// - **Entry point conversion**: Transforms hook() function to compute shader main
    /// - **Matrix operations**: Converts mat4 multiplications to WGSL syntax
    /// - **Bounds checking**: Adds texture bounds checking for edge cases
    /// - **Sampling**: Handles texture sampling for different scale factors
    ///
    /// The translation supports various GLSL patterns commonly used in Anime4K:
    /// - Offset-based texture access with GO macros
    /// - ReLU activation functions with G macros
    /// - Matrix-vector multiplications for convolutions
    /// - Bias addition with vector constants
    /// - Overlay operations for residual connections
    ///
    /// # Arguments
    /// * `source` - The mpv hook containing GLSL code to translate
    /// * `scale_factor_map` - Mapping of texture names to scale factors for proper sampling
    ///
    /// # Returns
    /// Complete WGSL compute shader source code
    ///
    /// # Errors
    /// Returns an error if:
    /// - Unknown GLSL patterns are encountered
    /// - Texture references cannot be resolved
    /// - Scale factor mismatches are detected
    /// - Macro definitions are malformed
    fn convert_conv_hook_code(source: &MpvHook, scale_factor_map: &HashMap<String, u32>) -> Result<String, std::boxed::Box<dyn std::error::Error>> {
        let output_texture = &source.output;

        let mut code = String::new();
        code.push_str(&format!("// Layer: {}\n", source.name));
        code.push_str(&format!("// Inputs: {}\n", source.inputs.join(", ")));
        code.push_str(&format!("// Output: {output_texture}\n"));
        code.push_str(&format!("// Scale Factor: x{} from source\n", source.scale_factor));
        code.push('\n');

        for (i, input) in source.inputs.iter().enumerate() {
            code.push_str(&format!("@group(0) @binding({i}) var {input}_tex: texture_2d<f32>;\n"));
        }
        code.push_str(&format!(
            "@group(0) @binding({}) var {output_texture}_tex: texture_storage_2d<rgba32float, write>;\n",
            source.inputs.len()
        ));
        if source.needs_sampler {
            code.push_str(&format!("@group(0) @binding({}) var input_sampler: sampler;\n", source.inputs.len() + 1));
        }
        code.push('\n');

        // Regex patterns for parsing different GLSL constructs

        // GO macro: #define GO(x_off, y_off) (texture_texOff(vec2(x_off, y_off) * 0.5))
        // Handles offset-based texture access with optional fractional scaling
        let re_go_macro =
            Regex::new(r"^#define (?<name>\w+)\(x_off, y_off\) \((?:max\((?<sign>-?)\()?(?<texture>\w+)_texOff\(vec2\(x_off, y_off\)(?: \* (?<fraction>0\.\d+))?\)(?:\), 0.0\))?\)$").unwrap();

        // G macro with ReLU: #define G (max(-(texture_tex(pos)), 0.0))
        // Handles simple texture access with ReLU activation
        let re_g_macro_relu = Regex::new(r"^#define (?<name>\w+) \(max\((?<sign>-?)\((?<texture>\w+)_tex\(\w+\)\), 0.0\)\)$").unwrap();

        // Entry point patterns
        let re_entrypoint_begin = Regex::new(r"^vec4 hook\(\) \{$").unwrap();
        let re_entrypoint_end = Regex::new(r"^\}$").unwrap();

        // Matrix-vector multiplication: result += mat4(...) * GO(1.0, 0.0);
        let re_result_add_prod = Regex::new(r"^(?<decl>vec4 )?result \+?= mat4\((?<weights>[^)]+)\) \* (?<func>\w+)(?:\((?<x_offset>1|0|-1)\.0, (?<y_offset>1|0|-1)\.0\))?;$").unwrap();

        // Bias addition: result += vec4(...);
        let re_result_add_vec = Regex::new(r"^result \+= vec4\((?<weights>[^)]+)\);$").unwrap();

        // Return statements
        let re_return_as_is = Regex::new(r"^return result;$").unwrap();
        let re_return_overlay = Regex::new(r"^return result(?<factor>(?: \* 0\.\d+)?) \+ MAIN_tex\(MAIN_pos\);$").unwrap();

        let mut func_to_scale_factor = HashMap::new();

        // Process the GLSL source code line by line, converting each construct to WGSL
        for line in source.code.lines() {
            let line = line.trim();

            // Handle GO macro definitions for offset-based texture access
            if let Some(caps) = re_go_macro.captures(line) {
                let func_name = &caps["name"];
                let texture_name = if &caps["texture"] == "MAIN" { "source" } else { &caps["texture"] };
                let fraction = caps.name("fraction").map(|m| m.as_str());
                let sign = caps.name("sign").map(|m| m.as_str());

                let target_scale_factor = *scale_factor_map
                    .get(texture_name)
                    .ok_or_else(|| std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown texture: {texture_name}"))))?;

                match fraction {
                    Some(fraction) => {
                        if target_scale_factor == source.scale_factor {
                            return Err(std::boxed::Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Fraction should only be used for textures with different scale factors",
                            )));
                        }

                        code.push_str(&format!("fn {func_name}(uv_pos: vec2f, offset: vec2i) -> vec4f {{\n",));
                        code.push_str(&format!("    let coords = uv_pos + vec2f(offset) * {fraction} / vec2f(textureDimensions({texture_name}_tex));\n"));
                        code.push_str(&format!("    let value = textureSampleLevel({texture_name}_tex, input_sampler, coords, 0.0);\n"));
                    }
                    None => {
                        if target_scale_factor != source.scale_factor {
                            return Err(std::boxed::Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Fraction should be used for textures with different scale factors",
                            )));
                        }

                        code.push_str(&format!("fn {func_name}(pos: vec2i) -> vec4f {{\n",));
                        code.push_str(&format!("    let value = textureLoad({texture_name}_tex, pos, 0);\n"));
                    }
                }
                match sign {
                    Some(sign) => code.push_str(&format!("    return max({sign}value, vec4f());\n")),
                    None => code.push_str("    return value;\n"),
                }
                code.push_str("}\n");
                code.push('\n');

                func_to_scale_factor.insert(func_name.to_string(), target_scale_factor);

            // Handle G macro definitions for simple texture access with ReLU
            } else if let Some(caps) = re_g_macro_relu.captures(line) {
                let func_name = &caps["name"];
                let texture_name = if &caps["texture"] == "MAIN" { "source" } else { &caps["texture"] };
                let sign = &caps["sign"];

                let target_scale_factor = *scale_factor_map
                    .get(texture_name)
                    .ok_or_else(|| std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown texture: {texture_name}"))))?;
                if target_scale_factor != source.scale_factor {
                    return Err(std::boxed::Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Non-offset macros should only be used for textures with the same scale factor",
                    )));
                }

                code.push_str(&format!("fn {func_name}(pos: vec2i) -> vec4f {{\n",));
                code.push_str(&format!("    let value = textureLoad({texture_name}_tex, pos, 0);\n"));
                code.push_str(&format!("    return max({sign}value, vec4f());\n"));
                code.push_str("}\n");
                code.push('\n');

                func_to_scale_factor.insert(func_name.to_string(), target_scale_factor);

            // Handle entry point conversion from GLSL hook() to WGSL compute shader
            } else if re_entrypoint_begin.is_match(line) {
                // Generate bounds-checked compute shader entry point
                code.push_str(&format!("@compute @workgroup_size({COMPUTE_WORKGROUP_SIZE_X}, {COMPUTE_WORKGROUP_SIZE_Y})\n"));
                code.push_str("fn main(@builtin(global_invocation_id) pixel: vec3u) {\n");
                code.push_str(&format!("    let out_dim: vec2u = textureDimensions({output_texture}_tex);\n"));
                code.push_str("    if (pixel.x < out_dim.x && pixel.y < out_dim.y) {\n");
                code.push_str("        process(vec2i(pixel.xy));\n");
                code.push_str("    }\n");
                code.push_str("}\n");
                code.push('\n');

                // Generate unchecked variant for when bounds are guaranteed
                code.push_str(&format!("@compute @workgroup_size({COMPUTE_WORKGROUP_SIZE_X}, {COMPUTE_WORKGROUP_SIZE_Y})\n"));
                code.push_str("fn main_unchecked(@builtin(global_invocation_id) pixel: vec3u) {\n");
                code.push_str("    process(vec2i(pixel.xy));\n");
                code.push_str("}\n");
                code.push('\n');

                code.push_str("fn process(pos: vec2i) {\n");
            } else if re_entrypoint_end.is_match(line) {
                code.push_str("}\n");

            // Handle matrix-vector multiplication for convolution operations
            } else if let Some(caps) = re_result_add_prod.captures(line) {
                let weights = &caps["weights"];
                let func = &caps["func"];
                let is_decl = caps.name("decl").is_some();
                let x_offset = caps.name("x_offset").map(|m| m.as_str());
                let y_offset = caps.name("y_offset").map(|m| m.as_str());

                let func_scale_factor = *func_to_scale_factor
                    .get(func)
                    .ok_or_else(|| std::boxed::Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown function: {func}"))))?;

                if is_decl {
                    if source.needs_bound {
                        code.push_str(&format!("    let bound = vec2i(textureDimensions({output_texture}_tex)) - 1;\n"));
                    }
                    if source.needs_sampler {
                        code.push_str(&format!("    let uv_pos = (vec2f(pos) + 0.5) / vec2f(textureDimensions({output_texture}_tex));\n"));
                    }
                    code.push_str("    var result = vec4f();\n");
                }
                match (x_offset, y_offset) {
                    (Some(x_offset), Some(y_offset)) => {
                        if func_scale_factor != source.scale_factor {
                            code.push_str(&format!("    result += mat4x4f({weights}) * {func}(uv_pos, vec2i({x_offset}, {y_offset}));\n"));
                        } else {
                            let needs_neg_check = x_offset.starts_with("-") || y_offset.starts_with("-");
                            let needs_pos_check = (!x_offset.starts_with("-") && x_offset != "0") || (!y_offset.starts_with("-") && y_offset != "0");
                            let bound_checked = if needs_neg_check && needs_pos_check {
                                &format!("clamp(pos + vec2i({x_offset}, {y_offset}), vec2i(0), bound)")
                            } else if needs_neg_check {
                                &format!("max(pos + vec2i({x_offset}, {y_offset}), vec2i(0))")
                            } else if needs_pos_check {
                                &format!("min(pos + vec2i({x_offset}, {y_offset}), bound)")
                            } else {
                                "pos"
                            };
                            code.push_str(&format!("    result += mat4x4f({weights}) * {func}({bound_checked});\n"));
                        }
                    }
                    _ => {
                        if func_scale_factor != source.scale_factor {
                            return Err(std::boxed::Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Non-offset macros should only be used for textures with the same scale factor",
                            )));
                        }
                        code.push_str(&format!("    result += mat4x4f({weights}) * {func}(pos);\n"));
                    }
                }

            // Handle bias addition (vector constants)
            } else if let Some(caps) = re_result_add_vec.captures(line) {
                let weights = &caps["weights"];
                code.push_str(&format!("    result += vec4f({weights});\n"));

            // Handle direct result output
            } else if re_return_as_is.is_match(line) {
                code.push_str(&format!("    textureStore({output_texture}_tex, pos, result);\n"));

            // Handle overlay/residual connections
            } else if let Some(caps) = re_return_overlay.captures(line) {
                let factor = &caps["factor"];
                if source.scale_factor == 1 {
                    code.push_str(&format!("    textureStore({output_texture}_tex, pos, result{factor} + textureLoad(source_tex, pos, 0));\n"));
                } else {
                    code.push_str(&format!(
                        "    textureStore({output_texture}_tex, pos, result{factor} + textureSampleLevel(source_tex, input_sampler, uv_pos, 0.0));\n"
                    ));
                }
            } else if line.starts_with("//") {
                // Ignore comments
            } else if line.is_empty() {
                // Ignore empty lines
            } else {
                Err(std::boxed::Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unexpected line in {} shader code: {line}", source.name),
                )))?;
            }
        }

        Ok(code)
    }
}
