// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Drum generator with Euclidean rhythms and style templates.
//!
//! Generates drum patterns using Euclidean rhythm algorithms,
//! style templates, humanization, and fill generation.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::{Generator, GeneratorContext, MidiEvent};

/// Standard General MIDI drum notes
pub mod gm_drums {
    pub const KICK: u8 = 36;
    pub const SNARE: u8 = 38;
    pub const CLOSED_HAT: u8 = 42;
    pub const OPEN_HAT: u8 = 46;
    pub const LOW_TOM: u8 = 45;
    pub const MID_TOM: u8 = 47;
    pub const HIGH_TOM: u8 = 50;
    pub const CRASH: u8 = 49;
    pub const RIDE: u8 = 51;
    pub const CLAP: u8 = 39;
    pub const RIM: u8 = 37;
    pub const COWBELL: u8 = 56;
}

/// Drum style presets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrumStyle {
    /// Four-on-floor kick, snare on 2 and 4
    FourOnFloor,
    /// Breakbeat style with syncopation
    Breakbeat,
    /// Minimal, sparse pattern
    Sparse,
    /// Busy, dense pattern
    Busy,
    /// Euclidean-based patterns
    Euclidean,
    /// Random/generative
    Random,
}

impl DrumStyle {
    fn from_value(v: u8) -> Self {
        match v {
            0 => DrumStyle::FourOnFloor,
            1 => DrumStyle::Breakbeat,
            2 => DrumStyle::Sparse,
            3 => DrumStyle::Busy,
            4 => DrumStyle::Euclidean,
            _ => DrumStyle::Random,
        }
    }

    fn to_value(self) -> u8 {
        match self {
            DrumStyle::FourOnFloor => 0,
            DrumStyle::Breakbeat => 1,
            DrumStyle::Sparse => 2,
            DrumStyle::Busy => 3,
            DrumStyle::Euclidean => 4,
            DrumStyle::Random => 5,
        }
    }
}

/// Configuration for a single drum instrument
#[derive(Debug, Clone)]
struct DrumVoice {
    /// MIDI note number
    note: u8,
    /// Hit pattern (16 steps, true = hit)
    pattern: Vec<bool>,
    /// Probability of each hit playing (0.0 - 1.0)
    probability: f64,
    /// Base velocity
    velocity: u8,
    /// Accent velocity
    accent_velocity: u8,
    /// Accent pattern (positions that get accent)
    accent_pattern: Vec<bool>,
    /// Ghost note positions
    ghost_pattern: Vec<bool>,
    /// Ghost note velocity
    ghost_velocity: u8,
    /// Enabled
    enabled: bool,
}

impl DrumVoice {
    fn new(note: u8) -> Self {
        Self {
            note,
            pattern: vec![false; 16],
            probability: 1.0,
            velocity: 100,
            accent_velocity: 120,
            accent_pattern: vec![false; 16],
            ghost_pattern: vec![false; 16],
            ghost_velocity: 50,
            enabled: true,
        }
    }

    fn with_pattern(mut self, pattern: Vec<bool>) -> Self {
        self.pattern = pattern;
        self
    }

    fn with_accents(mut self, accents: Vec<bool>) -> Self {
        self.accent_pattern = accents;
        self
    }
}

/// Configuration for drum generator
#[derive(Debug, Clone)]
struct DrumConfig {
    /// Style preset
    style: DrumStyle,
    /// Steps per bar (typically 16)
    steps_per_bar: u8,
    /// Swing amount (0.0 - 1.0)
    swing: f64,
    /// Humanize timing (ms variation)
    humanize_timing: f64,
    /// Humanize velocity (variation amount)
    humanize_velocity: u8,
    /// Fill probability (0.0 - 1.0)
    fill_probability: f64,
    /// Fill frequency (every N bars)
    fill_every_bars: u8,
    /// Euclidean hits for kick
    kick_euclidean_hits: u8,
    /// Euclidean hits for snare
    snare_euclidean_hits: u8,
    /// Euclidean hits for hats
    hat_euclidean_hits: u8,
}

