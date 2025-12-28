// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Sequencer core for scheduling and playing MIDI events.
//!
//! This module provides the core sequencing infrastructure:
//! - Event scheduler with microsecond-precision timing
//! - Track system for multi-channel output
//! - Clip system for sequenced and generated content
//! - Pattern triggering with quantization

pub mod clip;
pub mod scheduler;
pub mod track;
pub mod trigger;

pub use clip::{Clip, ClipMode, ClipState};
pub use scheduler::{ScheduledEvent, Scheduler};
pub use track::{Track, TrackState};
pub use trigger::{FollowAction, QuantizeMode, TriggerQueue};

/// Timing information for the sequencer
#[derive(Debug, Clone, Copy)]
pub struct SequencerTiming {
    /// Current tempo in BPM
    pub tempo: f64,
    /// Ticks per quarter note
    pub ppqn: u32,
    /// Current position in ticks from start
    pub position_ticks: u64,
    /// Beats per bar (time signature numerator)
    pub beats_per_bar: u8,
    /// Beat unit (time signature denominator, 4 = quarter note)
    pub beat_unit: u8,
}

impl Default for SequencerTiming {
    fn default() -> Self {
        Self {
            tempo: 120.0,
            ppqn: 24,
            position_ticks: 0,
            beats_per_bar: 4,
            beat_unit: 4,
        }
    }
}

impl SequencerTiming {
    /// Create new timing with specified tempo
    pub fn with_tempo(tempo: f64) -> Self {
        Self {
            tempo,
            ..Default::default()
        }
    }

    /// Get ticks per bar
    pub fn ticks_per_bar(&self) -> u64 {
        self.ppqn as u64 * self.beats_per_bar as u64
    }

    /// Get ticks per beat
    pub fn ticks_per_beat(&self) -> u64 {
        self.ppqn as u64
    }

    /// Convert ticks to microseconds at current tempo
    pub fn ticks_to_micros(&self, ticks: u64) -> u64 {
        let micros_per_beat = 60_000_000.0 / self.tempo;
        let micros_per_tick = micros_per_beat / self.ppqn as f64;
        (ticks as f64 * micros_per_tick) as u64
    }

    /// Convert microseconds to ticks at current tempo
    pub fn micros_to_ticks(&self, micros: u64) -> u64 {
        let micros_per_beat = 60_000_000.0 / self.tempo;
        let ticks_per_micro = self.ppqn as f64 / micros_per_beat;
        (micros as f64 * ticks_per_micro) as u64
    }

    /// Get current bar number (0-indexed)
    pub fn current_bar(&self) -> u64 {
        self.position_ticks / self.ticks_per_bar()
    }

    /// Get current beat within bar (0-indexed)
    pub fn current_beat(&self) -> u64 {
        (self.position_ticks % self.ticks_per_bar()) / self.ticks_per_beat()
    }

    /// Get current tick within beat
    pub fn current_tick(&self) -> u64 {
        self.position_ticks % self.ticks_per_beat()
    }

    /// Advance position by specified ticks
    pub fn advance(&mut self, ticks: u64) {
        self.position_ticks += ticks;
    }

    /// Reset position to beginning
    pub fn reset(&mut self) {
        self.position_ticks = 0;
    }

    /// Get ticks until next bar boundary
    pub fn ticks_to_next_bar(&self) -> u64 {
        let ticks_per_bar = self.ticks_per_bar();
        let into_bar = self.position_ticks % ticks_per_bar;
        if into_bar == 0 {
            0
        } else {
            ticks_per_bar - into_bar
        }
    }

    /// Get ticks until next beat boundary
    pub fn ticks_to_next_beat(&self) -> u64 {
        let ticks_per_beat = self.ticks_per_beat();
        let into_beat = self.position_ticks % ticks_per_beat;
        if into_beat == 0 {
            0
        } else {
            ticks_per_beat - into_beat
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_defaults() {
        let timing = SequencerTiming::default();
        assert_eq!(timing.tempo, 120.0);
        assert_eq!(timing.ppqn, 24);
        assert_eq!(timing.beats_per_bar, 4);
        assert_eq!(timing.ticks_per_bar(), 96);
        assert_eq!(timing.ticks_per_beat(), 24);
    }

    #[test]
    fn test_timing_position() {
        let mut timing = SequencerTiming::default();
        timing.position_ticks = 100; // Past one bar (96 ticks)

        assert_eq!(timing.current_bar(), 1);
        assert_eq!(timing.current_beat(), 0);
        assert_eq!(timing.current_tick(), 4);
    }

    #[test]
    fn test_ticks_to_micros() {
        let timing = SequencerTiming::with_tempo(120.0);
        // At 120 BPM, one beat = 500ms = 500,000 micros
        // One tick = 500,000 / 24 ≈ 20833 micros
        let micros = timing.ticks_to_micros(24);
        assert!((micros as i64 - 500_000).abs() < 100);
    }

    #[test]
    fn test_ticks_to_next_bar() {
        let mut timing = SequencerTiming::default();

        // At start
        assert_eq!(timing.ticks_to_next_bar(), 0);

        // After 10 ticks
        timing.position_ticks = 10;
        assert_eq!(timing.ticks_to_next_bar(), 86);

        // After 96 ticks (one bar)
        timing.position_ticks = 96;
        assert_eq!(timing.ticks_to_next_bar(), 0);
    }

    #[test]
    fn test_advance_and_reset() {
        let mut timing = SequencerTiming::default();
        timing.advance(50);
        assert_eq!(timing.position_ticks, 50);

        timing.reset();
        assert_eq!(timing.position_ticks, 0);
    }
}
