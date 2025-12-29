// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Clip freezing for capturing generator output.
//!
//! Converts real-time generator output to static clips
//! that can be saved and edited.

use crate::generators::MidiEvent;

/// Options for freezing
#[derive(Debug, Clone)]
pub struct FreezeOptions {
    /// Length to freeze in ticks
    pub length_ticks: u64,
    /// Number of bars to freeze
    pub length_bars: Option<u32>,
    /// Include note velocities
    pub include_velocity: bool,
    /// Quantize output
    pub quantize_grid: Option<u32>,
    /// Merge overlapping notes
    pub merge_notes: bool,
    /// Remove very short notes (< threshold ticks)
    pub min_note_length: u32,
    /// PPQN for bar calculations
    pub ppqn: u32,
    /// Beats per bar
    pub beats_per_bar: u32,
}

impl Default for FreezeOptions {
    fn default() -> Self {
        Self {
            length_ticks: 0,
            length_bars: Some(4),
            include_velocity: true,
            quantize_grid: None,
            merge_notes: false,
            min_note_length: 1,
            ppqn: 24,
            beats_per_bar: 4,
        }
    }
}

impl FreezeOptions {
    /// Create with specific bar length
    pub fn bars(bars: u32, ppqn: u32, beats_per_bar: u32) -> Self {
        Self {
            length_bars: Some(bars),
            length_ticks: bars as u64 * beats_per_bar as u64 * ppqn as u64,
            ppqn,
            beats_per_bar,
            ..Default::default()
        }
    }

    /// Create with specific tick length
    pub fn ticks(ticks: u64) -> Self {
        Self {
            length_ticks: ticks,
            length_bars: None,
            ..Default::default()
        }
    }

    /// Calculate total ticks
    pub fn total_ticks(&self) -> u64 {
        if let Some(bars) = self.length_bars {
            bars as u64 * self.beats_per_bar as u64 * self.ppqn as u64
        } else {
            self.length_ticks
        }
    }
}

/// A frozen note
#[derive(Debug, Clone, PartialEq)]
pub struct FrozenNote {
    /// MIDI channel
    pub channel: u8,
    /// Note number
    pub note: u8,
    /// Velocity
    pub velocity: u8,
    /// Start tick
    pub start_tick: u64,
    /// Duration in ticks
    pub duration: u64,
}

impl FrozenNote {
    /// Create from MIDI events
    pub fn from_events(note_on: &MidiEvent, duration: u64) -> Self {
        Self {
            channel: note_on.channel,
            note: note_on.note,
            velocity: note_on.velocity,
            start_tick: note_on.start_tick,
            duration,
        }
    }

    /// End tick
    pub fn end_tick(&self) -> u64 {
        self.start_tick + self.duration
    }
}

/// Freezer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezerState {
    /// Idle, not freezing
    Idle,
    /// Actively capturing
    Capturing,
    /// Finished capture
    Complete,
}

/// Active note during freezing
#[derive(Debug, Clone)]
struct ActiveNote {
    event: MidiEvent,
    start_tick: u64,
}

/// Clip freezer for capturing generator output
pub struct ClipFreezer {
    /// Current state
    state: FreezerState,
    /// Options
    options: FreezeOptions,
    /// Frozen notes
    notes: Vec<FrozenNote>,
    /// Active notes (note on received, waiting for note off)
    active_notes: Vec<ActiveNote>,
    /// Current position in ticks
    position: u64,
    /// PPQN
    ppqn: u32,
}

impl ClipFreezer {
    /// Create a new clip freezer
    pub fn new(ppqn: u32) -> Self {
        Self {
            state: FreezerState::Idle,
            options: FreezeOptions::default(),
            notes: Vec::new(),
            active_notes: Vec::new(),
            position: 0,
            ppqn,
        }
    }

    /// Get current state
    pub fn state(&self) -> FreezerState {
        self.state
    }

    /// Check if freezing
    pub fn is_freezing(&self) -> bool {
        self.state == FreezerState::Capturing
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.state == FreezerState::Complete
    }

    /// Set freeze options
    pub fn set_options(&mut self, options: FreezeOptions) {
        self.options = options;
    }

    /// Get options
    pub fn options(&self) -> &FreezeOptions {
        &self.options
    }

    /// Start freezing
    pub fn start(&mut self, options: FreezeOptions) {
        self.options = options;
        self.notes.clear();
        self.active_notes.clear();
        self.position = 0;
        self.state = FreezerState::Capturing;
    }

