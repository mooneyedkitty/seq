// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Arpeggiator generator for rhythmic note patterns.
//!
//! Generates arpeggiated patterns from scale notes with various
//! patterns, octave ranges, and rhythmic options including Euclidean rhythms.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::{Generator, GeneratorContext, MidiEvent};

/// Arpeggio pattern types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArpPattern {
    /// Play notes ascending
    Up,
    /// Play notes descending
    Down,
    /// Play up then down
    UpDown,
    /// Play down then up
    DownUp,
    /// Random note selection
    Random,
    /// Play in order notes were defined
    Order,
}

impl ArpPattern {
    fn from_value(v: u8) -> Self {
        match v {
            0 => ArpPattern::Up,
            1 => ArpPattern::Down,
            2 => ArpPattern::UpDown,
            3 => ArpPattern::DownUp,
            4 => ArpPattern::Random,
            _ => ArpPattern::Order,
        }
    }

    fn to_value(self) -> u8 {
        match self {
            ArpPattern::Up => 0,
            ArpPattern::Down => 1,
            ArpPattern::UpDown => 2,
            ArpPattern::DownUp => 3,
            ArpPattern::Random => 4,
            ArpPattern::Order => 5,
        }
    }
}

/// Configuration for arpeggiator
#[derive(Debug, Clone)]
struct ArpConfig {
    /// Pattern type
    pattern: ArpPattern,
    /// Note rate as division (4 = quarter, 8 = eighth, 16 = sixteenth)
    rate: u32,
    /// Gate percentage (0.0 - 1.0, portion of note length)
    gate: f64,
    /// Number of octaves to span
    octaves: u8,
    /// Base octave (MIDI octave)
    base_octave: i8,
    /// Base velocity
    velocity: u8,
    /// Velocity accent on beat 1
    accent_velocity: u8,
    /// Probability of playing each note (0.0 - 1.0)
    probability: f64,
    /// Use Euclidean rhythm
    euclidean: bool,
    /// Euclidean pattern: number of hits
    euclidean_hits: u8,
    /// Euclidean pattern: number of steps
    euclidean_steps: u8,
    /// Scale degrees to include (empty = all)
    degrees: Vec<usize>,
}

impl Default for ArpConfig {
    fn default() -> Self {
        Self {
            pattern: ArpPattern::Up,
            rate: 8, // Eighth notes
            gate: 0.8,
            octaves: 2,
            base_octave: 4,
            velocity: 100,
            accent_velocity: 120,
            probability: 1.0,
            euclidean: false,
            euclidean_hits: 5,
            euclidean_steps: 8,
            degrees: vec![], // All degrees
        }
    }
}

/// Arpeggiator generator
pub struct ArpeggioGenerator {
    config: ArpConfig,
    /// Current position in the arpeggio sequence
    position: usize,
    /// Direction for up-down patterns (true = going up)
    direction_up: bool,
    /// Euclidean pattern cache
    euclidean_pattern: Vec<bool>,
    /// Current step in euclidean pattern
    euclidean_step: usize,
    /// Notes in current arpeggio
    note_sequence: Vec<u8>,
    /// Accumulated ticks for timing
    tick_accumulator: u64,
    rng: StdRng,
}

