# Anime4K-wgpu

A high-performance WGSL/wgpu port of [Anime4K](https://github.com/bloc97/Anime4K), an AI-based image and video upscaling algorithm optimized for anime content. This implementation leverages modern GPU compute capabilities through WebGPU and WGSL shaders.

## Overview

Anime4K-wgpu is a complete reimplementation of the original Anime4K GLSL shaders using:

- **WGSL** (WebGPU Shading Language) for compute shaders
- **wgpu** as the graphics API abstraction layer
- **Rust** for the core implementation and build system

The project supports both CNN/GAN-based neural network upscaling and traditional auxiliary shaders for image enhancement tasks like deblurring, denoising, and effects processing.

## Features

### Supported Shaders

- ✅ **All CNN/GAN-based neural network shaders** (automatically converted)
- ✅ **Most auxiliary shaders** including:
  - Deblur (DoG, Original)
  - Denoise (Bilateral Mean/Median/Mode)
  - Experimental Effects (Darken, Thin)
  - Restore (Clamp Highlights)
  - Upscale (DoG, DTD, Original)
- ❌ **Unsupported auxiliary shaders**: `Anime4K_Upscale_Deblur_DoG_x2.glsl`, `Anime4K_Upscale_Deblur_Original_x2.glsl`, `Anime4K_Upscale_DTD_x2.glsl`

### Key Capabilities

- **Static image upscaling** with CLI tool
- **Video upscaling and playback** (H.264 support)
- **Multiple quality presets**: Light, Medium, High, Ultra, Extreme
- **Various Anime4K modes**: A, AA, B, BB, C, CA
- **Behavior verification** against original GLSL implementation
- **Bug fixes** for coordinate calculations and alpha channel handling

## Project Structure

```text
anime4k-wgpu/
├── anime4k-glsl/         # Original GLSL shaders from Anime4K
├── wgsl/                 # Manually ported auxiliary shaders
│   ├── auxiliary/        # Auxiliary shader implementations
│   ├── helpers/          # Helper WGSL shaders
│   └── wip/              # Work-in-progress shaders
├── crates/
│   ├── anime4k-wgpu/     # Main library and examples
│   │   ├── examples/     # CLI and video player applications
│   │   └── src/          # Core library implementation
│   ├── build/            # Build system for shader conversion
│   └── verification/     # GLSL runtime emulation for testing
└── example_image.png     # Sample test image with alpha channel
```

## Quick Start

### Prerequisites

- Rust (2024 edition)
- A GPU that supports filtering 32-bit floating point textures (the `"float32-filterable"` feature in WebGPU)  
  While it is technically possible to support GPUs without this feature by downgrading some or all textures to 16-bit floats, there are no plans to implement this due to the complexity involved.
- For video playback: A Vulkan-compatible GPU

### Installation

```bash
git clone https://github.com/SegaraRai/anime4k-wgpu.git
cd anime4k-wgpu
cargo build --release
```

### Basic Usage

#### CLI Image Upscaling

```bash
# Basic 2x upscaling with provided test image
cargo run --release --example cli example_image.png output.png

# Custom scale factor and presets
cargo run --release --example cli input.png output.png --scale-factor 4.0 --preset aa --performance ultra
```

**Available options:**

- **Presets**: `a`, `aa`, `b`, `bb`, `c`, `ca`
- **Performance**: `light`, `medium`, `high`, `ultra`, `extreme`

#### Video Player (Vulkan only)

```bash
# Play H.264 video with upscaling
cargo run --release --example player trapezium.h264 30

# Start paused
cargo run --release --example player video.h264 30 --paused
```

**Keyboard Shortcuts:**

- **Esc**: Exit player
- **Space**: Toggle pause
- **Ctrl+0**: Disable Anime4K
- **Ctrl+1-6**: Set Anime4K preset (A, AA, B, BB, C, CA)
- **Shift+1-5**: Set performance preset (Light, Medium, High, Ultra, Extreme)

## Architecture

### Build System

The build system in `crates/build/` translates original GLSL shaders to WGSL compute shaders for wgpu compatibility. It parses mpv hook directives (`//!DESC`, `//!BIND`, `//!SAVE`) and converts GLSL shader code while preserving the embedded neural network weights as matrix literals. The system handles two main types:

- **CNN/GAN shaders**: Direct GLSL-to-WGSL translation with convolutional operations and ReLU activations
- **Auxiliary shaders**: Hand-written WGSL with YAML manifests defining multi-pass pipelines

All shader code is embedded into the compiled binary, eliminating runtime file dependencies.

### Pipeline Architecture

Two distinct pipeline types handle different upscaling approaches:

- **Neural network pipelines**: Converted from GLSL files, ranging from 4-pass lightweight models to 25-pass ultra-quality sequences. Each pass applies learned convolutional operations with embedded weights.
- **Auxiliary pipelines**: Traditional image processing (deblur, denoise, effects) using multi-pass algorithms with operations like gaussian filtering and edge detection.

The system optimizes GPU memory through texture lifetime analysis and supports flexible resolution scaling. Compute shaders use 8x8 workgroups for optimal utilization.

### Verification System

The verification system in `crates/verification/` ensures conversion accuracy through dual reference engines:

- **GLSL reference engine**: Executes original shaders in recreated mpv hook environment
- **WGSL reference engine**: Runs converted implementations using the same pipeline system

Pixel-level comparisons identify discrepancies, with intermediate texture output for debugging specific pipeline stages.

## Testing and Debugging

**Performance Note:** Always use the `--release` flag for optimal performance, as debug builds can be significantly slower for GPU-intensive operations.

### Conformance Testing

The conformance tests in the `crates/verification/` directory are **not** executed by `cargo test`. Instead, use the dedicated verification binaries to ensure shader output matches the original GLSL implementation:

#### Test Images

- **example_image.png**: A Gemini-generated illustration with semi-transparent background, ideal for testing alpha channel handling
- Any PNG image with or without alpha channel can be used as input

#### CNN/GAN Shader Verification

```bash
# Verify all CNN-based neural network shaders using the provided test image
cargo run --release -p anime4k-wgpu-verification --bin verify_cnn example_image.png

# Or use your own image
cargo run --release -p anime4k-wgpu-verification --bin verify_cnn input.png
```

#### Auxiliary Shader Verification

```bash
# Verify auxiliary shaders (deblur, denoise, effects, etc.) using the provided test image
cargo run --release -p anime4k-wgpu-verification --bin verify_aux example_image.png

# Or use your own image
cargo run --release -p anime4k-wgpu-verification --bin verify_aux input.png
```

### Debugging Shader Discrepancies

If verification reveals discrepancies between GLSL and WGSL implementations, you can debug by examining the output of individual passes:

#### GLSL Reference Engine

```bash
# Run original GLSL shader and output intermediate results
cargo run --release -p anime4k-wgpu-verification --bin glsl_reference_engine shader.glsl input.png output.png
```

#### WGSL Reference Engine

```bash
# Run WGSL implementation and output intermediate results
cargo run --release -p anime4k-wgpu-verification --bin wgsl_reference_engine manifest.yaml input.png output.png
```

### Debugging Workflow

1. **Run verification** to identify which shaders have discrepancies
2. **Use reference engines** to output intermediate passes for both GLSL and WGSL
3. **Compare outputs** pass-by-pass to isolate the problematic stage
4. **Examine shader code** and manifest files for the failing pass
5. **Test fixes** by re-running verification

### Regular Testing

```bash
# Standard unit tests (excludes conformance tests)
cargo test

# Test specific crate
cargo test -p anime4k-wgpu

# Test with release optimizations
cargo test --release
```

## License

This project is licensed under the MIT License (see [LICENSE](LICENSE) for details).

This project builds upon the original Anime4K shaders by bloc97, which are licensed under the MIT License (see [anime4k-glsl/LICENSE](anime4k-glsl/LICENSE)).

## Acknowledgments

- **bloc97** and contributors for the original [Anime4K](https://github.com/bloc97/Anime4K) algorithm
- **[Anime4K-WebGPU](https://github.com/Anime4KWebBoost/Anime4K-WebGPU) authors** for the reference implementation of WGSL shaders
- **gfx-rs team** for the excellent wgpu implementation
- **WebGPU Working Group** for the WebGPU specification and WGSL
