//! Anime4K Video Player Example
//!
//! A hardware-accelerated video player that demonstrates the use of Anime4K upscaling
//! with Vulkan Video decoding. This example showcases real-time video processing with
//! GPU-based decoding and upscaling capabilities.
//!
//! # Features
//! - Hardware-accelerated H.264 video decoding using Vulkan Video
//! - Real-time Anime4K upscaling for improved video quality
//! - Multi-threaded architecture with separate decoder and renderer threads
//! - Interactive playback controls (pause/unpause)
//!
//! # Requirements
//! - Vulkan-capable GPU with video decode support
//! - H.264 video files for input
//!
//! # Usage
//! ```bash
//! cargo run --example player -- video.h264 60 [--paused]
//! ```

/// Application event handling and user interface
#[cfg(vulkan)]
mod app;

/// Hardware video decoding with Vulkan Video
#[cfg(vulkan)]
mod decoder;

/// Core video playback and rendering pipeline
#[cfg(vulkan)]
mod player;

/// Main entry point for Vulkan-enabled builds
///
/// Runs the video player application when Vulkan support is available.
/// This is the primary entry point that most users will encounter.
#[cfg(vulkan)]
fn main() -> Result<(), winit::error::EventLoopError> {
    use crate::app::VideoPlayerApp;
    use clap::Parser;
    use std::path::PathBuf;
    use winit::event_loop::{ControlFlow, EventLoop};

    /// Command-line arguments for the video player
    ///
    /// Defines the interface for controlling video playback parameters
    /// including input file, framerate, and initial playback state.
    #[derive(Parser)]
    #[command(version, about, long_about=None)]
    pub struct Args {
        /// Path to the video file to play (.h264)
        filename: PathBuf,

        /// Framerate of the video in frames per second
        framerate: u32,

        /// Start the video player in paused state
        #[arg(long, short)]
        paused: bool,
    }

    let args = Args::parse();

    // Set up logging for debugging and monitoring
    let subscriber = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!("Starting video player...");

    // Create window and event loop for user interface
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Initialize and run the video player application
    let mut app = VideoPlayerApp::new(&args.filename, args.framerate, args.paused);
    event_loop.run_app(&mut app)
}

/// Fallback main function for non-Vulkan platforms
///
/// Displays an informative error message when Vulkan support is not available.
/// This ensures the application fails gracefully on unsupported platforms.
#[cfg(not(vulkan))]
fn main() {
    println!("This crate doesn't work on your operating system, because it does not support vulkan");
}
