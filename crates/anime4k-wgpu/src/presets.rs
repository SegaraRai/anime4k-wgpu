//! Anime4K quality and performance preset configurations
//!
//! This module provides predefined combinations of Anime4K algorithms and
//! performance levels for common use cases.

use crate::{
    ExecutablePipeline,
    pipelines::{aux, cnn},
};

/// Performance presets that control the computational complexity of the upscaling process
///
/// Each preset uses different CNN model sizes, trading quality for performance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anime4KPerformancePreset {
    /// Fastest processing with smallest models
    Light,
    /// Balanced performance and quality
    Medium,
    /// Higher quality with moderate performance impact
    High,
    /// Very high quality with significant performance cost
    Ultra,
    /// Maximum quality with highest performance cost
    Extreme,
}

impl Anime4KPerformancePreset {
    /// Returns the human-readable name of this performance preset
    pub fn name(&self) -> &'static str {
        match self {
            Anime4KPerformancePreset::Light => "Light",
            Anime4KPerformancePreset::Medium => "Medium",
            Anime4KPerformancePreset::High => "High",
            Anime4KPerformancePreset::Ultra => "Ultra",
            Anime4KPerformancePreset::Extreme => "Extreme",
        }
    }

    /// Returns the restore CNN pipeline for the initial pass
    fn for_initial_restore(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::RESTORE_CNN_S,
            Anime4KPerformancePreset::Medium => &cnn::RESTORE_CNN_M,
            Anime4KPerformancePreset::High => &cnn::RESTORE_CNN_L,
            Anime4KPerformancePreset::Ultra => &cnn::RESTORE_CNN_VL,
            Anime4KPerformancePreset::Extreme => &cnn::RESTORE_CNN_UL,
        }
    }

    /// Returns the soft restore CNN pipeline for the initial pass
    fn for_initial_restore_soft(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::RESTORE_SOFT_CNN_S,
            Anime4KPerformancePreset::Medium => &cnn::RESTORE_SOFT_CNN_M,
            Anime4KPerformancePreset::High => &cnn::RESTORE_SOFT_CNN_L,
            Anime4KPerformancePreset::Ultra => &cnn::RESTORE_SOFT_CNN_VL,
            Anime4KPerformancePreset::Extreme => &cnn::RESTORE_SOFT_CNN_UL,
        }
    }

    /// Returns the upscale+denoise CNN pipeline for the initial 2x upscaling pass
    fn for_initial_upscale_denoise_2x(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::UPSCALE_DENOISE_CNN_X2_S,
            Anime4KPerformancePreset::Medium => &cnn::UPSCALE_DENOISE_CNN_X2_M,
            Anime4KPerformancePreset::High => &cnn::UPSCALE_DENOISE_CNN_X2_L,
            Anime4KPerformancePreset::Ultra => &cnn::UPSCALE_DENOISE_CNN_X2_VL,
            Anime4KPerformancePreset::Extreme => &cnn::UPSCALE_DENOISE_CNN_X2_UL,
        }
    }

    /// Returns the restore CNN pipeline for subsequent passes (typically smaller models)
    fn for_subsequent_restore(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::RESTORE_CNN_S,
            Anime4KPerformancePreset::Medium => &cnn::RESTORE_CNN_S,
            Anime4KPerformancePreset::High => &cnn::RESTORE_CNN_M,
            Anime4KPerformancePreset::Ultra => &cnn::RESTORE_CNN_L,
            Anime4KPerformancePreset::Extreme => &cnn::RESTORE_CNN_L,
        }
    }

    /// Returns the soft restore CNN pipeline for subsequent passes
    fn for_subsequent_restore_soft(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::RESTORE_SOFT_CNN_S,
            Anime4KPerformancePreset::Medium => &cnn::RESTORE_SOFT_CNN_S,
            Anime4KPerformancePreset::High => &cnn::RESTORE_SOFT_CNN_M,
            Anime4KPerformancePreset::Ultra => &cnn::RESTORE_SOFT_CNN_L,
            Anime4KPerformancePreset::Extreme => &cnn::RESTORE_SOFT_CNN_L,
        }
    }

    /// Returns the upscale CNN pipeline for the initial 2x upscaling pass
    fn for_initial_upscale_2x(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::UPSCALE_CNN_X2_S,
            Anime4KPerformancePreset::Medium => &cnn::UPSCALE_CNN_X2_M,
            Anime4KPerformancePreset::High => &cnn::UPSCALE_CNN_X2_L,
            Anime4KPerformancePreset::Ultra => &cnn::UPSCALE_CNN_X2_VL,
            Anime4KPerformancePreset::Extreme => &cnn::UPSCALE_CNN_X2_UL,
        }
    }

    /// Returns the upscale CNN pipeline for subsequent 2x upscaling passes
    fn for_subsequent_upscale_2x(&self) -> &'static ExecutablePipeline {
        match self {
            Anime4KPerformancePreset::Light => &cnn::UPSCALE_CNN_X2_S,
            Anime4KPerformancePreset::Medium => &cnn::UPSCALE_CNN_X2_S,
            Anime4KPerformancePreset::High => &cnn::UPSCALE_CNN_X2_M,
            Anime4KPerformancePreset::Ultra => &cnn::UPSCALE_CNN_X2_L,
            Anime4KPerformancePreset::Extreme => &cnn::UPSCALE_CNN_X2_L,
        }
    }
}