impl ArpeggioGenerator {
    /// Create a new arpeggiator
    pub fn new() -> Self {
        Self {
            config: ArpConfig::default(),
            position: 0,
            direction_up: true,
            euclidean_pattern: Vec::new(),
            euclidean_step: 0,
            note_sequence: Vec::new(),
            tick_accumulator: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Factory function for registry
    pub fn create() -> Box<dyn Generator> {
        Box::new(Self::new())
    }

    /// Generate Euclidean rhythm pattern
    fn generate_euclidean(hits: usize, steps: usize) -> Vec<bool> {
        if steps == 0 {
            return vec![];
        }
        if hits >= steps {
            return vec![true; steps];
        }
        if hits == 0 {
            return vec![false; steps];
        }

        // Bjorklund's algorithm
        let mut pattern = vec![vec![true]; hits];
        let mut remainder = vec![vec![false]; steps - hits];

        while remainder.len() > 1 {
            let min_len = pattern.len().min(remainder.len());
            for i in 0..min_len {
                pattern[i].extend(remainder[i].clone());
            }
            let new_remainder: Vec<Vec<bool>> = if pattern.len() > min_len {
                pattern.drain(min_len..).collect()
            } else {
                remainder.drain(min_len..).collect()
            };
            remainder = new_remainder;
        }

        // Flatten and append remainder
        let mut result: Vec<bool> = pattern.into_iter().flatten().collect();
        for r in remainder {
            result.extend(r);
        }
        result
    }

    /// Build the note sequence based on scale and configuration
    fn build_sequence(&mut self, context: &GeneratorContext) {
        let scale = context.scale();
        let degrees: Vec<usize> = if self.config.degrees.is_empty() {
            (1..=scale.len()).collect()
        } else {
            self.config.degrees.clone()
        };

        self.note_sequence.clear();

        // Build notes across octaves
        for octave_offset in 0..self.config.octaves {
            let octave = self.config.base_octave + octave_offset as i8;
            for &degree in &degrees {
                if let Some(note) = scale.midi_note_at(degree, octave) {
                    self.note_sequence.push(note);
                }
            }
        }

        // Sort for patterns that need it
        match self.config.pattern {
            ArpPattern::Up | ArpPattern::UpDown => {
                self.note_sequence.sort();
            }
            ArpPattern::Down | ArpPattern::DownUp => {
                self.note_sequence.sort();
                self.note_sequence.reverse();
            }
            _ => {}
        }

        // Update euclidean pattern if enabled
        if self.config.euclidean {
            self.euclidean_pattern = Self::generate_euclidean(
                self.config.euclidean_hits as usize,
                self.config.euclidean_steps as usize,
            );
        }
    }

    /// Get the next note in the sequence
    fn next_note(&mut self) -> Option<u8> {
        if self.note_sequence.is_empty() {
            return None;
        }

        let note = match self.config.pattern {
            ArpPattern::Random => {
                let idx = self.rng.gen_range(0..self.note_sequence.len());
                self.note_sequence[idx]
            }
            ArpPattern::UpDown => {
                let note = self.note_sequence[self.position];
                if self.direction_up {
                    self.position += 1;
                    if self.position >= self.note_sequence.len() {
                        self.position = self.note_sequence.len().saturating_sub(2);
                        self.direction_up = false;
                    }
                } else {
                    if self.position == 0 {
                        self.direction_up = true;
                        self.position = 1.min(self.note_sequence.len() - 1);
                    } else {
                        self.position -= 1;
                    }
                }
                note
            }
            ArpPattern::DownUp => {
                let note = self.note_sequence[self.position];
                if !self.direction_up {
                    if self.position == 0 {
                        self.direction_up = true;
                        self.position = 1.min(self.note_sequence.len() - 1);
                    } else {
                        self.position -= 1;
                    }
                } else {
                    self.position += 1;
                    if self.position >= self.note_sequence.len() {
                        self.position = self.note_sequence.len().saturating_sub(2);
                        self.direction_up = false;
                    }
                }
                note
            }
            _ => {
                let note = self.note_sequence[self.position];
                self.position = (self.position + 1) % self.note_sequence.len();
                note
            }
        };

        Some(note)
    }

    /// Check if current euclidean step should play
    fn should_play_euclidean(&mut self) -> bool {
        if !self.config.euclidean || self.euclidean_pattern.is_empty() {
            return true;
        }
        let should_play = self.euclidean_pattern[self.euclidean_step];
        self.euclidean_step = (self.euclidean_step + 1) % self.euclidean_pattern.len();
        should_play
    }
}

impl Default for ArpeggioGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for ArpeggioGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        // Rebuild sequence if needed
        if self.note_sequence.is_empty() {
            self.build_sequence(context);
        }

        let mut events = Vec::new();
        let ticks_per_note = context.note_duration(self.config.rate);
        let note_duration = (ticks_per_note as f64 * self.config.gate) as u64;

        // Process each tick position
        let mut tick = 0u64;
        while tick < context.ticks_to_generate {
            let total_tick = self.tick_accumulator + tick;

            // Check if we're on a note boundary
            if total_tick % ticks_per_note == 0 {
                // Check probability
                if self.rng.gen::<f64>() < self.config.probability {
                    // Check euclidean pattern
                    if self.should_play_euclidean() {
                        if let Some(note) = self.next_note() {
                            // Determine velocity (accent on beat 1)
                            let velocity = if total_tick % context.ticks_per_bar() == 0 {
                                self.config.accent_velocity
                            } else {
                                self.config.velocity
                            };

                            events.push(MidiEvent::new(
                                note,
                                velocity,
                                tick,
                                note_duration,
                            ));
                        }
                    }
                }
            }

            tick += 1;
        }

        self.tick_accumulator += context.ticks_to_generate;
        events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        match name {
            "pattern" => self.config.pattern = ArpPattern::from_value(value as u8),
            "rate" => self.config.rate = (value as u32).clamp(1, 64),
            "gate" => self.config.gate = value.clamp(0.1, 1.0),
            "octaves" => self.config.octaves = (value as u8).clamp(1, 4),
            "base_octave" => self.config.base_octave = (value as i8).clamp(0, 8),
            "velocity" => self.config.velocity = (value as u8).clamp(1, 127),
            "accent_velocity" => self.config.accent_velocity = (value as u8).clamp(1, 127),
            "probability" => self.config.probability = value.clamp(0.0, 1.0),
            "euclidean" => self.config.euclidean = value > 0.5,
            "euclidean_hits" => self.config.euclidean_hits = (value as u8).clamp(1, 32),
            "euclidean_steps" => self.config.euclidean_steps = (value as u8).clamp(1, 32),
            _ => {}
        }
        // Rebuild sequence when relevant params change
        if matches!(name, "octaves" | "base_octave" | "pattern") {
            self.note_sequence.clear();
        }
        if matches!(name, "euclidean_hits" | "euclidean_steps") {
            self.euclidean_pattern = Self::generate_euclidean(
                self.config.euclidean_hits as usize,
                self.config.euclidean_steps as usize,
            );
        }
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "pattern" => Some(self.config.pattern.to_value() as f64),
            "rate" => Some(self.config.rate as f64),
            "gate" => Some(self.config.gate),
            "octaves" => Some(self.config.octaves as f64),
            "base_octave" => Some(self.config.base_octave as f64),
            "velocity" => Some(self.config.velocity as f64),
            "accent_velocity" => Some(self.config.accent_velocity as f64),
            "probability" => Some(self.config.probability),
            "euclidean" => Some(if self.config.euclidean { 1.0 } else { 0.0 }),
            "euclidean_hits" => Some(self.config.euclidean_hits as f64),
            "euclidean_steps" => Some(self.config.euclidean_steps as f64),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.position = 0;
        self.direction_up = true;
        self.euclidean_step = 0;
        self.tick_accumulator = 0;
        self.note_sequence.clear();
    }

