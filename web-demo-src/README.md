# Anime4K-wgpu Web Demo Source

This directory contains the TypeScript source code for the Anime4K-wgpu web demo.

## Overview

This web application provides an interactive demonstration of the Anime4K-wgpu library, allowing users to apply real-time Anime4K video upscaling and filtering to video files directly in their browser. It is designed to replicate the core functionality of the native video player example found in `crates/anime4k-wgpu/examples/player`, but with a graphical user interface instead of keyboard shortcuts.

Users can:

- Load a local video file.
- Play and pause the video.
- Select different Anime4K upscaling and processing presets.
- Choose from various performance vs. quality settings.
- See the visual difference between the original and the processed video in real-time.

## Technical Implementation

This demo showcases the Anime4K algorithms running in the browser using WebGPU. However, it does not use the `anime4k-wgpu` or `wgpu` Rust crates directly.

Due to current web limitations and interoperability issues with using HTML `<video>` elements as textures in WebGPU (especially within `wgpu`), a different approach was taken:

1. **Pipeline Extraction**: The processing pipelines defined in the `anime4k-wgpu` crate were executed, and their internal structure was serialized to a JSON file (`components/predefinedPipelines.json`). This file contains the definitions for all the shader passes, their inputs, outputs, and interdependencies.
2. **Pipeline Reconstruction**: The web application loads and parses this JSON file to dynamically reconstruct the necessary WebGPU compute and render pipelines in the browser using TypeScript and the browser's native WebGPU API.
3. **Preset Re-implementation**: The logic for the predefined processing presets (e.g., "Mode A (Upscale)", "Mode C (Upscale + Deblur)") is re-implemented in TypeScript. This logic chains the reconstructed pipelines together in the correct sequence to match the behavior of the native library.

This method allows the demo to accurately replicate the behavior and visual output of the native `anime4k-wgpu` library while working around the current constraints of video texture handling in WebGPU on the web.
