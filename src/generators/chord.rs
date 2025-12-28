// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Chord generator for harmonic progressions.
//!
//! Generates chord progressions with various voicings, inversions,
//! and tension additions. Supports functional harmony and random-in-key modes.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::{Generator, GeneratorContext, MidiEvent};

/// Chord voicing types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Voicing {
    /// All notes close together
    Close,
    /// Root in bass, others spread
    Open,
    /// Drop the second-highest note an octave
    Drop2,
    /// Wide spread across octaves
    Spread,
}

impl Voicing {
    fn from_value(v: u8) -> Self {
        match v {
            0 => Voicing::Close,
            1 => Voicing::Open,
            2 => Voicing::Drop2,
            _ => Voicing::Spread,
        }
    }

    fn to_value(self) -> u8 {
        match self {
            Voicing::Close => 0,
            Voicing::Open => 1,
            Voicing::Drop2 => 2,
            Voicing::Spread => 3,
        }
    }
}

/// Inversion selection method
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InversionMode {
    /// Root position only
    Root,
    /// Random inversion
    Random,
    /// Choose smoothest voice leading
    VoiceLed,
    /// Cycle through inversions
    Ascending,
}

impl InversionMode {
    fn from_value(v: u8) -> Self {
        match v {
            0 => InversionMode::Root,
            1 => InversionMode::Random,
            2 => InversionMode::VoiceLed,
            _ => InversionMode::Ascending,
        }
    }

    fn to_value(self) -> u8 {
        match self {
            InversionMode::Root => 0,
            InversionMode::Random => 1,
            InversionMode::VoiceLed => 2,
            InversionMode::Ascending => 3,
        }
    }
}

/// Progression algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressionMode {
    /// Functional harmony (I-IV-V-I style)
    Functional,
    /// Random chords in key
    RandomInKey,
    /// User-defined progression
    Custom,
}

impl ProgressionMode {
    fn from_value(v: u8) -> Self {
        match v {
            0 => ProgressionMode::Functional,
            1 => ProgressionMode::RandomInKey,
            _ => ProgressionMode::Custom,
        }
    }

    fn to_value(self) -> u8 {
        match self {
            ProgressionMode::Functional => 0,
            ProgressionMode::RandomInKey => 1,
            ProgressionMode::Custom => 2,
        }
    }
}

/// Configuration for chord generator
#[derive(Debug, Clone)]
struct ChordConfig {
    /// Voicing type
    voicing: Voicing,
    /// Inversion selection
    inversion_mode: InversionMode,
    /// Progression algorithm
    progression_mode: ProgressionMode,
    /// How often chords change (in beats)
    change_rate: f64,
    /// Base octave
    base_octave: i8,
    /// Base velocity
    velocity: u8,
    /// Add 7ths probability (0.0 - 1.0)
    seventh_probability: f64,
    /// Add 9ths probability (0.0 - 1.0)
    ninth_probability: f64,
    /// Add sus probability (0.0 - 1.0)
    sus_probability: f64,
    /// Custom progression (scale degrees)
    custom_progression: Vec<u8>,
}

impl Default for ChordConfig {
    fn default() -> Self {
        Self {
            voicing: Voicing::Close,
            inversion_mode: InversionMode::Root,
            progression_mode: ProgressionMode::Functional,
            change_rate: 4.0, // One chord per bar
            base_octave: 3,
            velocity: 90,
            seventh_probability: 0.3,
            ninth_probability: 0.1,
            sus_probability: 0.1,
            custom_progression: vec![1, 4, 5, 1], // I-IV-V-I
        }
    }
}

/// Chord generator
pub struct ChordGenerator {
    config: ChordConfig,
    /// Current position in progression
    progression_position: usize,
    /// Current chord notes (MIDI)
    current_chord: Vec<u8>,
    /// Previous chord for voice leading
    previous_chord: Vec<u8>,
    /// Tick accumulator
    tick_accumulator: u64,
    /// Current inversion for ascending mode
    current_inversion: u8,
    rng: StdRng,
}

