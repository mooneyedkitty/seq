// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Drone generator for sustained pad-like sounds.
//!
//! Generates sustained notes with slow movement between scale tones,
//! featuring voice leading and configurable density.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::{Generator, GeneratorContext, MidiEvent};

/// Configuration for drone behavior
#[derive(Debug, Clone)]
struct DroneConfig {
    /// Number of voices (1-8)
    voices: u8,
    /// How often notes change (in beats, 0 = never)
    change_rate: f64,
    /// Probability of a voice changing when triggered (0.0 - 1.0)
    change_probability: f64,
    /// Base velocity (0-127)
    velocity: u8,
    /// Velocity variation (+/-)
    velocity_variation: u8,
    /// Prefer intervals: 0=any, 1=root, 2=fifth, 3=thirds
    interval_preference: u8,
    /// Maximum interval jump in scale degrees
    max_jump: u8,
    /// Base octave (MIDI octave, middle C = 4)
    base_octave: i8,
    /// Octave spread for voices
    octave_spread: u8,
}

impl Default for DroneConfig {
    fn default() -> Self {
        Self {
            voices: 3,
            change_rate: 4.0, // Change every 4 beats
            change_probability: 0.5,
            velocity: 80,
            velocity_variation: 10,
            interval_preference: 0,
            max_jump: 2,
            base_octave: 3,
            octave_spread: 2,
        }
    }
}

/// Active voice state
#[derive(Debug, Clone)]
struct Voice {
    /// Current MIDI note
    note: u8,
    /// Current velocity
    velocity: u8,
    /// Whether this voice is currently sounding
    active: bool,
    /// Ticks until this voice should change
    change_in: u64,
}

/// Drone generator
pub struct DroneGenerator {
    config: DroneConfig,
    voices: Vec<Voice>,
    last_change_tick: u64,
    rng: StdRng,
}

impl DroneGenerator {
    /// Create a new drone generator
    pub fn new() -> Self {
        Self {
            config: DroneConfig::default(),
            voices: Vec::new(),
            last_change_tick: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Factory function for registry
    pub fn create() -> Box<dyn Generator> {
        Box::new(Self::new())
    }

    /// Initialize voices if needed
    fn ensure_voices(&mut self, context: &GeneratorContext) {
        if self.voices.len() != self.config.voices as usize {
            self.voices.clear();
            let scale = context.scale();

            for i in 0..self.config.voices {
                // Distribute voices across octaves
                let octave_offset = if self.config.octave_spread > 0 {
                    (i as i8 % (self.config.octave_spread as i8 + 1)) - (self.config.octave_spread as i8 / 2)
                } else {
                    0
                };
                let octave = self.config.base_octave + octave_offset;

                // Pick initial scale degree based on voice index
                let degree = match i {
                    0 => 1, // Root
                    1 => 5, // Fifth
                    2 => 3, // Third
                    _ => (i as usize % scale.len()) + 1,
                };

                let note = scale.midi_note_at(degree, octave).unwrap_or(60);
                let velocity = self.random_velocity();
                let change_delay = self.random_change_delay(context);

                self.voices.push(Voice {
                    note,
                    velocity,
                    active: true,
                    change_in: change_delay,
                });
            }
        }
    }

    /// Generate a random velocity within configured range
    fn random_velocity(&mut self) -> u8 {
        let base = self.config.velocity as i16;
        let var = self.config.velocity_variation as i16;
        let offset = self.rng.gen_range(-var..=var);
        (base + offset).clamp(1, 127) as u8
    }

    /// Calculate random delay until next change
    fn random_change_delay(&mut self, context: &GeneratorContext) -> u64 {
        if self.config.change_rate <= 0.0 {
            return u64::MAX; // Never change
        }

        let base_ticks = (context.ppqn as f64 * self.config.change_rate) as u64;
        let variation = base_ticks / 4;
        let offset = self.rng.gen_range(0..=variation);
        base_ticks + offset
    }

    /// Pick a new note for a voice using voice leading
    fn pick_new_note(&mut self, voice_idx: usize, context: &GeneratorContext) -> u8 {
        let scale = context.scale();
        let current_note = self.voices[voice_idx].note;

        // Get current scale degree
        let current_pc = current_note % 12;
        let current_octave = (current_note / 12) as i8 - 1;

        // Find nearby scale tones
        let mut candidates: Vec<u8> = Vec::new();

        for degree in 1..=scale.len() {
            for octave_offset in -(self.config.octave_spread as i8)..=(self.config.octave_spread as i8) {
                let target_octave = self.config.base_octave + octave_offset;
                if let Some(note) = scale.midi_note_at(degree, target_octave) {
                    // Check if within max_jump scale degrees
                    let interval = (note as i16 - current_note as i16).abs();
                    if interval <= (self.config.max_jump as i16 * 2 + 2) {
                        // Apply interval preference
                        let weight = match self.config.interval_preference {
                            1 if degree == 1 => 3, // Prefer root
                            2 if degree == 5 => 3, // Prefer fifth
                            3 if degree == 3 || degree == 6 => 2, // Prefer thirds/sixths
                            _ => 1,
                        };

                        for _ in 0..weight {
                            candidates.push(note);
                        }
                    }
                }
            }
        }

        // Avoid doubling with other voices
        let other_notes: Vec<u8> = self.voices
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != voice_idx)
            .map(|(_, v)| v.note)
            .collect();

        candidates.retain(|n| !other_notes.contains(n));

        if candidates.is_empty() {
            return current_note; // Stay on current note
        }

        candidates[self.rng.gen_range(0..candidates.len())]
    }
}

impl Default for DroneGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for DroneGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        self.ensure_voices(context);