impl Default for DrumConfig {
    fn default() -> Self {
        Self {
            style: DrumStyle::FourOnFloor,
            steps_per_bar: 16,
            swing: 0.0,
            humanize_timing: 0.0,
            humanize_velocity: 5,
            fill_probability: 0.3,
            fill_every_bars: 4,
            kick_euclidean_hits: 4,
            snare_euclidean_hits: 4,
            hat_euclidean_hits: 8,
        }
    }
}

/// Drum pattern generator
pub struct DrumGenerator {
    config: DrumConfig,
    voices: HashMap<String, DrumVoice>,
    /// Current step in pattern
    current_step: usize,
    /// Current bar for fill tracking
    current_bar: u64,
    /// Tick accumulator
    tick_accumulator: u64,
    /// Is currently playing a fill
    in_fill: bool,
    rng: StdRng,
}

impl DrumGenerator {
    /// Create a new drum generator
    pub fn new() -> Self {
        let mut gen = Self {
            config: DrumConfig::default(),
            voices: HashMap::new(),
            current_step: 0,
            current_bar: 0,
            tick_accumulator: 0,
            in_fill: false,
            rng: StdRng::from_entropy(),
        };
        gen.build_pattern();
        gen
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

        let mut result: Vec<bool> = pattern.into_iter().flatten().collect();
        for r in remainder {
            result.extend(r);
        }
        result
    }

