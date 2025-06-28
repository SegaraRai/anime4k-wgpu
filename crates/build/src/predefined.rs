//! Predefined shader pipeline configurations.
//!
//! This module contains constant arrays that map human-readable pipeline names
//! to their source file paths. These are used by the build process to generate
//! optimized, embeddable pipeline objects.

/// A list of predefined auxiliary pipelines, mapping a name to its WGSL manifest file.
///
/// These pipelines are typically used for pre- and post-processing tasks such as
/// color correction, deblurring, and denoising. They are defined using a YAML
/// manifest format that specifies the sequence of shader passes and their inputs/outputs.
pub const PREDEFINED_PIPELINES_AUX: &[(&str, &str)] = &[
    // Image processing utilities
    ("CLAMP_HIGHLIGHTS", "wgsl/auxiliary/clamp_highlights_manifest.yaml"),
    // Deblur algorithms
    ("DEBLUR_DOG", "wgsl/auxiliary/deblur_dog_manifest.yaml"),
    ("DEBLUR_ORIGINAL", "wgsl/auxiliary/deblur_original_manifest.yaml"),
    // Denoise algorithms with different statistical approaches
    ("DENOISE_BILATERAL_MEAN", "wgsl/auxiliary/denoise_bilateral_mean_manifest.yaml"),
    ("DENOISE_BILATERAL_MEDIAN", "wgsl/auxiliary/denoise_bilateral_median_manifest.yaml"),
    ("DENOISE_BILATERAL_MODE", "wgsl/auxiliary/denoise_bilateral_mode_manifest.yaml"),
    // Visual effects with different performance profiles
    ("EFFECTS_DARKEN_HQ", "wgsl/auxiliary/effects_darken_manifest_hq.yaml"),
    ("EFFECTS_DARKEN_FAST", "wgsl/auxiliary/effects_darken_manifest_fast.yaml"),
    ("EFFECTS_DARKEN_VERYFAST", "wgsl/auxiliary/effects_darken_manifest_veryfast.yaml"),
    ("EFFECTS_THIN_HQ", "wgsl/auxiliary/effects_thin_manifest_hq.yaml"),
    ("EFFECTS_THIN_FAST", "wgsl/auxiliary/effects_thin_manifest_fast.yaml"),
    ("EFFECTS_THIN_VERYFAST", "wgsl/auxiliary/effects_thin_manifest_veryfast.yaml"),
    // Alternative upscaling algorithms
    ("UPSCALE_DOG_X2", "wgsl/auxiliary/upscale_dog_x2_manifest.yaml"),
    ("UPSCALE_ORIGINAL_X2", "wgsl/auxiliary/upscale_original_x2_manifest.yaml"),
];

/// A list of predefined CNN (Convolutional Neural Network) pipelines, mapping a name to its GLSL source file.
///
/// These pipelines are primarily used for advanced image restoration and upscaling tasks.
/// They are sourced from GLSL files compatible with the mpv player'''s hook format and are
/// converted to WGSL during the build process. The collection includes various models
/// with different quality and performance characteristics.
pub const PREDEFINED_PIPELINES_CNN: &[(&str, &str)] = &[
    // Restore variants - improve image quality without upscaling
    ("RESTORE_CNN_S", "anime4k-glsl/Restore/Anime4K_Restore_CNN_S.glsl"),
    ("RESTORE_CNN_M", "anime4k-glsl/Restore/Anime4K_Restore_CNN_M.glsl"),
    ("RESTORE_CNN_L", "anime4k-glsl/Restore/Anime4K_Restore_CNN_L.glsl"),
    ("RESTORE_CNN_VL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_VL.glsl"),
    ("RESTORE_CNN_UL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_UL.glsl"),
    // Restore GAN variants - generative adversarial network restoration
    ("RESTORE_GAN_UL", "anime4k-glsl/Restore/Anime4K_Restore_GAN_UL.glsl"),
    ("RESTORE_GAN_UUL", "anime4k-glsl/Restore/Anime4K_Restore_GAN_UUL.glsl"),
    // Restore Soft variants - gentler restoration algorithms
    ("RESTORE_SOFT_CNN_S", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_S.glsl"),
    ("RESTORE_SOFT_CNN_M", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_M.glsl"),
    ("RESTORE_SOFT_CNN_L", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_L.glsl"),
    ("RESTORE_SOFT_CNN_VL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_VL.glsl"),
    ("RESTORE_SOFT_CNN_UL", "anime4k-glsl/Restore/Anime4K_Restore_CNN_Soft_UL.glsl"),
    // Upscale variants - 2x upscaling with different quality levels
    ("UPSCALE_CNN_X2_S", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_S.glsl"),
    ("UPSCALE_CNN_X2_M", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_M.glsl"),
    ("UPSCALE_CNN_X2_L", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_L.glsl"),
    ("UPSCALE_CNN_X2_VL", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_VL.glsl"),
    ("UPSCALE_CNN_X2_UL", "anime4k-glsl/Upscale/Anime4K_Upscale_CNN_x2_UL.glsl"),
    // Upscale GAN variants - generative adversarial network upscaling
    ("UPSCALE_GAN_X2_S", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x2_S.glsl"),
    ("UPSCALE_GAN_X2_M", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x2_M.glsl"),
    ("UPSCALE_GAN_X3_L", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x3_L.glsl"),
    ("UPSCALE_GAN_X3_VL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x3_VL.glsl"),
    ("UPSCALE_GAN_X4_UL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x4_UL.glsl"),
    ("UPSCALE_GAN_X4_UUL", "anime4k-glsl/Upscale/Anime4K_Upscale_GAN_x4_UUL.glsl"),
    // Upscale + Denoise variants - combined upscaling and noise reduction
    ("UPSCALE_DENOISE_CNN_X2_S", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_S.glsl"),
    ("UPSCALE_DENOISE_CNN_X2_M", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_M.glsl"),
    ("UPSCALE_DENOISE_CNN_X2_L", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_L.glsl"),
    ("UPSCALE_DENOISE_CNN_X2_VL", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_VL.glsl"),
    ("UPSCALE_DENOISE_CNN_X2_UL", "anime4k-glsl/Upscale+Denoise/Anime4K_Upscale_Denoise_CNN_x2_UL.glsl"),
    // 3D Graphics variants - specialized for 3D rendered content
    ("UPSCALE_3DCG_CNN_X2_US", "anime4k-glsl/Upscale/Anime4K_3DGraphics_Upscale_x2_US.glsl"),
    ("UPSCALE_3DCG_AA_CNN_X2_US", "anime4k-glsl/Upscale/Anime4K_3DGraphics_AA_Upscale_x2_US.glsl"),
];
