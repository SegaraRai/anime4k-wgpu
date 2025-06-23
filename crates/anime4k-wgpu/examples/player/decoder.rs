//! Video Decoder Module
//!
//! This module provides hardware-accelerated video decoding using Vulkan Video.
//! It decodes video frames from a byte stream and outputs them as wgpu textures
//! with presentation timestamps for playback synchronization.

use bytes::BytesMut;
use std::{
    io::Read,
    sync::{Arc, mpsc::SyncSender},
    time::Duration,
};
use vk_video::{EncodedChunk, Frame, VulkanDevice};

/// A decoded video frame with presentation timestamp
///
/// Contains a decoded frame as a wgpu texture along with its presentation
/// timestamp for proper playback timing and synchronization.
pub struct FrameWithPts {
    /// The decoded video frame as a wgpu texture
    ///
    /// This texture can be directly used for rendering or further processing
    /// with wgpu compute or render pipelines.
    pub frame: wgpu::Texture,
    /// Presentation timestamp indicating when this frame should be displayed
    ///
    /// Used for synchronizing video playback with the target framerate.
    /// Calculated based on frame number and the video's framerate.
    pub pts: Duration,
}

/// Runs the video decoder in a dedicated thread
///
/// This function performs hardware-accelerated video decoding using Vulkan Video.
/// It reads encoded video data from a byte stream, decodes it frame by frame,
/// and sends the decoded frames with presentation timestamps through a channel.
///
/// The decoder operates in a streaming fashion:
/// 1. Reads chunks of encoded data from the input stream
/// 2. Feeds them to the Vulkan Video decoder
/// 3. Receives decoded frames as wgpu textures
/// 4. Calculates presentation timestamps based on framerate
/// 5. Sends frames through the provided channel for consumption
///
/// # Arguments
/// * `tx` - Channel sender for transmitting decoded frames with timestamps
/// * `framerate` - Target framerate in frames per second for timestamp calculation
/// * `vulkan_device` - Vulkan device instance for creating the decoder
/// * `bytestream_reader` - Input stream containing encoded video data
///
/// # Behavior
/// - Continues reading until the input stream ends (returns 0 bytes)
/// - Automatically flushes the decoder at the end to output remaining frames
/// - Exits gracefully if the receiver channel is closed
/// - Uses a 4KB buffer for reading encoded data chunks
///
/// # Panics
/// May panic if the Vulkan decoder creation or decoding operations fail.
/// In production code, these should be handled with proper error propagation.
pub fn run_decoder(tx: SyncSender<FrameWithPts>, framerate: u32, vulkan_device: Arc<VulkanDevice>, mut bytestream_reader: impl Read) {
    // Create a Vulkan Video decoder that outputs wgpu textures
    let mut decoder = vulkan_device.create_wgpu_textures_decoder().unwrap();

    // Calculate the time interval between frames based on framerate
    let frame_interval = 1.0 / (framerate as f64);
    let mut frame_number = 0u64;

    // Buffer for reading encoded data chunks
    let mut buffer = BytesMut::zeroed(4096);

    // Closure to send a decoded frame with calculated timestamp
    let send_frame = move |frame: Frame<wgpu::Texture>, frame_number: &mut u64| {
        let result = FrameWithPts {
            frame: frame.data,
            // Calculate presentation timestamp based on frame number and framerate
            pts: Duration::from_secs_f64(*frame_number as f64 * frame_interval),
        };

        *frame_number += 1;

        tx.send(result)
    };

    // Main decoding loop: read encoded data and decode frames
    while let Ok(n) = bytestream_reader.read(&mut buffer) {
        if n == 0 {
            // End of stream reached
            return;
        }

        // Create an encoded chunk without explicit PTS (decoder will handle timing)
        let frame = EncodedChunk { data: &buffer[..n], pts: None };

        // Decode the chunk, which may produce zero or more output frames
        let decoded = decoder.decode(frame).unwrap();

        // Send all decoded frames
        for f in decoded {
            if send_frame(f, &mut frame_number).is_err() {
                // Receiver channel closed, exit gracefully
                return;
            }
        }
    }

    // Flush the decoder to output any remaining frames
    for f in decoder.flush() {
        if send_frame(f, &mut frame_number).is_err() {
            // Receiver channel closed, exit gracefully
            return;
        }
    }
}
