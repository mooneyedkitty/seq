// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Track system for multi-channel MIDI output.
//!
//! Provides track state management with mute/solo, transpose,
//! swing, and channel routing.

use super::clip::{Clip, ClipState};
use super::scheduler::ScheduledEvent;
use crate::generators::{Generator, GeneratorContext, MidiEvent};

/// Track state for mute/solo/active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    /// Track is active and playing
    Active,
    /// Track is muted (produces no output)
    Muted,
    /// Track is soloed (only soloed tracks play)
    Soloed,
}

impl Default for TrackState {
    fn default() -> Self {
        TrackState::Active
    }
}

/// Configuration for a track
#[derive(Debug, Clone)]
pub struct TrackConfig {
    /// Track name
    pub name: String,
    /// MIDI channel (0-15)
    pub channel: u8,
    /// Transpose in semitones (-48 to +48)
    pub transpose: i8,
    /// Swing amount (0.0 to 1.0)
    pub swing: f64,
    /// Velocity scale (0.0 to 2.0)
    pub velocity_scale: f64,
    /// Velocity offset (-127 to +127)
    pub velocity_offset: i8,
    /// Note range minimum (0-127)
    pub note_min: u8,
    /// Note range maximum (0-127)
    pub note_max: u8,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            name: String::from("Track"),
            channel: 0,
            transpose: 0,
            swing: 0.0,
            velocity_scale: 1.0,
            velocity_offset: 0,
            note_min: 0,
            note_max: 127,
        }
    }
}

impl TrackConfig {
    /// Create a new track config with name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set MIDI channel
    pub fn with_channel(mut self, channel: u8) -> Self {
        self.channel = channel.min(15);
        self
    }

    /// Set transpose
    pub fn with_transpose(mut self, transpose: i8) -> Self {
        self.transpose = transpose.clamp(-48, 48);
        self
    }

    /// Set swing amount
    pub fn with_swing(mut self, swing: f64) -> Self {
        self.swing = swing.clamp(0.0, 1.0);
        self
    }
}

/// A sequencer track
pub struct Track {
    /// Track configuration
    config: TrackConfig,
    /// Current state
    state: TrackState,
    /// Active clip index (if any)
    active_clip: Option<usize>,
    /// Clips assigned to this track
    clips: Vec<Clip>,
    /// Generator for this track (if any)
    generator: Option<Box<dyn Generator>>,
    /// Current clip state
    clip_state: ClipState,
    /// Track index (for identification)
    index: usize,
    /// Whether this track has pending solo
    pending_solo: bool,
}

impl Track {
    /// Create a new track
    pub fn new(index: usize, config: TrackConfig) -> Self {
        Self {
            config,
            state: TrackState::Active,
            active_clip: None,
            clips: Vec::new(),
            generator: None,
            clip_state: ClipState::Stopped,
            index,
            pending_solo: false,
        }
    }

    /// Create a track with just an index
    pub fn with_index(index: usize) -> Self {
        Self::new(index, TrackConfig::new(format!("Track {}", index + 1)))
    }