impl ChordGenerator {
    /// Create a new chord generator
    pub fn new() -> Self {
        Self {
            config: ChordConfig::default(),
            progression_position: 0,
            current_chord: Vec::new(),
            previous_chord: Vec::new(),
            tick_accumulator: 0,
            current_inversion: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Factory function for registry
    pub fn create() -> Box<dyn Generator> {
        Box::new(Self::new())
    }

    /// Get the next chord root degree based on progression mode
    fn next_root_degree(&mut self) -> u8 {
        match self.config.progression_mode {
            ProgressionMode::Functional => {
                // Common functional progressions
                let progressions = [
                    vec![1, 4, 5, 1],     // I-IV-V-I
                    vec![1, 5, 6, 4],     // I-V-vi-IV (pop)
                    vec![2, 5, 1, 1],     // ii-V-I
                    vec![1, 6, 4, 5],     // I-vi-IV-V
                    vec![1, 4, 6, 5],     // I-IV-vi-V
                ];
                let prog = &progressions[self.rng.gen_range(0..progressions.len())];
                let degree = prog[self.progression_position % prog.len()];
                self.progression_position += 1;
                degree
            }
            ProgressionMode::RandomInKey => {
                self.rng.gen_range(1..=7)
            }
            ProgressionMode::Custom => {
                if self.config.custom_progression.is_empty() {
                    return 1;
                }
                let degree = self.config.custom_progression
                    [self.progression_position % self.config.custom_progression.len()];
                self.progression_position += 1;
                degree
            }
        }
    }

    /// Build a chord from a root scale degree
    fn build_chord(&mut self, root_degree: u8, context: &GeneratorContext) -> Vec<u8> {
        let scale = context.scale();
        let mut notes = Vec::new();

        // Basic triad: 1, 3, 5 from root
        let degrees = [root_degree, root_degree + 2, root_degree + 4];

        for &deg in degrees.iter() {
            // Wrap degree to scale length
            let actual_deg = ((deg - 1) % scale.len() as u8) + 1;
            let octave_offset = ((deg - 1) / scale.len() as u8) as i8;

            if let Some(note) = scale.midi_note_at(actual_deg as usize, self.config.base_octave + octave_offset) {
                notes.push(note);
            }
        }

        // Add 7th
        if self.rng.gen::<f64>() < self.config.seventh_probability {
            let seventh_deg = root_degree + 6;
            let actual_deg = ((seventh_deg - 1) % scale.len() as u8) + 1;
            let octave_offset = ((seventh_deg - 1) / scale.len() as u8) as i8;
            if let Some(note) = scale.midi_note_at(actual_deg as usize, self.config.base_octave + octave_offset) {
                notes.push(note);
            }
        }

        // Add 9th
        if self.rng.gen::<f64>() < self.config.ninth_probability {
            let ninth_deg = root_degree + 8;
            let actual_deg = ((ninth_deg - 1) % scale.len() as u8) + 1;
            let octave_offset = ((ninth_deg - 1) / scale.len() as u8) as i8;
            if let Some(note) = scale.midi_note_at(actual_deg as usize, self.config.base_octave + octave_offset + 1) {
                notes.push(note);
            }
        }

        // Sus4 replaces 3rd
        if self.rng.gen::<f64>() < self.config.sus_probability && notes.len() >= 2 {
            let sus_deg = root_degree + 3; // 4th
            let actual_deg = ((sus_deg - 1) % scale.len() as u8) + 1;
            if let Some(note) = scale.midi_note_at(actual_deg as usize, self.config.base_octave) {
                notes[1] = note; // Replace 3rd with 4th
            }
        }

        // Apply voicing
        notes = self.apply_voicing(notes);

        // Apply inversion
        notes = self.apply_inversion(notes);

        notes
    }

    /// Apply voicing to chord notes
    fn apply_voicing(&self, mut notes: Vec<u8>) -> Vec<u8> {
        if notes.len() < 3 {
            return notes;
        }

        match self.config.voicing {
            Voicing::Close => {
                // Notes already close, just sort
                notes.sort();
            }
            Voicing::Open => {
                // Move middle notes up an octave
                notes.sort();
                if notes.len() >= 3 {
                    for i in 1..notes.len() - 1 {
                        if notes[i] + 12 <= 127 {
                            notes[i] += 12;
                        }
                    }
                }
                notes.sort();
            }
            Voicing::Drop2 => {
                // Drop second-highest note an octave
                notes.sort();
                if notes.len() >= 2 {
                    let idx = notes.len() - 2;
                    if notes[idx] >= 12 {
                        notes[idx] -= 12;
                    }
                }
                notes.sort();
            }
            Voicing::Spread => {
                // Distribute across 2+ octaves
                notes.sort();
                let len = notes.len();
                for (i, note) in notes.iter_mut().enumerate() {
                    let offset = ((i as i16 * 12) / (len as i16).max(1)) as i16;
                    let new_note = (*note as i16 + offset).clamp(0, 127) as u8;
                    *note = new_note;
                }
                notes.sort();
            }
        }

        notes
    }

    /// Apply inversion to chord
    fn apply_inversion(&mut self, mut notes: Vec<u8>) -> Vec<u8> {
        if notes.len() < 2 {
            return notes;
        }

        notes.sort();

        let inversion = match self.config.inversion_mode {
            InversionMode::Root => 0,
            InversionMode::Random => self.rng.gen_range(0..notes.len() as u8),
            InversionMode::Ascending => {
                let inv = self.current_inversion;
                self.current_inversion = (self.current_inversion + 1) % notes.len() as u8;
                inv
            }
            InversionMode::VoiceLed => {
                // Find inversion that minimizes movement from previous chord
                if self.previous_chord.is_empty() {
                    0
                } else {
                    let mut best_inv = 0u8;
                    let mut best_movement = i32::MAX;

                    for inv in 0..notes.len() {
                        let inverted = self.invert_chord(&notes, inv as u8);
                        let movement = self.calculate_movement(&inverted);
                        if movement < best_movement {
                            best_movement = movement;
                            best_inv = inv as u8;
                        }
                    }
                    best_inv
                }
            }
        };

        self.invert_chord(&notes, inversion)
    }

    /// Invert a chord by moving bottom notes up an octave
    fn invert_chord(&self, notes: &[u8], inversion: u8) -> Vec<u8> {
        let mut result = notes.to_vec();
        for _ in 0..inversion {
            if !result.is_empty() {
                let bottom = result.remove(0);
                if bottom + 12 <= 127 {
                    result.push(bottom + 12);
                } else {
                    result.push(bottom);
                }
            }
        }
        result.sort();
        result
    }

    /// Calculate total voice movement from previous chord
    fn calculate_movement(&self, notes: &[u8]) -> i32 {
        let mut total = 0i32;
        for (i, &note) in notes.iter().enumerate() {
            if let Some(&prev) = self.previous_chord.get(i) {
                total += (note as i32 - prev as i32).abs();
            }
        }
        total
    }
}

impl Default for ChordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for ChordGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let ticks_per_change = (context.ppqn as f64 * self.config.change_rate) as u64;

        // Check if we need a new chord
        if self.current_chord.is_empty() || self.tick_accumulator % ticks_per_change == 0 {
            self.previous_chord = self.current_chord.clone();
            let root = self.next_root_degree();
            self.current_chord = self.build_chord(root, context);
        }

        // Generate events for current chord
        for &note in &self.current_chord {
            events.push(MidiEvent::new(
                note,
                self.config.velocity,
                0,
                context.ticks_to_generate,
            ));
        }

        self.tick_accumulator += context.ticks_to_generate;
        events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        match name {
            "voicing" => self.config.voicing = Voicing::from_value(value as u8),
            "inversion_mode" => self.config.inversion_mode = InversionMode::from_value(value as u8),
            "progression_mode" => self.config.progression_mode = ProgressionMode::from_value(value as u8),
            "change_rate" => self.config.change_rate = value.max(0.25),
            "base_octave" => self.config.base_octave = (value as i8).clamp(1, 6),
            "velocity" => self.config.velocity = (value as u8).clamp(1, 127),
            "seventh_probability" => self.config.seventh_probability = value.clamp(0.0, 1.0),
            "ninth_probability" => self.config.ninth_probability = value.clamp(0.0, 1.0),
            "sus_probability" => self.config.sus_probability = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "voicing" => Some(self.config.voicing.to_value() as f64),
            "inversion_mode" => Some(self.config.inversion_mode.to_value() as f64),
            "progression_mode" => Some(self.config.progression_mode.to_value() as f64),
            "change_rate" => Some(self.config.change_rate),
            "base_octave" => Some(self.config.base_octave as f64),
            "velocity" => Some(self.config.velocity as f64),
            "seventh_probability" => Some(self.config.seventh_probability),
            "ninth_probability" => Some(self.config.ninth_probability),
            "sus_probability" => Some(self.config.sus_probability),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.progression_position = 0;
        self.current_chord.clear();
        self.previous_chord.clear();
        self.tick_accumulator = 0;
        self.current_inversion = 0;
    }

    fn name(&self) -> &'static str {
        "chord"
    }

