//! WGSL shader minification utilities.
//!
//! This module provides functionality to reduce the size of WGSL shader source code.

/// Minifies WGSL shader source code to reduce binary size.
///
/// Uses `naga` to parse, validate, and regenerate the WGSL code in a more compact form.
/// This reduces the size of embedded shaders without affecting functionality.
///
/// # Arguments
///
/// * `shader` - A string slice containing the WGSL shader source code.
///
/// # Returns
///
/// A `Result` containing the minified WGSL source code as a `String`, or an error if parsing fails.
pub fn minify_wgsl(shader: &str) -> Result<String, std::boxed::Box<dyn std::error::Error>> {
    let mut module = naga::front::wgsl::parse_str(shader)?;

    wgsl_minifier::minify_module(&mut module);

    let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all());
    let info = validator.validate(&module)?;
    let output = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())?;

    let minified = wgsl_minifier::minify_wgsl_source(&output);

    Ok(minified)
}