    /// Stop freezing
    pub fn stop(&mut self) {
        // Close any active notes
        for active in &self.active_notes {
            let duration = self.position.saturating_sub(active.start_tick);
            if duration >= self.options.min_note_length as u64 {
                self.notes.push(FrozenNote {
                    channel: active.event.channel,
                    note: active.event.note,
                    velocity: if self.options.include_velocity {
                        active.event.velocity
                    } else {
                        100
                    },
                    start_tick: active.start_tick,
                    duration,
                });
            }
        }
        self.active_notes.clear();
        self.state = FreezerState::Complete;
    }

    /// Cancel freezing
    pub fn cancel(&mut self) {
        self.notes.clear();
        self.active_notes.clear();
        self.state = FreezerState::Idle;
    }

    /// Reset to idle state
    pub fn reset(&mut self) {
        self.notes.clear();
        self.active_notes.clear();
        self.position = 0;
        self.state = FreezerState::Idle;
    }

    /// Process MIDI events from generator
    pub fn process_events(&mut self, events: &[MidiEvent]) {
        if self.state != FreezerState::Capturing {
            return;
        }

        for event in events {
            // Note on: velocity > 0
            if event.velocity > 0 {
                self.active_notes.push(ActiveNote {
                    event: event.clone(),
                    start_tick: event.start_tick,
                });
            } else {
                // Note off: velocity == 0
                // Find matching note on
                if let Some(idx) = self.active_notes.iter().position(|a| {
                    a.event.channel == event.channel && a.event.note == event.note
                }) {
                    let active = self.active_notes.remove(idx);
                    let duration = event.start_tick.saturating_sub(active.start_tick);

                    if duration >= self.options.min_note_length as u64 {
                        let mut frozen = FrozenNote {
                            channel: active.event.channel,
                            note: active.event.note,
                            velocity: if self.options.include_velocity {
                                active.event.velocity
                            } else {
                                100
                            },
                            start_tick: active.start_tick,
                            duration,
                        };

                        // Apply quantization
                        if let Some(grid) = self.options.quantize_grid {
                            frozen.start_tick = self.quantize(frozen.start_tick, grid);
                            let end = self.quantize(frozen.end_tick(), grid);
                            frozen.duration = end.saturating_sub(frozen.start_tick);
                        }

                        self.notes.push(frozen);
                    }
                }
            }
        }
    }

    /// Quantize a tick value to grid
    fn quantize(&self, tick: u64, grid: u32) -> u64 {
        let grid = grid as u64;
        ((tick + grid / 2) / grid) * grid
    }

    /// Update position
    pub fn tick(&mut self, ticks: u64) {
        if self.state != FreezerState::Capturing {
            return;
        }

        self.position += ticks;

        // Check if we've reached the target length
        let target = self.options.total_ticks();
        if target > 0 && self.position >= target {
            self.stop();
        }
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        let target = self.options.total_ticks();
        if target == 0 {
            0.0
        } else {
            (self.position as f64 / target as f64).min(1.0)
        }
    }

    /// Get frozen notes
    pub fn notes(&self) -> &[FrozenNote] {
        &self.notes
    }

    /// Take frozen notes
    pub fn take_notes(&mut self) -> Vec<FrozenNote> {
        let notes = std::mem::take(&mut self.notes);
        self.state = FreezerState::Idle;
        notes
    }

    /// Get number of frozen notes
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Sort notes by start time
    pub fn sort_notes(&mut self) {
        self.notes.sort_by_key(|n| n.start_tick);
    }

