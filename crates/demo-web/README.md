# Anime4K-wgpu Web Video Player

A web-based video player with real-time Anime4K upscaling using WebGPU external video textures.

## Features

- Real-time video playback in the browser
- GPU-accelerated Anime4K upscaling using WebGPU compute shaders
- WebGPU external video texture integration for optimal performance
- Interactive Anime4K preset selection (Mode A through CA)
- Performance preset adjustment (Light to Extreme)
- Support for all browser-supported video formats

## Building

### Prerequisites

- Rust with `wasm32-unknown-unknown` target installed
- `wasm-pack` for building WASM packages

Install the required tools:

```bash
# Install the wasm32 target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Build the Demo

```bash
# Navigate to the demo-web directory
cd crates/demo-web

# Build the WASM package
wasm-pack build --target web --out-dir pkg

# Serve the demo (using any HTTP server)
# Option 1: Using Python
python -m http.server 8000

# Option 2: Using Node.js http-server
npx http-server

# Option 3: Using Rust basic-http-server
cargo install basic-http-server
basic-http-server
```

Then open your browser to `http://localhost:8000` (or the port shown by your server).

## Usage

1. Open the web demo in a WebGPU-compatible browser
2. Click "Choose a video file" and select a video file (MP4, WebM, etc.)
3. Use the Play/Pause controls to control video playback
4. Select an Anime4K preset (different modes optimized for different input resolutions)
5. Adjust the performance preset to balance quality vs. processing speed
6. The enhanced video will be displayed in real-time

## Browser Compatibility

### WebGPU Support

- **Chrome 113+**: Full WebGPU support
- **Firefox 113+**: Experimental WebGPU support (needs to be enabled)
- **Safari**: WebGPU support in development

### WebGL2 Fallback

- Most modern browsers support WebGL2 as a fallback
- Requires GPU with compute shader support for Anime4K processing

## Technical Implementation

### WebGPU External Video Textures

The player uses WebGPU's external video texture feature to efficiently process video frames:

```rust
// Create external texture from video element (conceptual)
let external_texture = device.import_external_texture(&video_element);

// Use in compute shader for Anime4K processing
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&external_texture.create_view()),
    }],
    // ...
});
```

### Shader Integration

Video textures use `texture_external` type in WGSL:

```wgsl
@group(0) @binding(0) var t_video: texture_external;
@group(0) @binding(1) var s_video: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleBaseClampToEdge(t_video, s_video, in.tex_coords);
}
```

### Performance Optimization

- **Zero-copy video processing**: Video frames processed directly on GPU
- **Compute shader acceleration**: Anime4K algorithms run as compute shaders
- **Efficient memory usage**: No CPU-GPU memory transfers for video data
- **Hardware decoding**: Leverages browser's native video decoder

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTML Video    │───▶│  WebGPU External │───▶│   Anime4K       │
│   Element       │    │  Video Texture   │    │   Compute       │
└─────────────────┘    └──────────────────┘    │   Shaders       │
                                               └─────────────────┘
                                                        │
                                               ┌─────────────────┐
                                               │   Final Render  │
                                               │   to Canvas     │
                                               └─────────────────┘
```

## Supported Video Formats

The player supports any video format that the browser can decode natively:

- **MP4**: H.264, H.265, AV1
- **WebM**: VP8, VP9, AV1
- **OGV**: Theora
- **MOV**: QuickTime formats
- **MKV**: Various codecs

## Development Status

⚠️ **Note**: This is a development version. WebGPU external video texture support in wgpu-rs for web targets is still evolving. The current implementation provides the foundation for full video texture integration as WebGPU features mature.

### Current Limitations

- External video texture binding requires additional WebGPU API maturation
- Some features may require browser flags to be enabled
- Performance may vary based on GPU and browser implementation

### Future Enhancements

- Full external video texture integration when wgpu-rs web support matures
- Video seeking and timeline controls
- Multiple video format optimizations
- WebCodecs integration for advanced video processing

## Anime4K Presets

### Mode Selection

- **Mode A**: Optimized for 1080p anime content
- **Mode AA**: Mode A with enhanced line art processing
- **Mode B**: Optimized for 720p anime content
- **Mode BB**: Mode B with enhanced line art processing
- **Mode C**: Optimized for 480p anime content
- **Mode CA**: Mode C with enhanced line art processing

### Performance Levels

- **Light**: Fastest processing, good for real-time preview
- **Medium**: Balanced speed and quality
- **High**: Better quality, moderate performance impact
- **Ultra**: High quality processing
- **Extreme**: Maximum quality, highest performance requirements

## Contributing

This web player is part of the larger Anime4K-wgpu project. See the main project README for contribution guidelines.