/// Anime4K algorithm presets that define the processing pipeline
///
/// Each mode represents a different approach to upscaling with varying
/// characteristics for different types of content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anime4KPreset {
    /// No processing applied
    Off,
    /// Standard restore then upscale - good for most anime content
    ModeA,
    /// Mode A with additional restore pass - higher quality
    ModeAA,
    /// Soft restore then upscale - gentler processing
    ModeB,
    /// Mode B with additional restore pass - gentler high quality
    ModeBB,
    /// Combined upscale and denoise - efficient for noisy content
    ModeC,
    /// Mode C with additional restore pass - denoise then restore
    ModeCA,
}

impl Anime4KPreset {
    /// Returns the human-readable name of this preset
    pub fn name(&self) -> &'static str {
        match self {
            Anime4KPreset::Off => "OFF",
            Anime4KPreset::ModeA => "Mode A",
            Anime4KPreset::ModeAA => "Mode AA",
            Anime4KPreset::ModeB => "Mode B",
            Anime4KPreset::ModeBB => "Mode BB",
            Anime4KPreset::ModeC => "Mode C",
            Anime4KPreset::ModeCA => "Mode CA",
        }
    }

    /// Creates the complete processing pipeline for this preset
    ///
    /// Builds a sequence of executable pipelines that implement the chosen Anime4K algorithm.
    /// Additional upscaling passes are automatically added until the target scale factor is reached.
    ///
    /// # Arguments
    /// * `performance_preset` - Controls the computational complexity and model sizes used
    /// * `target_scale_factor` - Desired output scale factor (e.g., 2.0 for 2x upscaling)
    ///
    /// # Returns
    /// A vector of executable pipelines that should be run in sequence
    pub fn create_pipelines(&self, performance_preset: Anime4KPerformancePreset, target_scale_factor: f64) -> Vec<&'static ExecutablePipeline> {
        let mut base = match self {
            Anime4KPreset::Off => return vec![],
            Anime4KPreset::ModeA => vec![&aux::CLAMP_HIGHLIGHTS, performance_preset.for_initial_restore(), performance_preset.for_initial_upscale_2x()],
            Anime4KPreset::ModeB => vec![&aux::CLAMP_HIGHLIGHTS, performance_preset.for_initial_restore_soft(), performance_preset.for_initial_upscale_2x()],
            Anime4KPreset::ModeC => vec![&aux::CLAMP_HIGHLIGHTS, performance_preset.for_initial_upscale_denoise_2x()],
            Anime4KPreset::ModeAA => vec![
                &aux::CLAMP_HIGHLIGHTS,
                performance_preset.for_initial_restore(),
                performance_preset.for_initial_upscale_2x(),
                performance_preset.for_subsequent_restore(),
            ],
            Anime4KPreset::ModeBB => vec![
                &aux::CLAMP_HIGHLIGHTS,
                performance_preset.for_initial_restore_soft(),
                performance_preset.for_initial_upscale_2x(),
                performance_preset.for_subsequent_restore_soft(),
            ],
            Anime4KPreset::ModeCA => vec![&aux::CLAMP_HIGHLIGHTS, performance_preset.for_initial_upscale_denoise_2x(), performance_preset.for_subsequent_restore()],
        };

        let mut current_scale_factor = 2.0;
        while current_scale_factor < target_scale_factor {
            base.push(performance_preset.for_subsequent_upscale_2x());
            current_scale_factor *= 2.0;
        }

        base
    }
}
