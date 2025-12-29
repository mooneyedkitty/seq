// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI recording to clips.
//!
//! Provides real-time MIDI input capture with quantization,
//! overdub, and punch in/out support.

use std::collections::HashMap;

/// Recording mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordMode {
    /// Replace existing notes
    Replace,
    /// Add to existing notes
    Overdub,
    /// Punch in/out at specified points
    Punch,
}

impl Default for RecordMode {
    fn default() -> Self {
        RecordMode::Replace
    }
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    /// Not recording
    Idle,
    /// Armed and waiting for input or start
    Armed,
    /// Actively recording
    Recording,
    /// Count-in before recording
    CountIn,
    /// Paused
    Paused,
}

impl Default for RecordingState {
    fn default() -> Self {
        RecordingState::Idle
    }
}

/// Quantization settings for recording
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizeSettings {
    /// Quantize grid in ticks (0 = no quantization)
    pub grid: u32,
    /// Quantize strength (0.0 = none, 1.0 = full)
    pub strength: f64,
    /// Quantize note start times
    pub start: bool,
    /// Quantize note end times
    pub end: bool,
}

impl Default for QuantizeSettings {
    fn default() -> Self {
        Self {
            grid: 0,
            start: true,
            end: false,
            strength: 1.0,
        }
    }
}

impl QuantizeSettings {
    /// Create with quarter note grid
    pub fn quarter(ppqn: u32) -> Self {
        Self {
            grid: ppqn,
            ..Default::default()
        }
    }

    /// Create with eighth note grid
    pub fn eighth(ppqn: u32) -> Self {
        Self {
            grid: ppqn / 2,
            ..Default::default()
        }
    }

    /// Create with sixteenth note grid
    pub fn sixteenth(ppqn: u32) -> Self {
        Self {
            grid: ppqn / 4,
            ..Default::default()
        }
    }

    /// Quantize a tick value
    pub fn quantize(&self, tick: u64) -> u64 {
        if self.grid == 0 || self.strength == 0.0 {
            return tick;
        }

        let grid = self.grid as u64;
        let quantized = ((tick + grid / 2) / grid) * grid;

        if self.strength >= 1.0 {
            quantized
        } else {
            let diff = quantized as f64 - tick as f64;
            (tick as f64 + diff * self.strength) as u64
        }
    }
}

/// A recorded note
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedNote {
    /// MIDI channel (0-15)
    pub channel: u8,
    /// Note number (0-127)
    pub note: u8,
    /// Velocity (1-127)
    pub velocity: u8,
    /// Start tick (relative to recording start)
    pub start_tick: u64,
    /// Duration in ticks
    pub duration: u64,
}

impl RecordedNote {
    /// Create a new recorded note
    pub fn new(channel: u8, note: u8, velocity: u8, start_tick: u64, duration: u64) -> Self {
        Self {
            channel,
            note,
            velocity,
            start_tick,
            duration,
        }
    }

    /// End tick
    pub fn end_tick(&self) -> u64 {
        self.start_tick + self.duration
    }
}

/// Active note being recorded (note on received, waiting for note off)
#[derive(Debug, Clone)]
struct ActiveNote {
    channel: u8,
    note: u8,
    velocity: u8,
    start_tick: u64,
}

/// Punch region for punch in/out recording
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PunchRegion {
    /// Punch in tick
    pub in_tick: u64,
    /// Punch out tick
    pub out_tick: u64,
}

impl PunchRegion {
    /// Create a new punch region
    pub fn new(in_tick: u64, out_tick: u64) -> Self {
        Self {
            in_tick: in_tick.min(out_tick),
            out_tick: out_tick.max(in_tick),
        }
    }

    /// Check if a tick is within the punch region
    pub fn contains(&self, tick: u64) -> bool {
        tick >= self.in_tick && tick < self.out_tick
    }
}