        let mut events = Vec::new();
        let current_tick = context.total_ticks();
        let change_prob = self.config.change_probability;

        // First pass: generate events and track which voices need changes
        let mut needs_change = Vec::new();
        for (i, voice) in self.voices.iter().enumerate() {
            // Generate sustained note for the duration
            if voice.active {
                events.push(MidiEvent::new(
                    voice.note,
                    voice.velocity,
                    0,
                    context.ticks_to_generate,
                ));
            }

            // Check if voice should change
            if voice.change_in == 0 {
                needs_change.push(i);
            }
        }

        // Update change timers
        let voices_len = self.voices.len();
        for i in 0..voices_len {
            if self.voices[i].change_in == 0 {
                self.voices[i].change_in = self.random_change_delay(context);
            } else {
                self.voices[i].change_in = self.voices[i].change_in.saturating_sub(context.ticks_to_generate);
            }
        }

        // Update notes for voices that should change
        for i in needs_change {
            if self.rng.gen::<f64>() < change_prob {
                let new_note = self.pick_new_note(i, context);
                self.voices[i].note = new_note;
                self.voices[i].velocity = self.random_velocity();
            }
        }

        self.last_change_tick = current_tick;
        events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        match name {
            "voices" => self.config.voices = (value as u8).clamp(1, 8),
            "change_rate" => self.config.change_rate = value.max(0.0),
            "change_probability" => self.config.change_probability = value.clamp(0.0, 1.0),
            "velocity" => self.config.velocity = (value as u8).clamp(1, 127),
            "velocity_variation" => self.config.velocity_variation = (value as u8).min(64),
            "interval_preference" => self.config.interval_preference = (value as u8).min(3),
            "max_jump" => self.config.max_jump = (value as u8).clamp(1, 7),
            "base_octave" => self.config.base_octave = (value as i8).clamp(0, 8),
            "octave_spread" => self.config.octave_spread = (value as u8).min(4),
            _ => {}
        }
        // Reset voices when config changes significantly
        if name == "voices" || name == "base_octave" || name == "octave_spread" {
            self.voices.clear();
        }
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "voices" => Some(self.config.voices as f64),
            "change_rate" => Some(self.config.change_rate),
            "change_probability" => Some(self.config.change_probability),
            "velocity" => Some(self.config.velocity as f64),
            "velocity_variation" => Some(self.config.velocity_variation as f64),
            "interval_preference" => Some(self.config.interval_preference as f64),
            "max_jump" => Some(self.config.max_jump as f64),
            "base_octave" => Some(self.config.base_octave as f64),
            "octave_spread" => Some(self.config.octave_spread as f64),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.voices.clear();
        self.last_change_tick = 0;
    }

    fn name(&self) -> &'static str {
        "drone"
    }

    fn params(&self) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        params.insert("voices".to_string(), self.config.voices as f64);
        params.insert("change_rate".to_string(), self.config.change_rate);
        params.insert("change_probability".to_string(), self.config.change_probability);
        params.insert("velocity".to_string(), self.config.velocity as f64);
        params.insert("velocity_variation".to_string(), self.config.velocity_variation as f64);
        params.insert("interval_preference".to_string(), self.config.interval_preference as f64);
        params.insert("max_jump".to_string(), self.config.max_jump as f64);
        params.insert("base_octave".to_string(), self.config.base_octave as f64);
        params.insert("octave_spread".to_string(), self.config.octave_spread as f64);
        params
    }
}

impl Clone for DroneGenerator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            voices: self.voices.clone(),
            last_change_tick: self.last_change_tick,
            rng: StdRng::from_entropy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::scale::{Key, Note, ScaleType};

    fn test_context() -> GeneratorContext {
        GeneratorContext {
            key: Key::new(Note::C, ScaleType::Major),
            ppqn: 24,
            ticks_to_generate: 24,
            ..Default::default()
        }
    }

    #[test]
    fn test_drone_creation() {
        let drone = DroneGenerator::new();
        assert_eq!(drone.name(), "drone");
        assert_eq!(drone.get_param("voices"), Some(3.0));
    }

    #[test]
    fn test_drone_generates_notes() {
        let mut drone = DroneGenerator::new();
        let ctx = test_context();

        let events = drone.generate(&ctx);
        assert!(!events.is_empty());

        // Should have 3 voices by default
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_drone_notes_in_scale() {
        let mut drone = DroneGenerator::new();
        let ctx = test_context();

        let events = drone.generate(&ctx);
        let scale = ctx.scale();

        for event in events {
            assert!(
                scale.contains_midi(event.note),
                "Note {} not in scale",
                event.note
            );
        }
    }

    #[test]
    fn test_drone_param_changes() {
        let mut drone = DroneGenerator::new();

        drone.set_param("voices", 5.0);
        assert_eq!(drone.get_param("voices"), Some(5.0));

        drone.set_param("change_rate", 8.0);
        assert_eq!(drone.get_param("change_rate"), Some(8.0));

        drone.set_param("velocity", 100.0);
        assert_eq!(drone.get_param("velocity"), Some(100.0));
    }

    #[test]
    fn test_drone_reset() {
        let mut drone = DroneGenerator::new();
        let ctx = test_context();

        drone.generate(&ctx);
        assert!(!drone.voices.is_empty());

        drone.reset();
        assert!(drone.voices.is_empty());
    }

    #[test]
    fn test_drone_voice_count() {
        let mut drone = DroneGenerator::new();
        drone.set_param("voices", 5.0);

        let ctx = test_context();
        let events = drone.generate(&ctx);

        assert_eq!(events.len(), 5);
    }
}
