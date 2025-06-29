//! Pipeline Manifest Parser
//!
//! This module provides parsing and validation for YAML manifest files that describe
//! shader pipeline configurations. The parser handles scale factors in formats like
//! "1/2", "2", etc. and validates pipeline structure.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;

/// Represents a rational scale factor as a fraction
///
/// Used to express scaling relationships between textures in the pipeline,
/// supporting both simple integers (e.g., "2") and fractions (e.g., "1/2").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ScaleFactor {
    /// The numerator of the fraction
    pub numerator: u32,
    /// The denominator of the fraction
    pub denominator: u32,
}

impl ScaleFactor {
    /// Creates a new scale factor from numerator and denominator
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self { numerator, denominator }
    }

    /// Converts the scale factor to a floating-point value
    pub fn to_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Returns true if this scale factor equals 1.0 (no scaling)
    pub fn is_unity(&self) -> bool {
        self.numerator == self.denominator
    }

    /// Returns true if this scale factor is greater than 1.0 (upscaling)
    pub fn is_upscale(&self) -> bool {
        self.numerator > self.denominator
    }

    /// Returns true if this scale factor is less than 1.0 (downscaling)
    pub fn is_downscale(&self) -> bool {
        self.numerator < self.denominator
    }
}

impl FromStr for ScaleFactor {
    type Err = ScaleFactorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() != 2 {
                return Err(ScaleFactorParseError::InvalidFormat);
            }

            let numerator = parts[0].parse::<u32>().map_err(|_| ScaleFactorParseError::InvalidNumerator)?;
            let denominator = parts[1].parse::<u32>().map_err(|_| ScaleFactorParseError::InvalidDenominator)?;

            if denominator == 0 {
                return Err(ScaleFactorParseError::ZeroDenominator);
            }

            Ok(ScaleFactor::new(numerator, denominator))
        } else {
            // Handle whole numbers like "1", "2", etc.
            let numerator = s.parse::<u32>().map_err(|_| ScaleFactorParseError::InvalidNumerator)?;
            Ok(ScaleFactor::new(numerator, 1))
        }
    }
}

impl fmt::Display for ScaleFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

impl<'de> Deserialize<'de> for ScaleFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Error types for scale factor parsing
#[derive(Debug, Clone)]
pub enum ScaleFactorParseError {
    /// The format is not recognized (should be "n" or "n/d")
    InvalidFormat,
    /// The numerator is not a valid integer
    InvalidNumerator,
    /// The denominator is not a valid integer
    InvalidDenominator,
    /// The denominator is zero (division by zero)
    ZeroDenominator,
}

impl fmt::Display for ScaleFactorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid scale factor format"),
            Self::InvalidNumerator => write!(f, "Invalid numerator"),
            Self::InvalidDenominator => write!(f, "Invalid denominator"),
            Self::ZeroDenominator => write!(f, "Denominator cannot be zero"),
        }
    }
}

impl std::error::Error for ScaleFactorParseError {}

/// Binding of a logical texture to a shader binding point
#[derive(Debug, Clone, Deserialize)]
pub struct TextureBindingSpec {
    /// Logical texture identifier
    pub id: String,
    /// Shader binding point index
    pub binding: u32,
}

/// Texture sampling filter modes
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum SamplerFilterMode {
    /// Nearest neighbor filtering - sharp, pixelated
    #[serde(rename = "nearest")]
    Nearest,
    /// Linear interpolation filtering - smooth, blurred
    #[default]
    #[serde(rename = "linear")]
    Linear,
}

/// Binding of a texture sampler to a shader binding point
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SamplerBinding {
    /// Shader binding point index
    pub binding: u32,
    /// Filter mode for this sampler (defaults to Linear)
    #[serde(default)]
    pub filter_mode: SamplerFilterMode,
}

