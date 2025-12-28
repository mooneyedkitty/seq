// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Event scheduler with microsecond-precision timing.
//!
//! Provides a priority queue for timed MIDI events with lookahead
//! buffering and tempo change handling.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use super::SequencerTiming;

/// Type of MIDI message in a scheduled event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiMessageType {
    /// Note on message
    NoteOn,
    /// Note off message
    NoteOff,
    /// Control change
    ControlChange,
    /// Program change
    ProgramChange,
    /// Pitch bend
    PitchBend,
}

/// A scheduled MIDI event
#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    /// Time in microseconds from sequence start
    pub time_micros: u64,
    /// Time in ticks from sequence start
    pub time_ticks: u64,
    /// MIDI channel (0-15)
    pub channel: u8,
    /// Message type
    pub message_type: MidiMessageType,
    /// First data byte (note number, CC number, etc.)
    pub data1: u8,
    /// Second data byte (velocity, CC value, etc.)
    pub data2: u8,
    /// Source track index (for tracking origin)
    pub track_index: Option<usize>,
}

impl ScheduledEvent {
    /// Create a note on event
    pub fn note_on(time_ticks: u64, channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            time_micros: 0, // Will be calculated by scheduler
            time_ticks,
            channel,
            message_type: MidiMessageType::NoteOn,
            data1: note,
            data2: velocity,
            track_index: None,
        }
    }

    /// Create a note off event
    pub fn note_off(time_ticks: u64, channel: u8, note: u8) -> Self {
        Self {
            time_micros: 0,
            time_ticks,
            channel,
            message_type: MidiMessageType::NoteOff,
            data1: note,
            data2: 0,
            track_index: None,
        }
    }

    /// Create a control change event
    pub fn control_change(time_ticks: u64, channel: u8, cc: u8, value: u8) -> Self {
        Self {
            time_micros: 0,
            time_ticks,
            channel,
            message_type: MidiMessageType::ControlChange,
            data1: cc,
            data2: value,
            track_index: None,
        }
    }

    /// Create a program change event
    pub fn program_change(time_ticks: u64, channel: u8, program: u8) -> Self {
        Self {
            time_micros: 0,
            time_ticks,
            channel,
            message_type: MidiMessageType::ProgramChange,
            data1: program,
            data2: 0,
            track_index: None,
        }
    }

    /// Set the track index for this event
    pub fn with_track(mut self, track_index: usize) -> Self {
        self.track_index = Some(track_index);
        self
    }

    /// Convert to MIDI bytes
    pub fn to_midi_bytes(&self) -> Vec<u8> {
        match self.message_type {
            MidiMessageType::NoteOn => vec![0x90 | self.channel, self.data1, self.data2],
            MidiMessageType::NoteOff => vec![0x80 | self.channel, self.data1, self.data2],
            MidiMessageType::ControlChange => vec![0xB0 | self.channel, self.data1, self.data2],
            MidiMessageType::ProgramChange => vec![0xC0 | self.channel, self.data1],
            MidiMessageType::PitchBend => {
                // Pitch bend uses two 7-bit values
                vec![0xE0 | self.channel, self.data1, self.data2]
            }
        }
    }
}

// For BinaryHeap - we want minimum time first
impl Eq for ScheduledEvent {}

impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time_micros == other.time_micros
    }
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other.time_micros.cmp(&self.time_micros)
    }
}

impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Configuration for the scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Lookahead time in milliseconds
    pub lookahead_ms: u32,
    /// Buffer size for events
    pub buffer_size: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            lookahead_ms: 50,
            buffer_size: 1024,
        }
    }
}