/// MIDI recorder for capturing input to clips
pub struct MidiRecorder {
    /// Current state
    state: RecordingState,
    /// Recording mode
    mode: RecordMode,
    /// Recorded notes
    notes: Vec<RecordedNote>,
    /// Active notes (note on received, waiting for note off)
    active_notes: HashMap<(u8, u8), ActiveNote>, // (channel, note) -> ActiveNote
    /// Current position in ticks
    position: u64,
    /// Recording start position
    start_position: u64,
    /// Loop length in ticks (0 = no loop)
    loop_length: u64,
    /// Quantize settings
    quantize: QuantizeSettings,
    /// Punch region (for punch mode)
    punch_region: Option<PunchRegion>,
    /// Count-in bars
    count_in_bars: u8,
    /// Count-in remaining ticks
    count_in_remaining: u64,
    /// PPQN for timing
    ppqn: u32,
    /// Beats per bar
    beats_per_bar: u32,
    /// Metronome enabled during recording
    metronome: bool,
    /// Input channel filter (None = all channels)
    channel_filter: Option<u8>,
}

impl MidiRecorder {
    /// Create a new MIDI recorder
    pub fn new(ppqn: u32) -> Self {
        Self {
            state: RecordingState::Idle,
            mode: RecordMode::Replace,
            notes: Vec::new(),
            active_notes: HashMap::new(),
            position: 0,
            start_position: 0,
            loop_length: 0,
            quantize: QuantizeSettings::default(),
            punch_region: None,
            count_in_bars: 0,
            count_in_remaining: 0,
            ppqn,
            beats_per_bar: 4,
            metronome: true,
            channel_filter: None,
        }
    }

    /// Get current state
    pub fn state(&self) -> RecordingState {
        self.state
    }

    /// Get recording mode
    pub fn mode(&self) -> RecordMode {
        self.mode
    }

    /// Set recording mode
    pub fn set_mode(&mut self, mode: RecordMode) {
        self.mode = mode;
    }

    /// Get recorded notes
    pub fn notes(&self) -> &[RecordedNote] {
        &self.notes
    }

    /// Take recorded notes (clears internal buffer)
    pub fn take_notes(&mut self) -> Vec<RecordedNote> {
        std::mem::take(&mut self.notes)
    }

    /// Clear recorded notes
    pub fn clear(&mut self) {
        self.notes.clear();
        self.active_notes.clear();
    }

    /// Set quantize settings
    pub fn set_quantize(&mut self, settings: QuantizeSettings) {
        self.quantize = settings;
    }

    /// Get quantize settings
    pub fn quantize(&self) -> &QuantizeSettings {
        &self.quantize
    }

    /// Set loop length in ticks
    pub fn set_loop_length(&mut self, ticks: u64) {
        self.loop_length = ticks;
    }

    /// Get loop length
    pub fn loop_length(&self) -> u64 {
        self.loop_length
    }

    /// Set punch region
    pub fn set_punch_region(&mut self, region: Option<PunchRegion>) {
        self.punch_region = region;
    }

    /// Get punch region
    pub fn punch_region(&self) -> Option<PunchRegion> {
        self.punch_region
    }

    /// Set count-in bars
    pub fn set_count_in(&mut self, bars: u8) {
        self.count_in_bars = bars;
    }

    /// Get count-in bars
    pub fn count_in_bars(&self) -> u8 {
        self.count_in_bars
    }

    /// Set metronome enabled
    pub fn set_metronome(&mut self, enabled: bool) {
        self.metronome = enabled;
    }

    /// Get metronome enabled
    pub fn metronome(&self) -> bool {
        self.metronome
    }

    /// Set channel filter
    pub fn set_channel_filter(&mut self, channel: Option<u8>) {
        self.channel_filter = channel;
    }

    /// Get channel filter
    pub fn channel_filter(&self) -> Option<u8> {
        self.channel_filter
    }

    /// Arm recording
    pub fn arm(&mut self) {
        if self.state == RecordingState::Idle {
            self.state = RecordingState::Armed;
        }
    }

    /// Disarm recording
    pub fn disarm(&mut self) {
        if self.state == RecordingState::Armed {
            self.state = RecordingState::Idle;
        }
    }

