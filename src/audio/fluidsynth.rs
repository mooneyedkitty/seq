// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! FluidSynth integration for software synthesis.
//!
//! Provides a wrapper around FluidLite for SF2 soundfont playback
//! with MIDI event routing.

use fluidlite::{IsSettings, Settings, Synth};
use std::path::Path;

use super::AudioError;

/// FluidSynth wrapper for software synthesis
pub struct FluidSynth {
    /// FluidLite synth instance
    synth: Option<Synth>,
    /// Current gain (0.0 - 1.0)
    gain: f32,
    /// Loaded soundfont ID
    soundfont_id: Option<u32>,
    /// Sample rate
    sample_rate: f64,
}

impl FluidSynth {
    /// Create a new FluidSynth instance
    pub fn new() -> Self {
        Self::with_sample_rate(44100.0)
    }

    /// Create with custom sample rate
    pub fn with_sample_rate(sample_rate: f64) -> Self {
        let settings = Settings::new().expect("Failed to create FluidLite settings");

        // Configure settings
        if let Some(setting) = settings.num("synth.sample-rate") {
            setting.set(sample_rate);
        }
        if let Some(setting) = settings.num("synth.gain") {
            setting.set(0.5);
        }
        if let Some(setting) = settings.int("synth.polyphony") {
            setting.set(256);
        }
        if let Some(setting) = settings.int("synth.midi-channels") {
            setting.set(16);
        }

        let synth = Synth::new(settings).expect("Failed to create FluidLite synth");

        // Disable reverb and chorus for lower latency
        synth.set_reverb_on(false);
        synth.set_chorus_on(false);

        Self {
            synth: Some(synth),
            gain: 0.5,
            soundfont_id: None,
            sample_rate,
        }
    }

    /// Load a soundfont file
    pub fn load_soundfont(&mut self, path: &str) -> Result<(), AudioError> {
        let synth = self.synth.as_ref().ok_or_else(|| {
            AudioError::InitFailed("Synth not initialized".to_string())
        })?;

        // Check if file exists
        if !Path::new(path).exists() {
            return Err(AudioError::SoundfontLoadFailed(format!(
                "Soundfont file not found: {}",
                path
            )));
        }

        // Unload previous soundfont if any
        if let Some(id) = self.soundfont_id {
            let _ = synth.sfunload(id, true);
        }

        // Load new soundfont
        match synth.sfload(path, true) {
            Ok(id) => {
                self.soundfont_id = Some(id);
                Ok(())
            }
            Err(_) => Err(AudioError::SoundfontLoadFailed(format!(
                "Failed to load soundfont: {}",
                path
            ))),
        }
    }

    /// Check if a soundfont is loaded
    pub fn has_soundfont(&self) -> bool {
        self.soundfont_id.is_some()
    }

    /// Render audio to buffer (interleaved stereo)
    pub fn render(&mut self, buffer: &mut [f32], channels: usize) {
        if let Some(ref synth) = self.synth {
            if channels == 2 {
                // Use direct interleaved rendering
                let _ = synth.write(&mut *buffer);
                // Apply gain
                for sample in buffer.iter_mut() {
                    *sample *= self.gain;
                }
            } else if channels == 1 {
                // Mono: render stereo then mix down
                let frames = buffer.len();
                let mut stereo = vec![0.0f32; frames * 2];
                let _ = synth.write(stereo.as_mut_slice());
                for i in 0..frames {
                    buffer[i] = (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5 * self.gain;
                }
            }
        }
    }

    /// Send note on
    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8) {
        if let Some(ref synth) = self.synth {
            let _ = synth.note_on(channel as u32, note as u32, velocity as u32);
        }
    }

    /// Send note off
    pub fn note_off(&mut self, channel: u8, note: u8) {
        if let Some(ref synth) = self.synth {
            let _ = synth.note_off(channel as u32, note as u32);
        }
    }

    /// Send control change
    pub fn control_change(&mut self, channel: u8, control: u8, value: u8) {
        if let Some(ref synth) = self.synth {
            let _ = synth.cc(channel as u32, control as u32, value as u32);
        }
    }