    /// Merge overlapping notes of same pitch
    pub fn merge_overlapping(&mut self) {
        if !self.options.merge_notes {
            return;
        }

        self.sort_notes();

        let mut i = 0;
        while i < self.notes.len() {
            let mut j = i + 1;
            while j < self.notes.len() {
                if self.notes[i].channel == self.notes[j].channel &&
                   self.notes[i].note == self.notes[j].note &&
                   self.notes[j].start_tick <= self.notes[i].end_tick()
                {
                    // Merge: extend first note to cover second
                    let new_end = self.notes[i].end_tick().max(self.notes[j].end_tick());
                    self.notes[i].duration = new_end - self.notes[i].start_tick;
                    self.notes.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freezer_creation() {
        let freezer = ClipFreezer::new(24);
        assert_eq!(freezer.state(), FreezerState::Idle);
        assert!(!freezer.is_freezing());
    }

    #[test]
    fn test_freezer_start_stop() {
        let mut freezer = ClipFreezer::new(24);
        let options = FreezeOptions::bars(4, 24, 4);

        freezer.start(options);
        assert_eq!(freezer.state(), FreezerState::Capturing);
        assert!(freezer.is_freezing());

        freezer.stop();
        assert_eq!(freezer.state(), FreezerState::Complete);
        assert!(freezer.is_complete());
    }

    #[test]
    fn test_freeze_events() {
        let mut freezer = ClipFreezer::new(24);
        let options = FreezeOptions::ticks(96);

        freezer.start(options);

        let events = vec![
            MidiEvent {
                start_tick: 0,
                channel: 0,
                note: 60,
                velocity: 100,
                duration_ticks: 24,
            },
        ];

        // Note on
        freezer.process_events(&events);

        // Note off
        let off_events = vec![
            MidiEvent {
                start_tick: 24,
                channel: 0,
                note: 60,
                velocity: 0,
                duration_ticks: 0,
            },
        ];
        freezer.process_events(&off_events);

        assert_eq!(freezer.note_count(), 1);
        let note = &freezer.notes()[0];
        assert_eq!(note.note, 60);
        assert_eq!(note.velocity, 100);
        assert_eq!(note.start_tick, 0);
        assert_eq!(note.duration, 24);
    }

    #[test]
    fn test_freeze_auto_complete() {
        let mut freezer = ClipFreezer::new(24);
        let options = FreezeOptions::ticks(48);

        freezer.start(options);
        assert!(freezer.is_freezing());

        freezer.tick(24);
        assert!(freezer.is_freezing());
        assert!((freezer.progress() - 0.5).abs() < 0.01);

        freezer.tick(24);
        assert!(freezer.is_complete());
        assert!((freezer.progress() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_freeze_options_bars() {
        let options = FreezeOptions::bars(2, 24, 4);
        assert_eq!(options.total_ticks(), 2 * 4 * 24);
    }

    #[test]
    fn test_freeze_quantization() {
        let mut freezer = ClipFreezer::new(24);
        let mut options = FreezeOptions::ticks(96);
        options.quantize_grid = Some(12); // 8th notes

        freezer.start(options);

        // Use start_tick 7 which will round up to 12 (7+6=13, 13/12=1, 1*12=12)
        let events = vec![
            MidiEvent {
                start_tick: 7, // Off grid, will quantize to 12
                channel: 0,
                note: 60,
                velocity: 100,
                duration_ticks: 20,
            },
        ];
        freezer.process_events(&events);

        let off_events = vec![
            MidiEvent {
                start_tick: 30, // Off grid, will quantize to 36
                channel: 0,
                note: 60,
                velocity: 0,
                duration_ticks: 0,
            },
        ];
        freezer.process_events(&off_events);

        let note = &freezer.notes()[0];
        assert_eq!(note.start_tick, 12); // Quantized: (7+6)/12*12 = 12
        assert_eq!(note.duration, 24); // End at 36, duration = 36 - 12
    }

    #[test]
    fn test_frozen_note() {
        let event = MidiEvent {
            start_tick: 10,
            channel: 0,
            note: 60,
            velocity: 100,
            duration_ticks: 24,
        };

        let frozen = FrozenNote::from_events(&event, 24);
        assert_eq!(frozen.note, 60);
        assert_eq!(frozen.start_tick, 10);
        assert_eq!(frozen.duration, 24);
        assert_eq!(frozen.end_tick(), 34);
    }

    #[test]
    fn test_merge_overlapping() {
        let mut freezer = ClipFreezer::new(24);
        let mut options = FreezeOptions::ticks(96);
        options.merge_notes = true;
        freezer.set_options(options);

        freezer.notes = vec![
            FrozenNote {
                channel: 0,
                note: 60,
                velocity: 100,
                start_tick: 0,
                duration: 20,
            },
            FrozenNote {
                channel: 0,
                note: 60,
                velocity: 100,
                start_tick: 15, // Overlaps with first
                duration: 20,
            },
        ];

        freezer.merge_overlapping();

        assert_eq!(freezer.note_count(), 1);
        assert_eq!(freezer.notes()[0].start_tick, 0);
        assert_eq!(freezer.notes()[0].duration, 35); // Merged duration
    }

    #[test]
    fn test_cancel_freeze() {
        let mut freezer = ClipFreezer::new(24);
        freezer.start(FreezeOptions::ticks(96));

        freezer.tick(24);
        freezer.cancel();

        assert_eq!(freezer.state(), FreezerState::Idle);
        assert!(freezer.notes().is_empty());
    }

    #[test]
    fn test_min_note_length() {
        let mut freezer = ClipFreezer::new(24);
        let mut options = FreezeOptions::ticks(96);
        options.min_note_length = 10;

        freezer.start(options);

        // Short note (should be filtered)
        let events = vec![
            MidiEvent {
                start_tick: 0,
                channel: 0,
                note: 60,
                velocity: 100,
                duration_ticks: 5,
            },
        ];
        freezer.process_events(&events);

        let off_events = vec![
            MidiEvent {
                start_tick: 5,
                channel: 0,
                note: 60,
                velocity: 0,
                duration_ticks: 0,
            },
        ];
        freezer.process_events(&off_events);

        assert_eq!(freezer.note_count(), 0);
    }
}
