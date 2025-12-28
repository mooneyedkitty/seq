// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Melodic generator using Markov chains and motif development.
//!
//! Generates melodies based on interval probabilities, rhythmic templates,
//! and motif transformations (repeat, transpose, invert, retrograde).

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::{Generator, GeneratorContext, MidiEvent};

/// Motif transformation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotifTransform {
    /// Play as-is
    Original,
    /// Repeat the motif
    Repeat,
    /// Transpose up/down by scale degrees
    Transpose(i8),
    /// Invert intervals (up becomes down)
    Invert,
    /// Play backwards
    Retrograde,
    /// Invert and play backwards
    RetroInvert,
}

/// A musical motif (short melodic fragment)
#[derive(Debug, Clone)]
struct Motif {
    /// Intervals from first note (in scale degrees)
    intervals: Vec<i8>,
    /// Rhythm as note divisions (4=quarter, 8=eighth, etc.)
    rhythm: Vec<u32>,
}

impl Motif {
    fn new(intervals: Vec<i8>, rhythm: Vec<u32>) -> Self {
        Self { intervals, rhythm }
    }

    /// Transform the motif
    fn transform(&self, transform: MotifTransform) -> Self {
        match transform {
            MotifTransform::Original | MotifTransform::Repeat => self.clone(),
            MotifTransform::Transpose(offset) => {
                let intervals = self.intervals.iter().map(|i| i + offset).collect();
                Self { intervals, rhythm: self.rhythm.clone() }
            }
            MotifTransform::Invert => {
                let intervals = self.intervals.iter().map(|i| -i).collect();
                Self { intervals, rhythm: self.rhythm.clone() }
            }
            MotifTransform::Retrograde => {
                let mut intervals = self.intervals.clone();
                let mut rhythm = self.rhythm.clone();
                intervals.reverse();
                rhythm.reverse();
                Self { intervals, rhythm }
            }
            MotifTransform::RetroInvert => {
                let mut intervals: Vec<i8> = self.intervals.iter().map(|i| -i).collect();
                let mut rhythm = self.rhythm.clone();
                intervals.reverse();
                rhythm.reverse();
                Self { intervals, rhythm }
            }
        }
    }
}

/// Configuration for melody generator
#[derive(Debug, Clone)]
struct MelodyConfig {
    /// Base octave
    base_octave: i8,
    /// Octave range
    octave_range: u8,
    /// Base velocity
    velocity: u8,
    /// Velocity variation
    velocity_variation: u8,
    /// Note rate as division
    base_rate: u32,
    /// Gate percentage
    gate: f64,
    /// Probability of step motion (vs skip)
    step_probability: f64,
    /// Probability of repeating a note
    repeat_probability: f64,
    /// Probability of rest
    rest_probability: f64,
    /// Maximum interval jump (scale degrees)
    max_jump: u8,
    /// Use motif development
    use_motifs: bool,
    /// Motif length in notes
    motif_length: u8,
    /// Rhythmic complexity (0.0 = simple, 1.0 = complex)
    rhythmic_complexity: f64,
}

impl Default for MelodyConfig {
    fn default() -> Self {
        Self {
            base_octave: 4,
            octave_range: 2,
            velocity: 100,
            velocity_variation: 15,
            base_rate: 8, // Eighth notes
            gate: 0.85,
            step_probability: 0.7,
            repeat_probability: 0.1,
            rest_probability: 0.15,
            max_jump: 4,
            use_motifs: true,
            motif_length: 4,
            rhythmic_complexity: 0.5,
        }
    }
}

/// Interval transition probabilities (Markov weights)
#[derive(Debug, Clone)]
struct IntervalProbabilities {
    /// Weights for each interval (-7 to +7 scale degrees)
    weights: [f64; 15],
}

impl Default for IntervalProbabilities {
    fn default() -> Self {
        // Default: prefer stepwise motion and small jumps
        Self {
            weights: [
                0.02, 0.05, 0.08, 0.12, 0.15, // -5, -4, -3, -2, -1
                0.20, 0.08, 0.05, // 0 (repeat), +1, +2
                0.08, 0.05, 0.04, 0.03, 0.02, 0.02, 0.01, // +3 to +7
            ],
        }
    }
}

