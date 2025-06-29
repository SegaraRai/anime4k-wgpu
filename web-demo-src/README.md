# Anime4K-wgpu Web Demo

This directory contains the TypeScript source code for the Anime4K-wgpu web demonstration application.

## Overview

This web application provides an interactive demonstration of the Anime4K upscaling algorithms running in the browser using WebGPU. It allows users to apply real-time Anime4K video upscaling and filtering to video files directly in their browser with a modern, responsive user interface.

### Features

- **Video File Support**: Drag-and-drop or browse to load any video file supported by your browser
- **Real-time Processing**: Apply Anime4K algorithms in real-time using WebGPU compute shaders
- **Multiple Presets**: Choose from 6 different Anime4K presets:
  - A (Restore → Upscale)
  - B (Restore Soft → Upscale)
  - C (Upscale Denoise)
  - AA (Restore → Upscale → Restore)
  - BB (Restore Soft → Upscale → Restore Soft)
  - CA (Upscale Denoise → Restore)
- **Performance Levels**: Select from 5 performance tiers (Light to Extreme) balancing speed vs quality
- **Flexible Scaling**: Automatically scale videos up to 8x with configurable scale factors
- **Comparison Modes**: View before/after comparisons with split-screen or overlay modes
- **Keyboard Controls**: Full keyboard navigation and shortcuts for efficient operation
- **Modern UI**: Responsive design with dark/light theme support using daisyUI

### Keyboard Shortcuts

- **Space/Enter**: Play/Pause
- **F**: Toggle fullscreen
- **C**: Toggle comparison mode (Shift+C to cycle backwards)
- **Ctrl+0**: Disable Anime4K processing
- **Ctrl+1-6**: Switch between Anime4K presets (A, B, C, AA, BB, CA)
- **Shift+1-5**: Switch performance levels (Light, Medium, High, Ultra, Extreme)

## Technical Implementation

This demo implements the Anime4K algorithms entirely in the browser using WebGPU, without requiring the native `anime4k-wgpu` or `wgpu` Rust crates.

### Architecture

1. **Pipeline Serialization**: The shader pipelines from the `anime4k-wgpu` crate are serialized to `anime4k/predefinedPipelines.json`, containing all shader passes, textures, and dependencies.
2. **WebGPU Reconstruction**: The web application dynamically reconstructs WebGPU compute and render pipelines using the browser's native WebGPU API, loading WGSL shaders and binding resources.
3. **Preset Logic**: Processing presets are re-implemented in TypeScript (`anime4k/presets.ts`) to chain pipelines in the correct sequence matching the native library behavior.
4. **Video Processing**: HTML5 video frames are copied to WebGPU textures using `copyExternalImageToTexture()`, processed through the Anime4K pipeline, and rendered to canvas.

### Technology Stack

- **Astro**: Static site generator with component islands
- **Preact**: Lightweight React alternative for interactive components
- **TypeScript**: Type-safe JavaScript with WebGPU type definitions
- **WebGPU**: Modern graphics API for compute and rendering
- **Tailwind CSS**: Utility-first CSS framework
- **daisyUI**: Component library built on Tailwind CSS
- **WGSL**: WebGPU Shading Language for compute shaders

## Project Structure

```text
web-demo-src/
├── anime4k/                     # Core Anime4K implementation
│   ├── executor.ts              # Pipeline execution engine
│   ├── player.ts                # Video player with Anime4K integration
│   ├── presets.ts               # Preset configuration logic
│   ├── predefinedPipelines.json # Serialized shader pipelines
│   └── render.wgsl              # WebGPU render shader
├── components/                  # Preact UI components
│   ├── VideoPlayerPage.tsx      # Main application page
│   ├── VideoPlayer.tsx          # Video player component
│   ├── VideoControls.tsx        # Player controls UI
│   ├── Toast.tsx                # Notification system
│   ├── constants.ts             # Application constants
│   └── useToast.ts              # Toast hook
├── layouts/                     # Astro layouts
│   └── Layout.astro             # Base page layout
├── pages/                       # Astro pages
│   └── index.astro              # Application entry point
└── global.css                   # Global styles
```

## Development

### Prerequisites

- **Node.js** (18+)
- **pnpm** (recommended package manager)
- **WebGPU-compatible browser** (Chrome 113+, Firefox Nightly, Safari Technology Preview)

### Setup

```bash
# Install dependencies
pnpm install

# Start development server
pnpm dev

# Build for production
pnpm build

# Preview production build
pnpm preview
```

### Browser Requirements

- **WebGPU Support**: The application requires WebGPU API support
- **Feature Detection**: The app will display compatibility warnings for unsupported browsers
- **Recommended Browsers**:
  - Chrome/Edge 113+ (stable)
  - Firefox with `dom.webgpu.enabled` flag
  - Safari Technology Preview

## Performance Considerations

- **GPU Memory**: Higher scale factors and quality settings require more GPU memory
- **Real-time Processing**: Performance varies based on GPU capabilities and video resolution
- **Automatic Scaling**: The app automatically selects appropriate scale factors based on viewport size
- **Background Processing**: WebGPU compute shaders run asynchronously for smooth playback

## Limitations

- **Video Codec Support**: Limited to codecs supported by the browser's `<video>` element
- **WebGPU Availability**: Not supported on older devices or browsers without WebGPU
- **Mobile Support**: Limited support on mobile devices due to WebGPU availability

## License

This project follows the same license as the parent `anime4k-wgpu` repository.