    /// Get track name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Set track name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.config.name = name.into();
    }

    /// Get MIDI channel
    pub fn channel(&self) -> u8 {
        self.config.channel
    }

    /// Set MIDI channel
    pub fn set_channel(&mut self, channel: u8) {
        self.config.channel = channel.min(15);
    }

    /// Get transpose
    pub fn transpose(&self) -> i8 {
        self.config.transpose
    }

    /// Set transpose
    pub fn set_transpose(&mut self, transpose: i8) {
        self.config.transpose = transpose.clamp(-48, 48);
    }

    /// Get swing
    pub fn swing(&self) -> f64 {
        self.config.swing
    }

    /// Set swing
    pub fn set_swing(&mut self, swing: f64) {
        self.config.swing = swing.clamp(0.0, 1.0);
    }

    /// Get current state
    pub fn state(&self) -> TrackState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: TrackState) {
        self.state = state;
    }

    /// Check if track is muted
    pub fn is_muted(&self) -> bool {
        self.state == TrackState::Muted
    }

    /// Check if track is soloed
    pub fn is_soloed(&self) -> bool {
        self.state == TrackState::Soloed
    }

    /// Toggle mute
    pub fn toggle_mute(&mut self) {
        self.state = if self.state == TrackState::Muted {
            TrackState::Active
        } else {
            TrackState::Muted
        };
    }

    /// Toggle solo
    pub fn toggle_solo(&mut self) {
        self.state = if self.state == TrackState::Soloed {
            TrackState::Active
        } else {
            TrackState::Soloed
        };
    }

    /// Get track index
    pub fn index(&self) -> usize {
        self.index
    }

    /// Set the generator for this track
    pub fn set_generator(&mut self, generator: Box<dyn Generator>) {
        self.generator = Some(generator);
    }

    /// Get reference to generator
    pub fn generator(&self) -> Option<&dyn Generator> {
        self.generator.as_deref()
    }

    /// Get mutable reference to generator
    pub fn generator_mut(&mut self) -> Option<&mut Box<dyn Generator>> {
        self.generator.as_mut()
    }

    /// Clear the generator
    pub fn clear_generator(&mut self) {
        self.generator = None;
    }

    /// Add a clip to this track
    pub fn add_clip(&mut self, clip: Clip) -> usize {
        self.clips.push(clip);
        self.clips.len() - 1
    }

    /// Get a clip by index
    pub fn clip(&self, index: usize) -> Option<&Clip> {
        self.clips.get(index)
    }

    /// Get a mutable clip by index
    pub fn clip_mut(&mut self, index: usize) -> Option<&mut Clip> {
        self.clips.get_mut(index)
    }

    /// Get number of clips
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    /// Set active clip
    pub fn set_active_clip(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.clips.len() {
                self.active_clip = Some(idx);
            }
        } else {
            self.active_clip = None;
        }
    }

    /// Get active clip index
    pub fn active_clip_index(&self) -> Option<usize> {
        self.active_clip
    }

    /// Get active clip
    pub fn active_clip(&self) -> Option<&Clip> {
        self.active_clip.and_then(|idx| self.clips.get(idx))
    }

    /// Get mutable active clip
    pub fn active_clip_mut(&mut self) -> Option<&mut Clip> {
        self.active_clip.and_then(|idx| self.clips.get_mut(idx))
    }

    /// Process MIDI events - apply transpose and velocity scaling
    fn process_event(&self, mut event: MidiEvent) -> Option<MidiEvent> {
        // Apply transpose
        let transposed = event.note as i16 + self.config.transpose as i16;
        if transposed < 0 || transposed > 127 {
            return None; // Note out of range
        }
        event.note = transposed as u8;

        // Apply note range filter
        if event.note < self.config.note_min || event.note > self.config.note_max {
            return None;
        }

        // Apply velocity scaling and offset
        let scaled = (event.velocity as f64 * self.config.velocity_scale) as i16
            + self.config.velocity_offset as i16;
        event.velocity = scaled.clamp(1, 127) as u8;

        // Set channel
        event.channel = self.config.channel;

        Some(event)
    }

    /// Apply swing to tick position
    fn apply_swing(&self, tick: u64, ppqn: u32) -> u64 {
        if self.config.swing == 0.0 {
            return tick;
        }

        let beat_ticks = ppqn as u64;
        let half_beat = beat_ticks / 2;
        let tick_in_beat = tick % beat_ticks;

        // Apply swing to off-beat notes (second half of beat)
        if tick_in_beat >= half_beat {
            let swing_offset = (half_beat as f64 * self.config.swing * 0.5) as u64;
            tick + swing_offset
        } else {
            tick
        }
    }

    /// Generate events for this track
    pub fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        // Check if we should produce output
        if self.state == TrackState::Muted {
            return Vec::new();
        }

        let mut events = Vec::new();

        // Generate from generator if present
        if let Some(ref mut generator) = self.generator {
            let generated = generator.generate(context);
            for event in generated {
                if let Some(processed) = self.process_event(event) {
                    events.push(processed);
                }
            }
        }

        // Generate from active clip if present
        if let Some(clip_idx) = self.active_clip {
            if let Some(clip) = self.clips.get_mut(clip_idx) {
                let clip_events = clip.generate(context);
                for event in clip_events {
                    if let Some(processed) = self.process_event(event) {
                        events.push(processed);
                    }
                }
            }
        }

        // Apply swing
        for event in &mut events {
            event.start_tick = self.apply_swing(event.start_tick, context.ppqn);
        }

        events
    }

    /// Convert generated events to scheduled events
    pub fn generate_scheduled(
        &mut self,
        context: &GeneratorContext,
        base_tick: u64,
    ) -> Vec<ScheduledEvent> {
        let events = self.generate(context);
        let mut scheduled = Vec::new();

        for event in events {
            let start_tick = base_tick + event.start_tick;
            let end_tick = start_tick + event.duration_ticks;

            // Note on
            scheduled.push(
                ScheduledEvent::note_on(start_tick, event.channel, event.note, event.velocity)
                    .with_track(self.index),
            );

            // Note off
            scheduled.push(
                ScheduledEvent::note_off(end_tick, event.channel, event.note)
                    .with_track(self.index),
            );
        }

        scheduled
    }

    /// Reset the track
    pub fn reset(&mut self) {
        if let Some(ref mut generator) = self.generator {
            generator.reset();
        }
        for clip in &mut self.clips {
            clip.reset();
        }
        self.clip_state = ClipState::Stopped;
    }
}

/// Manager for multiple tracks with solo handling
pub struct TrackManager {
    tracks: Vec<Track>,
    /// Whether any track is soloed
    has_solo: bool,
}