/// Event scheduler with priority queue
pub struct Scheduler {
    /// Priority queue of scheduled events
    queue: BinaryHeap<ScheduledEvent>,
    /// Current timing information
    timing: SequencerTiming,
    /// Configuration
    config: SchedulerConfig,
    /// Start time of playback
    start_time: Option<Instant>,
    /// Current playback position in microseconds
    position_micros: u64,
    /// Whether playback is active
    playing: bool,
    /// Accumulated timing error for drift correction
    timing_error_micros: i64,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::with_capacity(1024),
            timing: SequencerTiming::default(),
            config: SchedulerConfig::default(),
            start_time: None,
            position_micros: 0,
            playing: false,
            timing_error_micros: 0,
        }
    }

    /// Create scheduler with custom config
    pub fn with_config(config: SchedulerConfig) -> Self {
        Self {
            queue: BinaryHeap::with_capacity(config.buffer_size),
            config,
            ..Self::new()
        }
    }

    /// Set the tempo
    pub fn set_tempo(&mut self, tempo: f64) {
        // Record current position before tempo change
        if self.playing {
            self.update_position();
        }
        self.timing.tempo = tempo.clamp(20.0, 300.0);
        // Recalculate microsecond times for all queued events
        self.recalculate_event_times();
    }

    /// Get current tempo
    pub fn tempo(&self) -> f64 {
        self.timing.tempo
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, beats_per_bar: u8, beat_unit: u8) {
        self.timing.beats_per_bar = beats_per_bar;
        self.timing.beat_unit = beat_unit;
    }

    /// Get current timing information
    pub fn timing(&self) -> &SequencerTiming {
        &self.timing
    }

    /// Get mutable timing information
    pub fn timing_mut(&mut self) -> &mut SequencerTiming {
        &mut self.timing
    }

    /// Schedule an event
    pub fn schedule(&mut self, mut event: ScheduledEvent) {
        // Calculate microsecond time from tick time
        event.time_micros = self.timing.ticks_to_micros(event.time_ticks);
        self.queue.push(event);
    }

    /// Schedule multiple events
    pub fn schedule_all(&mut self, events: impl IntoIterator<Item = ScheduledEvent>) {
        for event in events {
            self.schedule(event);
        }
    }

    /// Start playback
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.playing = true;
        self.timing_error_micros = 0;
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.playing = false;
        self.start_time = None;
        self.position_micros = 0;
        self.timing.reset();
        self.timing_error_micros = 0;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.playing {
            self.update_position();
            self.playing = false;
            self.start_time = None;
        }
    }

    /// Resume playback
    pub fn resume(&mut self) {
        if !self.playing {
            self.start_time = Some(Instant::now());
            self.playing = true;
        }
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Get current position in ticks
    pub fn position_ticks(&self) -> u64 {
        self.timing.position_ticks
    }

    /// Get current position in microseconds
    pub fn position_micros(&self) -> u64 {
        self.position_micros
    }

    /// Clear all scheduled events
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Get number of queued events
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Update current position based on elapsed time
    fn update_position(&mut self) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let elapsed_micros = elapsed.as_micros() as u64;
            self.position_micros = elapsed_micros;
            self.timing.position_ticks = self.timing.micros_to_ticks(elapsed_micros);
        }
    }

    /// Get events that should be played now
    pub fn poll(&mut self) -> Vec<ScheduledEvent> {
        if !self.playing {
            return Vec::new();
        }

        self.update_position();

        let lookahead_micros = self.config.lookahead_ms as u64 * 1000;
        let target_time = self.position_micros + lookahead_micros;

        let mut events = Vec::new();

        while let Some(event) = self.queue.peek() {
            if event.time_micros <= target_time {
                events.push(self.queue.pop().unwrap());
            } else {
                break;
            }
        }

        events
    }

    /// Get events due within the specified time window
    pub fn poll_window(&mut self, window_micros: u64) -> Vec<ScheduledEvent> {
        if !self.playing {
            return Vec::new();
        }

        self.update_position();
        let target_time = self.position_micros + window_micros;

        let mut events = Vec::new();

        while let Some(event) = self.queue.peek() {
            if event.time_micros <= target_time {
                events.push(self.queue.pop().unwrap());
            } else {
                break;
            }
        }

        events
    }

    /// Calculate delay until next event
    pub fn time_to_next_event(&self) -> Option<Duration> {
        if !self.playing {
            return None;
        }

        self.queue.peek().map(|event| {
            let event_time = event.time_micros;
            if event_time <= self.position_micros {
                Duration::ZERO
            } else {
                Duration::from_micros(event_time - self.position_micros)
            }
        })
    }

    /// Recalculate microsecond times for all events after tempo change
    fn recalculate_event_times(&mut self) {
        let events: Vec<ScheduledEvent> = self.queue.drain().collect();
        for mut event in events {
            event.time_micros = self.timing.ticks_to_micros(event.time_ticks);
            self.queue.push(event);
        }
    }

    /// Apply drift correction
    pub fn apply_drift_correction(&mut self, correction_micros: i64) {
        self.timing_error_micros += correction_micros;
    }

    /// Get accumulated timing error
    pub fn timing_error(&self) -> i64 {
        self.timing_error_micros
    }

    /// Seek to a specific tick position
    pub fn seek(&mut self, ticks: u64) {
        let was_playing = self.playing;
        if was_playing {
            self.pause();
        }

        self.timing.position_ticks = ticks;
        self.position_micros = self.timing.ticks_to_micros(ticks);

        // Remove events before the seek position
        let events: Vec<ScheduledEvent> = self.queue
            .drain()
            .filter(|e| e.time_ticks >= ticks)
            .collect();

        for event in events {
            self.queue.push(event);
        }

        if was_playing {
            self.resume();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new();
        assert!(!scheduler.is_playing());
        assert_eq!(scheduler.queue_len(), 0);
        assert_eq!(scheduler.tempo(), 120.0);
    }

    #[test]
    fn test_schedule_events() {
        let mut scheduler = Scheduler::new();

        scheduler.schedule(ScheduledEvent::note_on(0, 0, 60, 100));
        scheduler.schedule(ScheduledEvent::note_off(24, 0, 60));

        assert_eq!(scheduler.queue_len(), 2);
    }

    #[test]
    fn test_event_ordering() {
        let mut scheduler = Scheduler::new();

        // Schedule events out of order
        scheduler.schedule(ScheduledEvent::note_on(48, 0, 62, 100));
        scheduler.schedule(ScheduledEvent::note_on(0, 0, 60, 100));
        scheduler.schedule(ScheduledEvent::note_on(24, 0, 61, 100));

        scheduler.start();
        // Poll with large window to get all events
        let events = scheduler.poll_window(10_000_000);

        assert_eq!(events.len(), 3);
        // Events should be in time order
        assert_eq!(events[0].data1, 60); // tick 0
        assert_eq!(events[1].data1, 61); // tick 24
        assert_eq!(events[2].data1, 62); // tick 48
    }

    #[test]
    fn test_start_stop() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(ScheduledEvent::note_on(0, 0, 60, 100));

        assert!(!scheduler.is_playing());

        scheduler.start();
        assert!(scheduler.is_playing());

        scheduler.stop();
        assert!(!scheduler.is_playing());
        assert_eq!(scheduler.position_ticks(), 0);
    }

    #[test]
    fn test_tempo_change() {
        let mut scheduler = Scheduler::new();
        scheduler.set_tempo(60.0);

        // At 60 BPM, one beat = 1 second = 1,000,000 micros
        // One tick = 1,000,000 / 24 ≈ 41,667 micros
        scheduler.schedule(ScheduledEvent::note_on(24, 0, 60, 100));

        // The event at tick 24 should be at ~1,000,000 micros
        let events: Vec<_> = scheduler.queue.iter().collect();
        let event = events[0];
        assert!((event.time_micros as i64 - 1_000_000).abs() < 100);
    }

    #[test]
    fn test_midi_bytes() {
        let note_on = ScheduledEvent::note_on(0, 1, 60, 100);
        assert_eq!(note_on.to_midi_bytes(), vec![0x91, 60, 100]);

        let note_off = ScheduledEvent::note_off(0, 2, 64);
        assert_eq!(note_off.to_midi_bytes(), vec![0x82, 64, 0]);

        let cc = ScheduledEvent::control_change(0, 0, 1, 64);
        assert_eq!(cc.to_midi_bytes(), vec![0xB0, 1, 64]);
    }

    #[test]
    fn test_clear_queue() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(ScheduledEvent::note_on(0, 0, 60, 100));
        scheduler.schedule(ScheduledEvent::note_on(24, 0, 61, 100));

        assert_eq!(scheduler.queue_len(), 2);
        scheduler.clear();
        assert_eq!(scheduler.queue_len(), 0);
    }

    #[test]
    fn test_seek() {
        let mut scheduler = Scheduler::new();

        // Schedule events at ticks 0, 24, 48, 72
        scheduler.schedule(ScheduledEvent::note_on(0, 0, 60, 100));
        scheduler.schedule(ScheduledEvent::note_on(24, 0, 61, 100));
        scheduler.schedule(ScheduledEvent::note_on(48, 0, 62, 100));
        scheduler.schedule(ScheduledEvent::note_on(72, 0, 63, 100));

        // Seek to tick 48
        scheduler.seek(48);

        // Should only have events at tick 48 and 72
        assert_eq!(scheduler.queue_len(), 2);
        assert_eq!(scheduler.position_ticks(), 48);
    }

    #[test]
    fn test_time_signature() {
        let mut scheduler = Scheduler::new();
        scheduler.set_time_signature(3, 4); // 3/4 time

        assert_eq!(scheduler.timing().beats_per_bar, 3);
        assert_eq!(scheduler.timing().ticks_per_bar(), 72); // 3 * 24
    }
}