impl IntervalProbabilities {
    /// Sample an interval based on weights
    fn sample(&self, rng: &mut StdRng) -> i8 {
        let total: f64 = self.weights.iter().sum();
        let mut roll = rng.gen::<f64>() * total;

        for (i, &weight) in self.weights.iter().enumerate() {
            roll -= weight;
            if roll <= 0.0 {
                return (i as i8) - 7; // Convert to -7..+7
            }
        }
        0 // Default to repeat
    }
}

/// Melody generator
pub struct MelodyGenerator {
    config: MelodyConfig,
    interval_probs: IntervalProbabilities,
    /// Current note (MIDI)
    current_note: Option<u8>,
    /// Current scale degree (1-based)
    current_degree: u8,
    /// Current motif being developed
    current_motif: Option<Motif>,
    /// Position in current motif
    motif_position: usize,
    /// How many times motif has been played
    motif_repetitions: u8,
    /// Tick accumulator
    tick_accumulator: u64,
    rng: StdRng,
}

impl MelodyGenerator {
    /// Create a new melody generator
    pub fn new() -> Self {
        Self {
            config: MelodyConfig::default(),
            interval_probs: IntervalProbabilities::default(),
            current_note: None,
            current_degree: 1,
            current_motif: None,
            motif_position: 0,
            motif_repetitions: 0,
            tick_accumulator: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Factory function for registry
    pub fn create() -> Box<dyn Generator> {
        Box::new(Self::new())
    }

    /// Generate a random velocity
    fn random_velocity(&mut self) -> u8 {
        let base = self.config.velocity as i16;
        let var = self.config.velocity_variation as i16;
        let offset = self.rng.gen_range(-var..=var);
        (base + offset).clamp(1, 127) as u8
    }

    /// Choose next interval based on Markov probabilities
    fn choose_interval(&mut self) -> i8 {
        if self.rng.gen::<f64>() < self.config.repeat_probability {
            return 0;
        }

        if self.rng.gen::<f64>() < self.config.step_probability {
            // Stepwise motion
            if self.rng.gen() {
                1
            } else {
                -1
            }
        } else {
            // Use full probability distribution
            let interval = self.interval_probs.sample(&mut self.rng);
            interval.clamp(-(self.config.max_jump as i8), self.config.max_jump as i8)
        }
    }

    /// Get note for current degree
    fn note_for_degree(&self, degree: u8, context: &GeneratorContext) -> Option<u8> {
        let scale = context.scale();
        let scale_len = scale.len() as u8;

        // Handle degrees outside 1-7 range
        let octave_offset = if degree == 0 {
            -1
        } else {
            ((degree - 1) / scale_len) as i8
        };
        let actual_degree = if degree == 0 {
            scale_len as usize
        } else {
            ((degree - 1) % scale_len + 1) as usize
        };

        let octave = self.config.base_octave + octave_offset;
        if octave < 0 || octave > 9 {
            return None;
        }

        scale.midi_note_at(actual_degree, octave)
    }

    /// Move to next degree based on interval
    fn move_by_interval(&mut self, interval: i8, context: &GeneratorContext) {
        let scale = context.scale();
        let scale_len = scale.len() as i8;

        let new_degree = self.current_degree as i8 + interval;

        // Keep within octave range
        let min_degree = 1i8;
        let max_degree = 1 + (self.config.octave_range as i8 * scale_len);

        self.current_degree = new_degree.clamp(min_degree, max_degree) as u8;
    }

    /// Generate a new motif
    fn generate_motif(&mut self) -> Motif {
        let mut intervals = Vec::new();
        let mut rhythm = Vec::new();

        let length = self.config.motif_length;
        let base_rate = self.config.base_rate;

        for i in 0..length {
            if i == 0 {
                intervals.push(0); // First note is reference
            } else {
                intervals.push(self.choose_interval());
            }

            // Generate rhythm based on complexity
            let div = if self.rng.gen::<f64>() < self.config.rhythmic_complexity {
                // More complex: vary the rhythm
                let options = [base_rate, base_rate * 2, base_rate / 2];
                options[self.rng.gen_range(0..options.len())]
            } else {
                base_rate
            };
            rhythm.push(div.max(1));
        }

        Motif::new(intervals, rhythm)
    }

    /// Choose a motif transformation
    fn choose_transform(&mut self) -> MotifTransform {
        let roll = self.rng.gen::<f64>();
        if roll < 0.3 {
            MotifTransform::Original
        } else if roll < 0.5 {
            MotifTransform::Repeat
        } else if roll < 0.7 {
            let offset = self.rng.gen_range(-3..=3);
            MotifTransform::Transpose(offset)
        } else if roll < 0.85 {
            MotifTransform::Invert
        } else if roll < 0.95 {
            MotifTransform::Retrograde
        } else {
            MotifTransform::RetroInvert
        }
    }
}

impl Default for MelodyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for MelodyGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        let mut events = Vec::new();

        // Initialize if needed
        if self.current_note.is_none() {
            self.current_degree = 1;
            if let Some(note) = self.note_for_degree(self.current_degree, context) {
                self.current_note = Some(note);
            }
        }

        let base_duration = context.note_duration(self.config.base_rate);
        let mut tick = 0u64;

        while tick < context.ticks_to_generate {
            // Check for rest
            if self.rng.gen::<f64>() < self.config.rest_probability {
                tick += base_duration;
                continue;
            }

            // Determine interval and duration
            let (interval, duration) = if self.config.use_motifs {
                // Motif-based generation
                if self.current_motif.is_none() || self.motif_position >= self.config.motif_length as usize {
                    // Generate or transform motif
                    if self.current_motif.is_none() || self.motif_repetitions >= 3 {
                        self.current_motif = Some(self.generate_motif());
                        self.motif_repetitions = 0;
                    } else {
                        let transform = self.choose_transform();
                        if let Some(ref motif) = self.current_motif {
                            self.current_motif = Some(motif.transform(transform));
                        }
                        self.motif_repetitions += 1;
                    }
                    self.motif_position = 0;
                }

                if let Some(ref motif) = self.current_motif {
                    let interval = motif.intervals.get(self.motif_position).copied().unwrap_or(0);
                    let rhythm = motif.rhythm.get(self.motif_position).copied().unwrap_or(self.config.base_rate);
                    self.motif_position += 1;
                    (interval, context.note_duration(rhythm))
                } else {
                    (self.choose_interval(), base_duration)
                }
            } else {
                // Free generation
                (self.choose_interval(), base_duration)
            };

            // Move by interval
            self.move_by_interval(interval, context);

            // Get the note
            if let Some(note) = self.note_for_degree(self.current_degree, context) {
                self.current_note = Some(note);

                let note_length = (duration as f64 * self.config.gate) as u64;
                events.push(MidiEvent::new(
                    note,
                    self.random_velocity(),
                    tick,
                    note_length,
                ));
            }

            tick += duration;
        }

        self.tick_accumulator += context.ticks_to_generate;
        events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        match name {
            "base_octave" => self.config.base_octave = (value as i8).clamp(1, 7),
            "octave_range" => self.config.octave_range = (value as u8).clamp(1, 4),
            "velocity" => self.config.velocity = (value as u8).clamp(1, 127),
            "velocity_variation" => self.config.velocity_variation = (value as u8).min(64),
            "base_rate" => self.config.base_rate = (value as u32).clamp(1, 32),
            "gate" => self.config.gate = value.clamp(0.1, 1.0),
            "step_probability" => self.config.step_probability = value.clamp(0.0, 1.0),
            "repeat_probability" => self.config.repeat_probability = value.clamp(0.0, 1.0),
            "rest_probability" => self.config.rest_probability = value.clamp(0.0, 0.5),
            "max_jump" => self.config.max_jump = (value as u8).clamp(1, 7),
            "use_motifs" => self.config.use_motifs = value > 0.5,
            "motif_length" => self.config.motif_length = (value as u8).clamp(2, 8),
            "rhythmic_complexity" => self.config.rhythmic_complexity = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "base_octave" => Some(self.config.base_octave as f64),
            "octave_range" => Some(self.config.octave_range as f64),
            "velocity" => Some(self.config.velocity as f64),
            "velocity_variation" => Some(self.config.velocity_variation as f64),
            "base_rate" => Some(self.config.base_rate as f64),
            "gate" => Some(self.config.gate),
            "step_probability" => Some(self.config.step_probability),
            "repeat_probability" => Some(self.config.repeat_probability),
            "rest_probability" => Some(self.config.rest_probability),
            "max_jump" => Some(self.config.max_jump as f64),
            "use_motifs" => Some(if self.config.use_motifs { 1.0 } else { 0.0 }),
            "motif_length" => Some(self.config.motif_length as f64),
            "rhythmic_complexity" => Some(self.config.rhythmic_complexity),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.current_note = None;
        self.current_degree = 1;
        self.current_motif = None;
        self.motif_position = 0;
        self.motif_repetitions = 0;
        self.tick_accumulator = 0;
    }

    fn name(&self) -> &'static str {
        "melody"
    }