impl TrackManager {
    /// Create a new track manager
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            has_solo: false,
        }
    }

    /// Add a track
    pub fn add_track(&mut self, config: TrackConfig) -> usize {
        let index = self.tracks.len();
        self.tracks.push(Track::new(index, config));
        index
    }

    /// Get a track by index
    pub fn track(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    /// Get a mutable track by index
    pub fn track_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }

    /// Get number of tracks
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Update solo state
    fn update_solo_state(&mut self) {
        self.has_solo = self.tracks.iter().any(|t| t.is_soloed());
    }

    /// Set track state and update solo handling
    pub fn set_track_state(&mut self, index: usize, state: TrackState) {
        if let Some(track) = self.tracks.get_mut(index) {
            track.set_state(state);
            self.update_solo_state();
        }
    }

    /// Toggle mute for a track
    pub fn toggle_mute(&mut self, index: usize) {
        if let Some(track) = self.tracks.get_mut(index) {
            track.toggle_mute();
            self.update_solo_state();
        }
    }

    /// Toggle solo for a track
    pub fn toggle_solo(&mut self, index: usize) {
        if let Some(track) = self.tracks.get_mut(index) {
            track.toggle_solo();
            self.update_solo_state();
        }
    }

    /// Check if track should produce output (considering solo)
    pub fn should_output(&self, index: usize) -> bool {
        if let Some(track) = self.tracks.get(index) {
            if track.is_muted() {
                return false;
            }
            if self.has_solo {
                return track.is_soloed();
            }
            true
        } else {
            false
        }
    }

    /// Generate events from all tracks
    pub fn generate_all(&mut self, context: &GeneratorContext, base_tick: u64) -> Vec<ScheduledEvent> {
        let mut all_events = Vec::new();

        for i in 0..self.tracks.len() {
            if self.should_output(i) {
                let events = self.tracks[i].generate_scheduled(context, base_tick);
                all_events.extend(events);
            }
        }

        all_events
    }

    /// Reset all tracks
    pub fn reset_all(&mut self) {
        for track in &mut self.tracks {
            track.reset();
        }
    }

    /// Iterate over tracks
    pub fn iter(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter()
    }

    /// Iterate over tracks mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Track> {
        self.tracks.iter_mut()
    }
}

impl Default for TrackManager {
    fn default() -> Self {
        Self::new()
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
            tempo: 120.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_track_creation() {
        let track = Track::with_index(0);
        assert_eq!(track.name(), "Track 1");
        assert_eq!(track.channel(), 0);
        assert_eq!(track.state(), TrackState::Active);
    }

    #[test]
    fn test_track_config() {
        let config = TrackConfig::new("Bass")
            .with_channel(2)
            .with_transpose(-12)
            .with_swing(0.3);

        let track = Track::new(0, config);
        assert_eq!(track.name(), "Bass");
        assert_eq!(track.channel(), 2);
        assert_eq!(track.transpose(), -12);
        assert!((track.swing() - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_mute_solo() {
        let mut track = Track::with_index(0);

        track.toggle_mute();
        assert!(track.is_muted());

        track.toggle_mute();
        assert!(!track.is_muted());

        track.toggle_solo();
        assert!(track.is_soloed());
    }

    #[test]
    fn test_transpose() {
        let mut track = Track::with_index(0);
        track.set_transpose(12);

        let event = MidiEvent::new(60, 100, 0, 24);
        let processed = track.process_event(event).unwrap();

        assert_eq!(processed.note, 72); // 60 + 12
    }

    #[test]
    fn test_transpose_out_of_range() {
        let mut track = Track::with_index(0);
        track.set_transpose(48);

        // Note 100 + 48 = 148, out of MIDI range
        let event = MidiEvent::new(100, 100, 0, 24);
        let processed = track.process_event(event);

        assert!(processed.is_none());
    }

    #[test]
    fn test_velocity_scaling() {
        let config = TrackConfig {
            velocity_scale: 0.5,
            velocity_offset: 10,
            ..Default::default()
        };
        let track = Track::new(0, config);

        let event = MidiEvent::new(60, 100, 0, 24);
        let processed = track.process_event(event).unwrap();

        // 100 * 0.5 + 10 = 60
        assert_eq!(processed.velocity, 60);
    }

    #[test]
    fn test_track_manager_solo() {
        let mut manager = TrackManager::new();
        manager.add_track(TrackConfig::new("Track 1"));
        manager.add_track(TrackConfig::new("Track 2"));
        manager.add_track(TrackConfig::new("Track 3"));

        // Initially all tracks output
        assert!(manager.should_output(0));
        assert!(manager.should_output(1));
        assert!(manager.should_output(2));

        // Solo track 1
        manager.toggle_solo(0);

        // Only track 1 should output
        assert!(manager.should_output(0));
        assert!(!manager.should_output(1));
        assert!(!manager.should_output(2));
    }

    #[test]
    fn test_track_manager_mute() {
        let mut manager = TrackManager::new();
        manager.add_track(TrackConfig::new("Track 1"));
        manager.add_track(TrackConfig::new("Track 2"));

        // Mute track 1
        manager.toggle_mute(0);

        assert!(!manager.should_output(0));
        assert!(manager.should_output(1));
    }

    #[test]
    fn test_swing_application() {
        let config = TrackConfig {
            swing: 0.5,
            ..Default::default()
        };
        let track = Track::new(0, config);

        // Tick 0 (on-beat) should not be affected
        assert_eq!(track.apply_swing(0, 24), 0);

        // Tick 12 (off-beat) should be delayed
        let swung = track.apply_swing(12, 24);
        assert!(swung > 12);
    }
}