    /// Start recording
    pub fn start(&mut self, position: u64) {
        if self.state == RecordingState::Idle || self.state == RecordingState::Armed {
            if self.mode == RecordMode::Replace {
                self.notes.clear();
            }

            self.start_position = position;
            self.position = position;
            self.active_notes.clear();

            if self.count_in_bars > 0 {
                let ticks_per_bar = self.ppqn as u64 * self.beats_per_bar as u64;
                self.count_in_remaining = self.count_in_bars as u64 * ticks_per_bar;
                self.state = RecordingState::CountIn;
            } else {
                self.state = RecordingState::Recording;
            }
        }
    }

    /// Stop recording
    pub fn stop(&mut self) {
        // Close any active notes
        for ((channel, note), active) in self.active_notes.drain() {
            let duration = self.position.saturating_sub(active.start_tick);
            if duration > 0 {
                self.notes.push(RecordedNote::new(
                    channel,
                    note,
                    active.velocity,
                    active.start_tick - self.start_position,
                    duration,
                ));
            }
        }

        self.state = RecordingState::Idle;
    }

    /// Pause recording
    pub fn pause(&mut self) {
        if self.state == RecordingState::Recording {
            self.state = RecordingState::Paused;
        }
    }

    /// Resume recording
    pub fn resume(&mut self) {
        if self.state == RecordingState::Paused {
            self.state = RecordingState::Recording;
        }
    }

    /// Update position (call each tick)
    pub fn tick(&mut self, ticks: u64) {
        if self.state == RecordingState::CountIn {
            if ticks >= self.count_in_remaining {
                self.count_in_remaining = 0;
                self.state = RecordingState::Recording;
                self.start_position = self.position + ticks;
            } else {
                self.count_in_remaining -= ticks;
            }
        }

        self.position += ticks;

        // Handle loop
        if self.loop_length > 0 && self.state == RecordingState::Recording {
            let relative_pos = self.position - self.start_position;
            if relative_pos >= self.loop_length {
                // Wrap position
                self.position = self.start_position + (relative_pos % self.loop_length);
            }
        }
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get recording duration
    pub fn duration(&self) -> u64 {
        if self.position > self.start_position {
            self.position - self.start_position
        } else {
            0
        }
    }

    /// Check if should record at current position (for punch mode)
    fn should_record(&self) -> bool {
        if self.state != RecordingState::Recording {
            return false;
        }

        if self.mode == RecordMode::Punch {
            if let Some(region) = self.punch_region {
                return region.contains(self.position);
            }
            return false;
        }

        true
    }

    /// Record note on
    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8) {
        // Check channel filter
        if let Some(filter) = self.channel_filter {
            if channel != filter {
                return;
            }
        }

        if !self.should_record() {
            return;
        }

        // Store as active note
        let start_tick = if self.quantize.start {
            self.quantize.quantize(self.position)
        } else {
            self.position
        };

        self.active_notes.insert(
            (channel, note),
            ActiveNote {
                channel,
                note,
                velocity,
                start_tick,
            },
        );
    }

    /// Record note off
    pub fn note_off(&mut self, channel: u8, note: u8) {
        // Check channel filter
        if let Some(filter) = self.channel_filter {
            if channel != filter {
                return;
            }
        }

        // Find and complete active note
        if let Some(active) = self.active_notes.remove(&(channel, note)) {
            let end_tick = if self.quantize.end {
                self.quantize.quantize(self.position)
            } else {
                self.position
            };

            let duration = end_tick.saturating_sub(active.start_tick);
            if duration > 0 {
                self.notes.push(RecordedNote::new(
                    active.channel,
                    active.note,
                    active.velocity,
                    active.start_tick.saturating_sub(self.start_position),
                    duration,
                ));
            }
        }
    }