    /// Build pattern based on current style
    fn build_pattern(&mut self) {
        self.voices.clear();
        let steps = self.config.steps_per_bar as usize;

        match self.config.style {
            DrumStyle::FourOnFloor => {
                // Kick on every beat
                let kick_pattern: Vec<bool> = (0..steps).map(|i| i % 4 == 0).collect();
                // Snare on 2 and 4
                let snare_pattern: Vec<bool> = (0..steps).map(|i| i == 4 || i == 12).collect();
                // Closed hat on every eighth
                let hat_pattern: Vec<bool> = (0..steps).map(|i| i % 2 == 0).collect();
                // Open hat on off-beats occasionally
                let open_hat_pattern: Vec<bool> = (0..steps).map(|i| i == 7 || i == 15).collect();

                self.voices.insert(
                    "kick".to_string(),
                    DrumVoice::new(gm_drums::KICK)
                        .with_pattern(kick_pattern)
                        .with_accents((0..steps).map(|i| i == 0).collect()),
                );
                self.voices.insert(
                    "snare".to_string(),
                    DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern),
                );
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
                self.voices.insert(
                    "open_hat".to_string(),
                    DrumVoice::new(gm_drums::OPEN_HAT).with_pattern(open_hat_pattern),
                );
            }
            DrumStyle::Breakbeat => {
                // Syncopated kick
                let kick_pattern: Vec<bool> = (0..steps)
                    .map(|i| i == 0 || i == 6 || i == 10)
                    .collect();
                // Snare with ghost notes
                let snare_pattern: Vec<bool> = (0..steps).map(|i| i == 4 || i == 12).collect();
                let snare_ghost: Vec<bool> = (0..steps).map(|i| i == 7 || i == 11).collect();
                // Busy hi-hats
                let hat_pattern: Vec<bool> = (0..steps).map(|_| true).collect();

                let mut snare = DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern);
                snare.ghost_pattern = snare_ghost;
                snare.ghost_velocity = 45;

                self.voices.insert(
                    "kick".to_string(),
                    DrumVoice::new(gm_drums::KICK).with_pattern(kick_pattern),
                );
                self.voices.insert("snare".to_string(), snare);
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
            }
            DrumStyle::Sparse => {
                // Minimal pattern
                let kick_pattern: Vec<bool> = (0..steps).map(|i| i == 0 || i == 10).collect();
                let snare_pattern: Vec<bool> = (0..steps).map(|i| i == 4 || i == 12).collect();
                let hat_pattern: Vec<bool> = (0..steps).map(|i| i % 4 == 0).collect();

                let mut kick = DrumVoice::new(gm_drums::KICK).with_pattern(kick_pattern);
                kick.probability = 0.9;

                self.voices.insert("kick".to_string(), kick);
                self.voices.insert(
                    "snare".to_string(),
                    DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern),
                );
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
            }
            DrumStyle::Busy => {
                // Dense, busy pattern
                let kick_pattern: Vec<bool> = (0..steps)
                    .map(|i| i == 0 || i == 3 || i == 6 || i == 10 || i == 14)
                    .collect();
                let snare_pattern: Vec<bool> = (0..steps)
                    .map(|i| i == 4 || i == 8 || i == 12)
                    .collect();
                let hat_pattern: Vec<bool> = (0..steps).map(|_| true).collect();
                let rim_pattern: Vec<bool> = (0..steps).map(|i| i == 2 || i == 7 || i == 15).collect();

                self.voices.insert(
                    "kick".to_string(),
                    DrumVoice::new(gm_drums::KICK).with_pattern(kick_pattern),
                );
                self.voices.insert(
                    "snare".to_string(),
                    DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern),
                );
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
                self.voices.insert(
                    "rim".to_string(),
                    DrumVoice::new(gm_drums::RIM).with_pattern(rim_pattern),
                );
            }
            DrumStyle::Euclidean => {
                let kick_pattern = Self::generate_euclidean(
                    self.config.kick_euclidean_hits as usize,
                    steps,
                );
                let snare_pattern = Self::generate_euclidean(
                    self.config.snare_euclidean_hits as usize,
                    steps,
                );
                let hat_pattern = Self::generate_euclidean(
                    self.config.hat_euclidean_hits as usize,
                    steps,
                );

                self.voices.insert(
                    "kick".to_string(),
                    DrumVoice::new(gm_drums::KICK).with_pattern(kick_pattern),
                );
                self.voices.insert(
                    "snare".to_string(),
                    DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern),
                );
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
            }
            DrumStyle::Random => {
                // Random patterns with configurable density
                let mut kick_pattern = vec![false; steps];
                let mut snare_pattern = vec![false; steps];
                let mut hat_pattern = vec![false; steps];

                for i in 0..steps {
                    kick_pattern[i] = self.rng.gen::<f64>() < 0.25;
                    snare_pattern[i] = self.rng.gen::<f64>() < 0.2;
                    hat_pattern[i] = self.rng.gen::<f64>() < 0.5;
                }

                // Ensure downbeat has kick
                kick_pattern[0] = true;

                self.voices.insert(
                    "kick".to_string(),
                    DrumVoice::new(gm_drums::KICK).with_pattern(kick_pattern),
                );
                self.voices.insert(
                    "snare".to_string(),
                    DrumVoice::new(gm_drums::SNARE).with_pattern(snare_pattern),
                );
                self.voices.insert(
                    "hat".to_string(),
                    DrumVoice::new(gm_drums::CLOSED_HAT).with_pattern(hat_pattern),
                );
            }
        }
    }

    /// Generate a fill pattern
    fn generate_fill(&mut self) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let steps = self.config.steps_per_bar as usize;

        // Tom fill across last few steps
        let fill_notes = [gm_drums::HIGH_TOM, gm_drums::MID_TOM, gm_drums::LOW_TOM, gm_drums::KICK];

        for i in 12..steps {
            let note = fill_notes[(i - 12) % fill_notes.len()];
            let velocity = 100 + self.rng.gen_range(0..20);
            events.push(MidiEvent::new(note, velocity, 0, 6));
        }

        events
    }

    /// Apply humanization to a velocity
    fn humanize_velocity(&mut self, velocity: u8) -> u8 {
        let var = self.config.humanize_velocity as i16;
        let offset = self.rng.gen_range(-var..=var);
        (velocity as i16 + offset).clamp(1, 127) as u8
    }
}

