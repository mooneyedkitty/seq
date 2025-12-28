// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Generative engines for algorithmic music creation.
//!
//! This module provides various generators that produce MIDI events
//! algorithmically based on musical rules and probability.

pub mod arpeggio;
pub mod chord;
pub mod drone;
pub mod drums;
pub mod melody;

use std::collections::HashMap;
use std::fmt;

use crate::music::scale::{Key, Note, Scale, ScaleType};

/// MIDI event produced by generators
#[derive(Debug, Clone, PartialEq)]
pub struct MidiEvent {
    /// MIDI note number (0-127)
    pub note: u8,
    /// Velocity (0-127)
    pub velocity: u8,
    /// Start time in ticks from current position
    pub start_tick: u64,
    /// Duration in ticks
    pub duration_ticks: u64,
    /// MIDI channel (0-15)
    pub channel: u8,
}

impl MidiEvent {
    /// Create a new MIDI event
    pub fn new(note: u8, velocity: u8, start_tick: u64, duration_ticks: u64) -> Self {
        Self {
            note,
            velocity,
            start_tick,
            duration_ticks,
            channel: 0,
        }
    }

    /// Set the channel for this event
    pub fn with_channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }
}

/// Context provided to generators for generating events
#[derive(Debug, Clone)]
pub struct GeneratorContext {
    /// Current tempo in BPM
    pub tempo: f64,
    /// Ticks per quarter note (PPQN)
    pub ppqn: u32,
    /// Current beat (0-indexed)
    pub beat: u64,
    /// Current tick within beat
    pub tick: u32,
    /// Current bar (0-indexed)
    pub bar: u64,
    /// Beats per bar (time signature numerator)
    pub beats_per_bar: u8,
    /// Current musical key
    pub key: Key,
    /// Number of ticks to generate
    pub ticks_to_generate: u64,
    /// Global swing amount (0.0 - 1.0)
    pub swing: f64,
}

impl Default for GeneratorContext {
    fn default() -> Self {
        Self {
            tempo: 120.0,
            ppqn: 24,
            beat: 0,
            tick: 0,
            bar: 0,
            beats_per_bar: 4,
            key: Key::new(Note::C, ScaleType::Major),
            ticks_to_generate: 24, // One beat
            swing: 0.0,
        }
    }
}

impl GeneratorContext {
    /// Get total ticks from start
    pub fn total_ticks(&self) -> u64 {
        self.bar * self.beats_per_bar as u64 * self.ppqn as u64
            + self.beat * self.ppqn as u64
            + self.tick as u64
    }

    /// Get the scale for this context
    pub fn scale(&self) -> &Scale {
        self.key.scale()
    }

    /// Calculate ticks per beat
    pub fn ticks_per_beat(&self) -> u64 {
        self.ppqn as u64
    }

    /// Calculate ticks per bar
    pub fn ticks_per_bar(&self) -> u64 {
        self.ppqn as u64 * self.beats_per_bar as u64
    }

    /// Calculate duration in ticks for a note value
    /// division: 1 = whole, 2 = half, 4 = quarter, 8 = eighth, etc.
    pub fn note_duration(&self, division: u32) -> u64 {
        (self.ppqn as u64 * 4) / division as u64
    }
}

/// Trait for all generator implementations
pub trait Generator: Send {
    /// Generate MIDI events for the given context
    ///
    /// Returns a vector of MIDI events that should be played
    /// during the time window specified by the context.
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent>;

    /// Set a parameter by name
    ///
    /// Parameters are generator-specific and control behavior.
    fn set_param(&mut self, name: &str, value: f64);

    /// Get a parameter by name
    fn get_param(&self, name: &str) -> Option<f64>;

    /// Reset the generator state
    ///
    /// Called when playback stops or the generator is restarted.
    fn reset(&mut self);

    /// Get the generator type name
    fn name(&self) -> &'static str;

    /// Get a list of available parameters with their current values
    fn params(&self) -> HashMap<String, f64>;
}

/// Factory function type for creating generators
pub type GeneratorFactory = fn() -> Box<dyn Generator>;

/// Registry for generator types
#[derive(Default)]
pub struct GeneratorRegistry {
    factories: HashMap<String, GeneratorFactory>,
}