    fn params(&self) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        params.insert("base_octave".to_string(), self.config.base_octave as f64);
        params.insert("octave_range".to_string(), self.config.octave_range as f64);
        params.insert("velocity".to_string(), self.config.velocity as f64);
        params.insert("velocity_variation".to_string(), self.config.velocity_variation as f64);
        params.insert("base_rate".to_string(), self.config.base_rate as f64);
        params.insert("gate".to_string(), self.config.gate);
        params.insert("step_probability".to_string(), self.config.step_probability);
        params.insert("repeat_probability".to_string(), self.config.repeat_probability);
        params.insert("rest_probability".to_string(), self.config.rest_probability);
        params.insert("max_jump".to_string(), self.config.max_jump as f64);
        params.insert("use_motifs".to_string(), if self.config.use_motifs { 1.0 } else { 0.0 });
        params.insert("motif_length".to_string(), self.config.motif_length as f64);
        params.insert("rhythmic_complexity".to_string(), self.config.rhythmic_complexity);
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
    fn test_melody_creation() {
        let melody = MelodyGenerator::new();
        assert_eq!(melody.name(), "melody");
    }

    #[test]
    fn test_melody_generates_notes() {
        let mut melody = MelodyGenerator::new();
        melody.set_param("rest_probability", 0.0); // No rests for testing

        let ctx = test_context();
        let events = melody.generate(&ctx);

        assert!(!events.is_empty());
    }

