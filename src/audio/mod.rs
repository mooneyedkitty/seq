// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Audio engine for the SEQ sequencer.
//!
//! This module provides:
//! - FluidSynth integration for software synthesis
//! - Audio output via cpal (Core Audio on macOS)
//! - Buffer management and latency control

pub mod fluidsynth;
pub mod output;

pub use fluidsynth::FluidSynth;
pub use output::{AudioConfig, AudioOutput};

use std::sync::{Arc, Mutex};

/// Audio engine combining synth and output
pub struct AudioEngine {
    /// FluidSynth instance
    synth: Arc<Mutex<FluidSynth>>,
    /// Audio output
    output: Option<AudioOutput>,
    /// Whether audio is running
    running: bool,
    /// Sample rate
    sample_rate: u32,
    /// Buffer size in frames
    buffer_size: u32,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new() -> Self {
        Self {
            synth: Arc::new(Mutex::new(FluidSynth::new())),
            output: None,
            running: false,
            sample_rate: 44100,
            buffer_size: 512,
        }
    }

    /// Create with custom sample rate
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        let mut engine = Self::new();
        engine.sample_rate = sample_rate;
        engine
    }

    /// Get synth reference
    pub fn synth(&self) -> Arc<Mutex<FluidSynth>> {
        Arc::clone(&self.synth)
    }

    /// Load a soundfont
    pub fn load_soundfont(&mut self, path: &str) -> Result<(), AudioError> {
        let mut synth = self.synth.lock().map_err(|_| AudioError::LockFailed)?;
        synth.load_soundfont(path)
    }

    /// Start audio output
    pub fn start(&mut self) -> Result<(), AudioError> {
        if self.running {
            return Ok(());
        }

        let config = AudioConfig {
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
            channels: 2,
        };

        let synth = Arc::clone(&self.synth);
        let output = AudioOutput::new(config, move |buffer, channels| {
            if let Ok(mut synth) = synth.lock() {
                synth.render(buffer, channels);
            }
        })?;

        self.output = Some(output);
        self.running = true;
        Ok(())
    }

    /// Stop audio output
    pub fn stop(&mut self) {
        self.output = None;
        self.running = false;
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Send note on
    pub fn note_on(&self, channel: u8, note: u8, velocity: u8) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.note_on(channel, note, velocity);
        }
    }

    /// Send note off
    pub fn note_off(&self, channel: u8, note: u8) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.note_off(channel, note);
        }
    }

    /// Send control change
    pub fn control_change(&self, channel: u8, control: u8, value: u8) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.control_change(channel, control, value);
        }
    }

    /// Send program change
    pub fn program_change(&self, channel: u8, program: u8) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.program_change(channel, program);
        }
    }

    /// Send pitch bend
    pub fn pitch_bend(&self, channel: u8, value: i16) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.pitch_bend(channel, value);
        }
    }

    /// All notes off
    pub fn all_notes_off(&self) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.all_notes_off();
        }
    }

    /// Set master volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.set_gain(volume);
        }
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set buffer size (requires restart)
    pub fn set_buffer_size(&mut self, size: u32) {
        self.buffer_size = size.clamp(64, 4096);
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio error types
#[derive(Debug, Clone)]
pub enum AudioError {
    /// Failed to initialize audio
    InitFailed(String),
    /// Failed to load soundfont
    SoundfontLoadFailed(String),
    /// Failed to start audio stream
    StreamFailed(String),
    /// Failed to acquire lock
    LockFailed,
    /// No audio device available
    NoDevice,
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::InitFailed(msg) => write!(f, "Audio initialization failed: {}", msg),
            AudioError::SoundfontLoadFailed(msg) => write!(f, "Soundfont load failed: {}", msg),
            AudioError::StreamFailed(msg) => write!(f, "Audio stream failed: {}", msg),
            AudioError::LockFailed => write!(f, "Failed to acquire audio lock"),
            AudioError::NoDevice => write!(f, "No audio device available"),
            AudioError::InvalidConfig(msg) => write!(f, "Invalid audio configuration: {}", msg),
        }
    }
}

impl std::error::Error for AudioError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_engine_creation() {
        let engine = AudioEngine::new();
        assert!(!engine.is_running());
        assert_eq!(engine.sample_rate(), 44100);
    }

    #[test]
    fn test_audio_engine_with_sample_rate() {
        let engine = AudioEngine::with_sample_rate(48000);
        assert_eq!(engine.sample_rate(), 48000);
    }

    #[test]
    fn test_buffer_size_clamping() {
        let mut engine = AudioEngine::new();

        engine.set_buffer_size(32);
        assert_eq!(engine.buffer_size, 64);

        engine.set_buffer_size(10000);
        assert_eq!(engine.buffer_size, 4096);
    }
}