    /// Apply quantization to all recorded notes
    pub fn quantize_all(&mut self) {
        for note in &mut self.notes {
            if self.quantize.start {
                note.start_tick = self.quantize.quantize(note.start_tick);
            }
            if self.quantize.end {
                let end = self.quantize.quantize(note.end_tick());
                note.duration = end.saturating_sub(note.start_tick);
            }
        }
    }

    /// Sort notes by start time
    pub fn sort_notes(&mut self) {
        self.notes.sort_by_key(|n| n.start_tick);
    }

    /// Remove duplicate notes
    pub fn remove_duplicates(&mut self) {
        self.notes.dedup_by(|a, b| {
            a.channel == b.channel &&
            a.note == b.note &&
            a.start_tick == b.start_tick
        });
    }

    /// Get number of recorded notes
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Check if recording
    pub fn is_recording(&self) -> bool {
        self.state == RecordingState::Recording
    }

    /// Check if armed
    pub fn is_armed(&self) -> bool {
        self.state == RecordingState::Armed
    }

    /// Get count-in remaining (ticks)
    pub fn count_in_remaining(&self) -> u64 {
        self.count_in_remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let recorder = MidiRecorder::new(24);
        assert_eq!(recorder.state(), RecordingState::Idle);
        assert_eq!(recorder.mode(), RecordMode::Replace);
        assert!(recorder.notes().is_empty());
    }

    #[test]
    fn test_recorder_arm_disarm() {
        let mut recorder = MidiRecorder::new(24);

        recorder.arm();
        assert_eq!(recorder.state(), RecordingState::Armed);

        recorder.disarm();
        assert_eq!(recorder.state(), RecordingState::Idle);
    }

    #[test]
    fn test_recorder_start_stop() {
        let mut recorder = MidiRecorder::new(24);

        recorder.start(0);
        assert_eq!(recorder.state(), RecordingState::Recording);

        recorder.stop();
        assert_eq!(recorder.state(), RecordingState::Idle);
    }

    #[test]
    fn test_record_note() {
        let mut recorder = MidiRecorder::new(24);
        recorder.start(0);

        recorder.note_on(0, 60, 100);
        recorder.tick(24); // One beat
        recorder.note_off(0, 60);

        recorder.stop();

        assert_eq!(recorder.note_count(), 1);
        let note = &recorder.notes()[0];
        assert_eq!(note.channel, 0);
        assert_eq!(note.note, 60);
        assert_eq!(note.velocity, 100);
        assert_eq!(note.start_tick, 0);
        assert_eq!(note.duration, 24);
    }

    #[test]
    fn test_record_multiple_notes() {
        let mut recorder = MidiRecorder::new(24);
        recorder.start(0);

        recorder.note_on(0, 60, 100);
        recorder.tick(12);
        recorder.note_on(0, 64, 90);
        recorder.tick(12);
        recorder.note_off(0, 60);
        recorder.tick(12);
        recorder.note_off(0, 64);

        recorder.stop();

        assert_eq!(recorder.note_count(), 2);
    }

    #[test]
    fn test_quantize_settings() {
        let ppqn = 24;
        let quantize = QuantizeSettings::sixteenth(ppqn);

        assert_eq!(quantize.grid, 6); // 24/4 = 6 ticks

        // Test quantization
        assert_eq!(quantize.quantize(0), 0);
        assert_eq!(quantize.quantize(2), 0); // Rounds down
        assert_eq!(quantize.quantize(4), 6); // Rounds up
        assert_eq!(quantize.quantize(6), 6);
        assert_eq!(quantize.quantize(8), 6);
        assert_eq!(quantize.quantize(10), 12);
    }

