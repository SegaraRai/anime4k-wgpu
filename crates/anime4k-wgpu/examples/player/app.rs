//! Application event handler for the Anime4K video player
//!
//! This module contains the main application structure that handles window events,
//! keyboard input, and coordinates the overall playback experience.

use super::player::PlayerContext;
use anime4k_wgpu::presets::{Anime4KPerformancePreset, Anime4KPreset};
use std::path::{Path, PathBuf};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::WindowId,
};

/// Main video player application structure
pub struct VideoPlayerApp {
    /// The video file to play
    filename: PathBuf,
    /// The framerate of the video in frames per second
    framerate: u32,
    /// Whether the video starts in paused state
    start_paused: bool,
    /// Keyboard modifiers state
    modifiers: ModifiersState,
    /// The application context containing window, playback state, and renderer
    context: Option<PlayerContext>,
}

impl VideoPlayerApp {
    /// Creates a new video player application instance
    ///
    /// # Arguments
    /// * `filename` - Path to the video file to play
    /// * `framerate` - Video framerate in frames per second
    /// * `start_paused` - Whether the video should start in paused state
    ///
    /// # Returns
    /// A new `VideoPlayerApp` instance ready to be run in an event loop
    pub fn new(filename: &Path, framerate: u32, start_paused: bool) -> Self {
        Self {
            filename: filename.to_path_buf(),
            framerate,
            start_paused,
            modifiers: ModifiersState::default(),
            context: None,
        }
    }
}

impl ApplicationHandler for VideoPlayerApp {
    /// Handles application resumption by initializing the player context
    ///
    /// This is called when the application becomes active and creates the window,
    /// initializes video decoding, and displays keyboard shortcuts to the user.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let file = std::fs::File::open(&self.filename).unwrap_or_else(|_| panic!("Failed to open video file: {}", self.filename.display()));
        self.context = Some(PlayerContext::new(event_loop, file, self.framerate, self.start_paused));

        println!();
        println!("Keyboard shortcuts:");
        println!("  - Esc: Quit");
        println!("  - Space: Pause/Resume video playback");
        println!("  - Ctrl+0: Disable Anime4K");
        println!("  - Ctrl+1-6: Set Anime4K preset (A, B, C, AA, BB, CA)");
        println!("  - Shift+1-5: Set Anime4K performance preset (Light, Medium, High, Ultra, Extreme)");
        println!();

        println!("NOTE:");
        println!("  - Anime4K is disabled by default. Use Ctrl+1-6 to enable it.");
        if self.start_paused {
            println!("  - Video starts in paused state. Press Space to resume playback.");
        }
        println!();
    }

    /// Handles all window events including keyboard input and resize events
    ///
    /// This method processes user input for playback control and Anime4K preset changes:
    /// - Escape: Quit application
    /// - Space: Toggle pause/resume
    /// - Ctrl+0: Disable Anime4K processing
    /// - Ctrl+1-6: Set Anime4K presets (A, B, C, AA, BB, CA)
    /// - Shift+1-5: Set performance presets (Light, Medium, High, Ultra, Extreme)
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            // Track modifier key state for keyboard shortcuts
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            // Handle quit requests (Escape key or window close button)
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            }
            | WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // Handle pause/resume toggle (Space key)
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Space),
                    ..
                },
                ..
            } => {
                if let Some(context) = self.context.as_mut() {
                    if context.is_paused() {
                        context.resume();
                    } else {
                        context.pause();
                    }
                }
            }

            // Handle Anime4K preset selection (Ctrl+0-6)
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } if self.modifiers == ModifiersState::CONTROL => {
                // Map digit keys to Anime4K presets
                let preset = match keycode {
                    KeyCode::Digit0 => Some(None),
                    KeyCode::Digit1 => Some(Some(Anime4KPreset::ModeA)),
                    KeyCode::Digit2 => Some(Some(Anime4KPreset::ModeB)),
                    KeyCode::Digit3 => Some(Some(Anime4KPreset::ModeC)),
                    KeyCode::Digit4 => Some(Some(Anime4KPreset::ModeAA)),
                    KeyCode::Digit5 => Some(Some(Anime4KPreset::ModeBB)),
                    KeyCode::Digit6 => Some(Some(Anime4KPreset::ModeCA)),
                    _ => None,
                };

                if let Some(preset) = preset {
                    if let Some(context) = self.context.as_mut() {
                        context.set_anime4k_preset(preset);
                    }
                }
            }

            // Handle Anime4K performance preset selection (Shift+1-5)
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } if self.modifiers == ModifiersState::SHIFT => {
                // Map digit keys to performance presets
                let performance_preset = match keycode {
                    KeyCode::Digit1 => Some(Anime4KPerformancePreset::Light),
                    KeyCode::Digit2 => Some(Anime4KPerformancePreset::Medium),
                    KeyCode::Digit3 => Some(Anime4KPerformancePreset::High),
                    KeyCode::Digit4 => Some(Anime4KPerformancePreset::Ultra),
                    KeyCode::Digit5 => Some(Anime4KPerformancePreset::Extreme),
                    _ => None,
                };

                if let Some(performance_preset) = performance_preset {
                    if let Some(context) = self.context.as_mut() {
                        context.set_anime4k_performance_preset(performance_preset);
                    }
                }
            }

            // Handle frame rendering and timing
            WindowEvent::RedrawRequested => {
                if let Some(context) = self.context.as_mut() {
                    context.handle_redraw();
                }
            }

            // Handle window resize
            WindowEvent::Resized(new_size) => {
                if let Some(context) = self.context.as_mut() {
                    context.resize(new_size);
                }
            }

            _ => {}
        }
    }
}