    /// Send program change
    pub fn program_change(&mut self, channel: u8, program: u8) {
        if let Some(ref synth) = self.synth {
            let _ = synth.program_change(channel as u32, program as u32);
        }
    }

    /// Send pitch bend (-8192 to 8191)
    pub fn pitch_bend(&mut self, channel: u8, value: i16) {
        if let Some(ref synth) = self.synth {
            // FluidLite expects 0-16383, center at 8192
            let bend_value = (value as i32 + 8192).clamp(0, 16383) as u32;
            let _ = synth.pitch_bend(channel as u32, bend_value);
        }
    }

    /// All notes off on all channels
    pub fn all_notes_off(&mut self) {
        if let Some(ref synth) = self.synth {
            for channel in 0..16 {
                // CC 123 = All Notes Off
                let _ = synth.cc(channel, 123, 0);
                // CC 120 = All Sound Off
                let _ = synth.cc(channel, 120, 0);
            }
        }
    }

    /// Set master gain (0.0 - 1.0)
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Get current gain
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Set reverb level (0.0 - 1.0)
    pub fn set_reverb(&mut self, level: f32) {
        if let Some(ref synth) = self.synth {
            if level > 0.0 {
                synth.set_reverb_on(true);
                // Set reverb parameters: roomsize, damp, width, level
                synth.set_reverb_params(0.6, 0.4, 0.5, level as f64);
            } else {
                synth.set_reverb_on(false);
            }
        }
    }

    /// Set chorus on/off
    pub fn set_chorus(&mut self, enabled: bool) {
        if let Some(ref synth) = self.synth {
            synth.set_chorus_on(enabled);
        }
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Reset synth state
    pub fn reset(&mut self) {
        self.all_notes_off();
        // Reset all controllers on all channels
        if let Some(ref synth) = self.synth {
            for channel in 0..16 {
                // CC 121 = Reset All Controllers
                let _ = synth.cc(channel, 121, 0);
            }
        }
    }

    /// Set bank and program for a channel
    pub fn bank_select(&mut self, channel: u8, bank: u16) {
        if let Some(ref synth) = self.synth {
            // Bank select MSB (CC 0)
            let _ = synth.cc(channel as u32, 0, (bank >> 7) as u32);
            // Bank select LSB (CC 32)
            let _ = synth.cc(channel as u32, 32, (bank & 0x7F) as u32);
        }
    }
}

impl Default for FluidSynth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluidsynth_creation() {
        let synth = FluidSynth::new();
        assert!(!synth.has_soundfont());
        assert_eq!(synth.sample_rate(), 44100.0);
        assert!((synth.gain() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_fluidsynth_gain() {
        let mut synth = FluidSynth::new();

        synth.set_gain(0.8);
        assert!((synth.gain() - 0.8).abs() < 0.01);

        synth.set_gain(1.5); // Should clamp
        assert!((synth.gain() - 1.0).abs() < 0.01);

        synth.set_gain(-0.5); // Should clamp
        assert!((synth.gain() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_soundfont_not_found() {
        let mut synth = FluidSynth::new();
        let result = synth.load_soundfont("/nonexistent/path.sf2");
        assert!(result.is_err());
    }

    #[test]
    fn test_render_without_soundfont() {
        let mut synth = FluidSynth::new();
        let mut buffer = vec![0.0f32; 512];

        // Should not panic even without soundfont
        synth.render(&mut buffer, 2);
    }

    #[test]
    fn test_midi_messages() {
        let mut synth = FluidSynth::new();

        // These should not panic even without a soundfont
        synth.note_on(0, 60, 100);
        synth.note_off(0, 60);
        synth.control_change(0, 1, 64);
        synth.program_change(0, 0);
        synth.pitch_bend(0, 0);
        synth.all_notes_off();
    }

    #[test]
    fn test_custom_sample_rate() {
        let synth = FluidSynth::with_sample_rate(48000.0);
        assert_eq!(synth.sample_rate(), 48000.0);
    }

    #[test]
    fn test_reset() {
        let mut synth = FluidSynth::new();
        synth.note_on(0, 60, 100);
        synth.reset();
        // Reset should not panic
    }
}