impl Default for DrumGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for DrumGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let ticks_per_step = context.ticks_per_bar() / self.config.steps_per_bar as u64;

        let mut tick = 0u64;
        while tick < context.ticks_to_generate {
            let step = self.current_step;

            // Check for fill
            if step == 12 && self.config.fill_every_bars > 0 {
                if (self.current_bar + 1) % self.config.fill_every_bars as u64 == 0 {
                    if self.rng.gen::<f64>() < self.config.fill_probability {
                        self.in_fill = true;
                    }
                }
            }

            if self.in_fill && step >= 12 {
                // Generate fill
                let fill_events = self.generate_fill();
                for mut event in fill_events {
                    event.start_tick += tick;
                    events.push(event);
                }
            } else {
                // Collect voice data first
                let voice_data: Vec<_> = self.voices.values()
                    .filter(|v| v.enabled)
                    .map(|voice| {
                        let step_idx = step % voice.pattern.len();
                        let should_hit = voice.pattern.get(step_idx).copied().unwrap_or(false);
                        let is_accent = voice.accent_pattern.get(step_idx).copied().unwrap_or(false);
                        let should_ghost = voice.ghost_pattern.get(step_idx).copied().unwrap_or(false);
                        (voice.note, voice.velocity, voice.accent_velocity, voice.ghost_velocity,
                         voice.probability, should_hit, is_accent, should_ghost)
                    })
                    .collect();

                // Now generate events
                for (note, vel, accent_vel, ghost_vel, prob, should_hit, is_accent, should_ghost) in voice_data {
                    // Check main hit
                    if should_hit {
                        if self.rng.gen::<f64>() < prob {
                            let base_vel = if is_accent { accent_vel } else { vel };
                            let velocity = self.humanize_velocity(base_vel);
                            events.push(MidiEvent::new(note, velocity, tick, ticks_per_step));
                        }
                    }

                    // Check ghost note
                    if should_ghost {
                        let velocity = self.humanize_velocity(ghost_vel);
                        events.push(MidiEvent::new(note, velocity, tick, ticks_per_step / 2));
                    }
                }
            }

            // Advance step
            self.current_step = (self.current_step + 1) % self.config.steps_per_bar as usize;
            if self.current_step == 0 {
                self.current_bar += 1;
                self.in_fill = false;
            }

            tick += ticks_per_step;
        }

        self.tick_accumulator += context.ticks_to_generate;
        events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        let rebuild = match name {
            "style" => {
                self.config.style = DrumStyle::from_value(value as u8);
                true
            }
            "swing" => {
                self.config.swing = value.clamp(0.0, 1.0);
                false
            }
            "humanize_timing" => {
                self.config.humanize_timing = value.clamp(0.0, 50.0);
                false
            }
            "humanize_velocity" => {
                self.config.humanize_velocity = (value as u8).min(30);
                false
            }
            "fill_probability" => {
                self.config.fill_probability = value.clamp(0.0, 1.0);
                false
            }
            "fill_every_bars" => {
                self.config.fill_every_bars = (value as u8).clamp(1, 16);
                false
            }
            "kick_euclidean_hits" => {
                self.config.kick_euclidean_hits = (value as u8).clamp(1, 16);
                self.config.style == DrumStyle::Euclidean
            }
            "snare_euclidean_hits" => {
                self.config.snare_euclidean_hits = (value as u8).clamp(1, 16);
                self.config.style == DrumStyle::Euclidean
            }
            "hat_euclidean_hits" => {
                self.config.hat_euclidean_hits = (value as u8).clamp(1, 16);
                self.config.style == DrumStyle::Euclidean
            }
            _ => false,
        };

        if rebuild {
            self.build_pattern();
        }
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "style" => Some(self.config.style.to_value() as f64),
            "swing" => Some(self.config.swing),
            "humanize_timing" => Some(self.config.humanize_timing),
            "humanize_velocity" => Some(self.config.humanize_velocity as f64),
            "fill_probability" => Some(self.config.fill_probability),
            "fill_every_bars" => Some(self.config.fill_every_bars as f64),
            "kick_euclidean_hits" => Some(self.config.kick_euclidean_hits as f64),
            "snare_euclidean_hits" => Some(self.config.snare_euclidean_hits as f64),
            "hat_euclidean_hits" => Some(self.config.hat_euclidean_hits as f64),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.current_step = 0;
        self.current_bar = 0;
        self.tick_accumulator = 0;
        self.in_fill = false;
    }

    fn name(&self) -> &'static str {
        "drums"
    }

    fn params(&self) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        params.insert("style".to_string(), self.config.style.to_value() as f64);
        params.insert("swing".to_string(), self.config.swing);
        params.insert("humanize_timing".to_string(), self.config.humanize_timing);
        params.insert("humanize_velocity".to_string(), self.config.humanize_velocity as f64);
        params.insert("fill_probability".to_string(), self.config.fill_probability);
        params.insert("fill_every_bars".to_string(), self.config.fill_every_bars as f64);
        params.insert("kick_euclidean_hits".to_string(), self.config.kick_euclidean_hits as f64);
        params.insert("snare_euclidean_hits".to_string(), self.config.snare_euclidean_hits as f64);
        params.insert("hat_euclidean_hits".to_string(), self.config.hat_euclidean_hits as f64);
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
    fn test_drums_creation() {
        let drums = DrumGenerator::new();
        assert_eq!(drums.name(), "drums");
    }

    #[test]
    fn test_drums_generates_events() {
        let mut drums = DrumGenerator::new();
        let ctx = test_context();

        let events = drums.generate(&ctx);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_drums_gm_notes() {
        // Verify GM drum note mappings
        assert_eq!(gm_drums::KICK, 36);
        assert_eq!(gm_drums::SNARE, 38);
        assert_eq!(gm_drums::CLOSED_HAT, 42);
    }

    #[test]
    fn test_drum_styles() {
        let mut drums = DrumGenerator::new();
        let ctx = test_context();

        for style_val in 0..6 {
            drums.set_param("style", style_val as f64);
            let events = drums.generate(&ctx);
            assert!(!events.is_empty(), "Style {} produced no events", style_val);
            drums.reset();
        }
    }

    #[test]
    fn test_euclidean_patterns() {
        // Classic 3-over-8
        let pattern = DrumGenerator::generate_euclidean(3, 8);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 3);

        // 5-over-8
        let pattern = DrumGenerator::generate_euclidean(5, 8);
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 5);

        // 4-over-16 (four-on-floor)
        let pattern = DrumGenerator::generate_euclidean(4, 16);
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 4);
    }

    #[test]
    fn test_drums_param_changes() {
        let mut drums = DrumGenerator::new();

        drums.set_param("style", 1.0);
        assert_eq!(drums.get_param("style"), Some(1.0));

        drums.set_param("humanize_velocity", 10.0);
        assert_eq!(drums.get_param("humanize_velocity"), Some(10.0));

        drums.set_param("fill_probability", 0.5);
        assert_eq!(drums.get_param("fill_probability"), Some(0.5));
    }

    #[test]
    fn test_drums_reset() {
        let mut drums = DrumGenerator::new();
        let ctx = test_context();

        drums.generate(&ctx);
        assert!(drums.tick_accumulator > 0);

        drums.reset();
        assert_eq!(drums.tick_accumulator, 0);
        assert_eq!(drums.current_step, 0);
        assert_eq!(drums.current_bar, 0);
    }

    #[test]
    fn test_four_on_floor_pattern() {
        let mut drums = DrumGenerator::new();
        drums.set_param("style", 0.0); // FourOnFloor
        drums.set_param("humanize_velocity", 0.0); // Disable for predictable testing

        let ctx = test_context();
        let events = drums.generate(&ctx);

        // Should have kick, snare, and hat events
        let kick_events: Vec<_> = events.iter().filter(|e| e.note == gm_drums::KICK).collect();
        let snare_events: Vec<_> = events.iter().filter(|e| e.note == gm_drums::SNARE).collect();

        assert!(!kick_events.is_empty(), "Should have kick events");
        assert!(!snare_events.is_empty(), "Should have snare events");
    }
}