/// Output texture specification for a shader pass
#[derive(Debug, Clone, Deserialize)]
pub struct TextureOutput {
    /// Logical texture identifier
    pub id: String,
    /// Shader binding point index
    pub binding: u32,
    /// Number of color components in this texture
    pub components: u32,
    /// Scale factors [width_scale, height_scale] relative to input
    pub scale_factor: [ScaleFactor; 2],
}

/// A single shader pass in the pipeline
#[derive(Debug, Clone, Deserialize)]
pub struct Pass {
    /// Unique identifier for this pass
    pub id: String,
    /// Shader file path relative to manifest
    pub file: String,
    /// Input texture bindings
    pub inputs: Vec<TextureBindingSpec>,
    /// Output texture specifications
    pub outputs: Vec<TextureOutput>,
    /// Sampler bindings (optional)
    #[serde(default)]
    pub samplers: Vec<SamplerBinding>,
}

/// Raw pipeline manifest as parsed from YAML
///
/// Contains the unprocessed pipeline specification before analysis
/// and optimization.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineSpec {
    /// Unique pipeline identifier
    pub id: String,
    /// Human-readable pipeline name
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Sequence of shader passes
    pub passes: Vec<Pass>,
}

impl PipelineSpec {
    /// Parses a raw pipeline manifest from YAML content
    ///
    /// # Arguments
    /// * `yaml_content` - YAML string containing the manifest
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_norway::Error> {
        serde_norway::from_str(yaml_content)
    }

    /// Parses a raw pipeline manifest from a YAML file
    ///
    /// # Arguments
    /// * `path` - Path to the YAML manifest file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_factor_parsing() {
        // Test whole numbers
        assert_eq!("1".parse::<ScaleFactor>().unwrap(), ScaleFactor::new(1, 1));
        assert_eq!("2".parse::<ScaleFactor>().unwrap(), ScaleFactor::new(2, 1));

        // Test fractions
        assert_eq!("1/2".parse::<ScaleFactor>().unwrap(), ScaleFactor::new(1, 2));
        assert_eq!("3/4".parse::<ScaleFactor>().unwrap(), ScaleFactor::new(3, 4));

        // Test edge cases
        assert!("0/1".parse::<ScaleFactor>().is_ok());
        assert!("1/0".parse::<ScaleFactor>().is_err());
        assert!("invalid".parse::<ScaleFactor>().is_err());
    }

    #[test]
    fn test_scale_factor_properties() {
        let unity = ScaleFactor::new(1, 1);
        assert!(unity.is_unity());
        assert!(!unity.is_upscale());
        assert!(!unity.is_downscale());

        let upscale = ScaleFactor::new(2, 1);
        assert!(!upscale.is_unity());
        assert!(upscale.is_upscale());
        assert!(!upscale.is_downscale());

        let downscale = ScaleFactor::new(1, 2);
        assert!(!downscale.is_unity());
        assert!(!downscale.is_upscale());
        assert!(downscale.is_downscale());
    }

    #[test]
    fn test_raw_pipeline_parsing() {
        let yaml = r#"
id: test_pipeline
name: Test Pipeline
description: A test pipeline
passes:
  - id: pass1
    file: pass1.wgsl
    inputs:
      - id: SOURCE
        binding: 0
    outputs:
      - id: RESULT
        binding: 1
        components: 4
        scale_factor: ["2", "2"]
"#;

        let raw_pipeline = PipelineSpec::from_yaml(yaml).unwrap();
        assert_eq!(raw_pipeline.id, "test_pipeline");
        assert_eq!(raw_pipeline.name, "Test Pipeline");
        assert_eq!(raw_pipeline.passes.len(), 1);

        let pass = &raw_pipeline.passes[0];
        assert_eq!(pass.id, "pass1");
        assert_eq!(pass.file, "pass1.wgsl");
        assert_eq!(pass.inputs.len(), 1);
        assert_eq!(pass.outputs.len(), 1);

        let output = &pass.outputs[0];
        assert_eq!(output.scale_factor[0], ScaleFactor::new(2, 1));
        assert_eq!(output.scale_factor[1], ScaleFactor::new(2, 1));
    }
}