    #[test]
    fn test_melody_notes_in_scale() {
        let mut melody = MelodyGenerator::new();
        melody.set_param("rest_probability", 0.0);

        let ctx = test_context();
        let events = melody.generate(&ctx);
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
    fn test_motif_transforms() {
        let motif = Motif::new(vec![0, 2, 4, 2], vec![8, 8, 8, 8]);

        // Invert
        let inverted = motif.transform(MotifTransform::Invert);
        assert_eq!(inverted.intervals, vec![0, -2, -4, -2]);

        // Retrograde
        let retro = motif.transform(MotifTransform::Retrograde);
        assert_eq!(retro.intervals, vec![2, 4, 2, 0]);

        // Transpose
        let transposed = motif.transform(MotifTransform::Transpose(3));
        assert_eq!(transposed.intervals, vec![3, 5, 7, 5]);
    }

    #[test]
    fn test_melody_param_changes() {
        let mut melody = MelodyGenerator::new();

        melody.set_param("base_octave", 5.0);
        assert_eq!(melody.get_param("base_octave"), Some(5.0));

        melody.set_param("step_probability", 0.9);
        assert_eq!(melody.get_param("step_probability"), Some(0.9));

        melody.set_param("use_motifs", 0.0);
        assert_eq!(melody.get_param("use_motifs"), Some(0.0));
    }

    #[test]
    fn test_melody_reset() {
        let mut melody = MelodyGenerator::new();
        let ctx = test_context();

        melody.generate(&ctx);
        assert!(melody.current_note.is_some());

        melody.reset();
        assert!(melody.current_note.is_none());
        assert_eq!(melody.tick_accumulator, 0);
    }

    #[test]
    fn test_interval_probabilities() {
        let probs = IntervalProbabilities::default();
        let mut rng = StdRng::from_entropy();

        // Sample many times and verify we get valid intervals
        for _ in 0..100 {
            let interval = probs.sample(&mut rng);
            assert!(interval >= -7 && interval <= 7);
        }
    }
}