    fn params(&self) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        params.insert("voicing".to_string(), self.config.voicing.to_value() as f64);
        params.insert("inversion_mode".to_string(), self.config.inversion_mode.to_value() as f64);
        params.insert("progression_mode".to_string(), self.config.progression_mode.to_value() as f64);
        params.insert("change_rate".to_string(), self.config.change_rate);
        params.insert("base_octave".to_string(), self.config.base_octave as f64);
        params.insert("velocity".to_string(), self.config.velocity as f64);
        params.insert("seventh_probability".to_string(), self.config.seventh_probability);
        params.insert("ninth_probability".to_string(), self.config.ninth_probability);
        params.insert("sus_probability".to_string(), self.config.sus_probability);
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
    fn test_chord_creation() {
        let chord = ChordGenerator::new();
        assert_eq!(chord.name(), "chord");
    }

    #[test]
    fn test_chord_generates_notes() {
        let mut chord = ChordGenerator::new();
        let ctx = test_context();

        let events = chord.generate(&ctx);
        assert!(!events.is_empty());

        // Should have at least 3 notes (triad)
        assert!(events.len() >= 3);
    }

    #[test]
    fn test_chord_notes_in_scale() {
        let mut chord = ChordGenerator::new();
        chord.set_param("seventh_probability", 0.0);
        chord.set_param("ninth_probability", 0.0);
        chord.set_param("sus_probability", 0.0);

        let ctx = test_context();
        let events = chord.generate(&ctx);
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
    fn test_voicing_types() {
        assert_eq!(Voicing::from_value(0), Voicing::Close);
        assert_eq!(Voicing::from_value(1), Voicing::Open);
        assert_eq!(Voicing::from_value(2), Voicing::Drop2);
        assert_eq!(Voicing::from_value(3), Voicing::Spread);
    }

    #[test]
    fn test_chord_param_changes() {
        let mut chord = ChordGenerator::new();

        chord.set_param("voicing", 1.0);
        assert_eq!(chord.get_param("voicing"), Some(1.0));

        chord.set_param("change_rate", 2.0);
        assert_eq!(chord.get_param("change_rate"), Some(2.0));

        chord.set_param("seventh_probability", 0.5);
        assert_eq!(chord.get_param("seventh_probability"), Some(0.5));
    }

    #[test]
    fn test_chord_reset() {
        let mut chord = ChordGenerator::new();
        let ctx = test_context();

        chord.generate(&ctx);
        assert!(!chord.current_chord.is_empty());

        chord.reset();
        assert!(chord.current_chord.is_empty());
        assert_eq!(chord.tick_accumulator, 0);
    }

    #[test]
    fn test_inversion_modes() {
        assert_eq!(InversionMode::from_value(0), InversionMode::Root);
        assert_eq!(InversionMode::from_value(1), InversionMode::Random);
        assert_eq!(InversionMode::from_value(2), InversionMode::VoiceLed);
        assert_eq!(InversionMode::from_value(3), InversionMode::Ascending);
    }
}