    #[test]
    fn test_quantize_strength() {
        let quantize = QuantizeSettings {
            grid: 24,
            strength: 0.5,
            start: true,
            end: false,
        };

        // Tick 12 with 50% strength: rounds to 24, then moves halfway back
        // quantized = 24 (since 12+12=24 rounds up), diff = 24-12 = 12
        // result = 12 + 12*0.5 = 18
        let result = quantize.quantize(12);
        assert_eq!(result, 18);

        // Tick 6 with 50% strength
        // quantized = 0 (since 6+12=18 rounds down to 0), diff = 0-6 = -6
        // result = 6 + (-6)*0.5 = 3
        let result = quantize.quantize(6);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_overdub_mode() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_mode(RecordMode::Overdub);

        // First recording pass
        recorder.start(0);
        recorder.note_on(0, 60, 100);
        recorder.tick(24);
        recorder.note_off(0, 60);
        recorder.stop();

        assert_eq!(recorder.note_count(), 1);

        // Second recording pass (overdub)
        recorder.start(0);
        recorder.note_on(0, 64, 90);
        recorder.tick(24);
        recorder.note_off(0, 64);
        recorder.stop();

        assert_eq!(recorder.note_count(), 2);
    }

    #[test]
    fn test_replace_mode() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_mode(RecordMode::Replace);

        // First recording pass
        recorder.start(0);
        recorder.note_on(0, 60, 100);
        recorder.tick(24);
        recorder.note_off(0, 60);
        recorder.stop();

        assert_eq!(recorder.note_count(), 1);

        // Second recording pass (replace)
        recorder.start(0);
        recorder.note_on(0, 64, 90);
        recorder.tick(24);
        recorder.note_off(0, 64);
        recorder.stop();

        // Should only have one note from second pass
        assert_eq!(recorder.note_count(), 1);
        assert_eq!(recorder.notes()[0].note, 64);
    }

    #[test]
    fn test_punch_mode() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_mode(RecordMode::Punch);
        recorder.set_punch_region(Some(PunchRegion::new(48, 96)));

        recorder.start(0);

        // Before punch in - should not record
        recorder.note_on(0, 60, 100);
        recorder.tick(24);
        recorder.note_off(0, 60);
        recorder.tick(24);

        // Inside punch region - should record
        recorder.note_on(0, 64, 90);
        recorder.tick(24);
        recorder.note_off(0, 64);
        recorder.tick(24);

        // After punch out - should not record
        recorder.note_on(0, 67, 80);
        recorder.tick(24);
        recorder.note_off(0, 67);

        recorder.stop();

        assert_eq!(recorder.note_count(), 1);
        assert_eq!(recorder.notes()[0].note, 64);
    }

    #[test]
    fn test_channel_filter() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_channel_filter(Some(0));

        recorder.start(0);

        recorder.note_on(0, 60, 100); // Channel 0 - should record
        recorder.note_on(1, 64, 90);  // Channel 1 - should not record
        recorder.tick(24);
        recorder.note_off(0, 60);
        recorder.note_off(1, 64);

        recorder.stop();

        assert_eq!(recorder.note_count(), 1);
        assert_eq!(recorder.notes()[0].note, 60);
    }

    #[test]
    fn test_count_in() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_count_in(1); // 1 bar count-in

        recorder.start(0);
        assert_eq!(recorder.state(), RecordingState::CountIn);

        // Tick through count-in (4 beats = 96 ticks)
        recorder.tick(48);
        assert_eq!(recorder.state(), RecordingState::CountIn);

        recorder.tick(48);
        assert_eq!(recorder.state(), RecordingState::Recording);
    }

    #[test]
    fn test_loop_recording() {
        let mut recorder = MidiRecorder::new(24);
        recorder.set_loop_length(96); // 4 beats

        recorder.start(0);
        recorder.tick(100); // Past loop point

        // Position should wrap: 100 % 96 = 4
        let expected_pos = 100 % 96;
        assert_eq!(recorder.position() - recorder.start_position, expected_pos);
    }

    #[test]
    fn test_punch_region() {
        let region = PunchRegion::new(100, 200);

        assert!(!region.contains(50));
        assert!(region.contains(100));
        assert!(region.contains(150));
        assert!(!region.contains(200));
        assert!(!region.contains(250));
    }

    #[test]
    fn test_recorded_note() {
        let note = RecordedNote::new(0, 60, 100, 0, 24);
        assert_eq!(note.end_tick(), 24);
    }
}