    fn name(&self) -> &'static str {
        "arpeggio"
    }

    fn params(&self) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        params.insert("pattern".to_string(), self.config.pattern.to_value() as f64);
        params.insert("rate".to_string(), self.config.rate as f64);
        params.insert("gate".to_string(), self.config.gate);
        params.insert("octaves".to_string(), self.config.octaves as f64);
        params.insert("base_octave".to_string(), self.config.base_octave as f64);
        params.insert("velocity".to_string(), self.config.velocity as f64);
        params.insert("accent_velocity".to_string(), self.config.accent_velocity as f64);
        params.insert("probability".to_string(), self.config.probability);
        params.insert("euclidean".to_string(), if self.config.euclidean { 1.0 } else { 0.0 });
        params.insert("euclidean_hits".to_string(), self.config.euclidean_hits as f64);
        params.insert("euclidean_steps".to_string(), self.config.euclidean_steps as f64);
        params
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
            ticks_to_generate: 96, // One bar
            beats_per_bar: 4,
            ..Default::default()
        }
    }

    #[test]
    fn test_arpeggio_creation() {
        let arp = ArpeggioGenerator::new();
        assert_eq!(arp.name(), "arpeggio");
        assert_eq!(arp.get_param("rate"), Some(8.0));
    }

    #[test]
    fn test_arpeggio_generates_notes() {
        let mut arp = ArpeggioGenerator::new();
        let ctx = test_context();

        let events = arp.generate(&ctx);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_euclidean_rhythm() {
        // Classic 3-over-8 euclidean
        let pattern = ArpeggioGenerator::generate_euclidean(3, 8);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 3);

        // 5-over-8
        let pattern = ArpeggioGenerator::generate_euclidean(5, 8);
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 5);

        // Edge cases
        let pattern = ArpeggioGenerator::generate_euclidean(0, 8);
        assert!(pattern.iter().all(|&b| !b));

        let pattern = ArpeggioGenerator::generate_euclidean(8, 8);
        assert!(pattern.iter().all(|&b| b));
    }

    #[test]
    fn test_arpeggio_pattern_up() {
        let mut arp = ArpeggioGenerator::new();
        arp.set_param("pattern", 0.0); // Up
        arp.set_param("octaves", 1.0);
        arp.set_param("rate", 4.0); // Quarter notes

        let ctx = test_context();
        let events = arp.generate(&ctx);

        // Should generate 4 quarter notes in one bar
        assert_eq!(events.len(), 4);

        // Notes should be ascending
        for i in 1..events.len() {
            assert!(events[i].note >= events[i - 1].note);
        }
    }

    #[test]
    fn test_arpeggio_param_changes() {
        let mut arp = ArpeggioGenerator::new();

        arp.set_param("rate", 16.0);
        assert_eq!(arp.get_param("rate"), Some(16.0));

        arp.set_param("gate", 0.5);
        assert_eq!(arp.get_param("gate"), Some(0.5));

        arp.set_param("euclidean", 1.0);
        assert_eq!(arp.get_param("euclidean"), Some(1.0));
    }

    #[test]
    fn test_arpeggio_reset() {
        let mut arp = ArpeggioGenerator::new();
        let ctx = test_context();

        arp.generate(&ctx);
        assert!(arp.tick_accumulator > 0);

        arp.reset();
        assert_eq!(arp.tick_accumulator, 0);
        assert_eq!(arp.position, 0);
    }

    #[test]
    fn test_arpeggio_notes_in_scale() {
        let mut arp = ArpeggioGenerator::new();
        let ctx = test_context();

        let events = arp.generate(&ctx);
        let scale = ctx.scale();

        for event in events {
            assert!(
                scale.contains_midi(event.note),
                "Note {} not in scale",
                event.note
            );
        }
    }
}