impl GeneratorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all built-in generators registered
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register("drone", drone::DroneGenerator::create);
        registry.register("arpeggio", arpeggio::ArpeggioGenerator::create);
        registry.register("chord", chord::ChordGenerator::create);
        registry.register("melody", melody::MelodyGenerator::create);
        registry.register("drums", drums::DrumGenerator::create);
        registry
    }

    /// Register a generator factory
    pub fn register(&mut self, name: &str, factory: GeneratorFactory) {
        self.factories.insert(name.to_string(), factory);
    }

    /// Create a generator by name
    pub fn create(&self, name: &str) -> Option<Box<dyn Generator>> {
        self.factories.get(name).map(|factory| factory())
    }

    /// Get list of registered generator names
    pub fn available(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

impl fmt::Debug for GeneratorRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeneratorRegistry")
            .field("generators", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockGenerator {
        counter: u64,
        params: HashMap<String, f64>,
    }

    impl MockGenerator {
        fn new() -> Self {
            let mut params = HashMap::new();
            params.insert("density".to_string(), 0.5);
            Self { counter: 0, params }
        }

        fn create() -> Box<dyn Generator> {
            Box::new(MockGenerator::new())
        }
    }

    impl Generator for MockGenerator {
        fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
            self.counter += 1;
            vec![MidiEvent::new(60, 100, 0, context.ppqn as u64 / 2)]
        }

        fn set_param(&mut self, name: &str, value: f64) {
            self.params.insert(name.to_string(), value);
        }

        fn get_param(&self, name: &str) -> Option<f64> {
            self.params.get(name).copied()
        }

        fn reset(&mut self) {
            self.counter = 0;
        }

        fn name(&self) -> &'static str {
            "mock"
        }

        fn params(&self) -> HashMap<String, f64> {
            self.params.clone()
        }
    }

    #[test]
    fn test_midi_event_creation() {
        let event = MidiEvent::new(60, 100, 0, 12);
        assert_eq!(event.note, 60);
        assert_eq!(event.velocity, 100);
        assert_eq!(event.start_tick, 0);
        assert_eq!(event.duration_ticks, 12);
        assert_eq!(event.channel, 0);

        let event = event.with_channel(5);
        assert_eq!(event.channel, 5);
    }

    #[test]
    fn test_generator_context_defaults() {
        let ctx = GeneratorContext::default();
        assert_eq!(ctx.tempo, 120.0);
        assert_eq!(ctx.ppqn, 24);
        assert_eq!(ctx.beats_per_bar, 4);
    }

    #[test]
    fn test_generator_context_timing() {
        let ctx = GeneratorContext {
            bar: 2,
            beat: 3,
            tick: 12,
            ppqn: 24,
            beats_per_bar: 4,
            ..Default::default()
        };

        // Bar 2, beat 3, tick 12 = 2*4*24 + 3*24 + 12 = 192 + 72 + 12 = 276
        assert_eq!(ctx.total_ticks(), 276);
        assert_eq!(ctx.ticks_per_beat(), 24);
        assert_eq!(ctx.ticks_per_bar(), 96);
    }

    #[test]
    fn test_note_duration() {
        let ctx = GeneratorContext {
            ppqn: 24,
            ..Default::default()
        };

        assert_eq!(ctx.note_duration(1), 96);  // Whole note
        assert_eq!(ctx.note_duration(2), 48);  // Half note
        assert_eq!(ctx.note_duration(4), 24);  // Quarter note
        assert_eq!(ctx.note_duration(8), 12);  // Eighth note
        assert_eq!(ctx.note_duration(16), 6);  // Sixteenth note
    }

    #[test]
    fn test_mock_generator() {
        let mut gen = MockGenerator::new();
        let ctx = GeneratorContext::default();

        let events = gen.generate(&ctx);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].note, 60);

        gen.set_param("density", 0.8);
        assert_eq!(gen.get_param("density"), Some(0.8));

        gen.reset();
        assert_eq!(gen.name(), "mock");
    }

    #[test]
    fn test_generator_registry() {
        let mut registry = GeneratorRegistry::new();
        registry.register("mock", MockGenerator::create);

        let available = registry.available();
        assert!(available.contains(&"mock".to_string()));

        let gen = registry.create("mock");
        assert!(gen.is_some());
        assert_eq!(gen.unwrap().name(), "mock");

        let missing = registry.create("nonexistent");
        assert!(missing.is_none());
    }
}
