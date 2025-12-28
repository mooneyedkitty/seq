// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Clip system for sequenced and generated content.
//!
//! Provides clips that can contain static sequences, generate content
//! in real-time, or combine both approaches.

use crate::generators::{Generator, GeneratorContext, MidiEvent};

/// Clip playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipState {
    /// Clip is stopped
    Stopped,
    /// Clip is playing
    Playing,
    /// Clip is queued to start
    Queued,
    /// Clip is stopping (playing until end)
    Stopping,
}

/// Clip playback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    /// Play once and stop
    OneShot,
    /// Loop continuously
    Loop,
    /// Loop a specified number of times
    LoopCount(u32),
    /// Ping-pong (forward then backward)
    PingPong,
}

impl Default for ClipMode {
    fn default() -> Self {
        ClipMode::Loop
    }
}

/// A note event within a clip
#[derive(Debug, Clone, PartialEq)]
pub struct ClipNote {
    /// Start position in ticks from clip start
    pub start_tick: u64,
    /// Duration in ticks
    pub duration: u64,
    /// MIDI note number
    pub note: u8,
    /// Velocity
    pub velocity: u8,
}

impl ClipNote {
    /// Create a new clip note
    pub fn new(start_tick: u64, duration: u64, note: u8, velocity: u8) -> Self {
        Self {
            start_tick,
            duration,
            note,
            velocity,
        }
    }

    /// Convert to MidiEvent
    pub fn to_midi_event(&self) -> MidiEvent {
        MidiEvent::new(self.note, self.velocity, self.start_tick, self.duration)
    }
}

/// Clip type - static, generated, or hybrid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipType {
    /// Static sequence of notes
    Sequenced,
    /// Real-time generated content
    Generated,
    /// Static sequence with variations
    Hybrid,
}

/// A clip containing musical content
pub struct Clip {
    /// Clip name
    name: String,
    /// Clip type
    clip_type: ClipType,
    /// Playback mode
    mode: ClipMode,
    /// Current playback state
    state: ClipState,
    /// Length in ticks
    length_ticks: u64,
    /// Loop start point in ticks
    loop_start: u64,
    /// Loop end point in ticks (0 = use length)
    loop_end: u64,
    /// Static notes (for Sequenced and Hybrid)
    notes: Vec<ClipNote>,
    /// Generator (for Generated and Hybrid)
    generator: Option<Box<dyn Generator>>,
    /// Current position in ticks (relative to clip start)
    position: u64,
    /// Number of times looped
    loop_count: u32,
    /// Variation amount for hybrid mode (0.0 - 1.0)
    variation: f64,
    /// Whether playing in reverse (for ping-pong)
    reverse: bool,
}

impl Clip {
    /// Create a new empty clip
    pub fn new(name: impl Into<String>, length_ticks: u64) -> Self {
        Self {
            name: name.into(),
            clip_type: ClipType::Sequenced,
            mode: ClipMode::Loop,
            state: ClipState::Stopped,
            length_ticks,
            loop_start: 0,
            loop_end: 0,
            notes: Vec::new(),
            generator: None,
            position: 0,
            loop_count: 0,
            variation: 0.0,
            reverse: false,
        }
    }

    /// Create a generated clip
    pub fn generated(name: impl Into<String>, generator: Box<dyn Generator>) -> Self {
        Self {
            name: name.into(),
            clip_type: ClipType::Generated,
            mode: ClipMode::Loop,
            state: ClipState::Stopped,
            length_ticks: 96, // Default to 1 bar
            loop_start: 0,
            loop_end: 0,
            notes: Vec::new(),
            generator: Some(generator),
            position: 0,
            loop_count: 0,
            variation: 0.0,
            reverse: false,
        }
    }

    /// Create a hybrid clip
    pub fn hybrid(
        name: impl Into<String>,
        length_ticks: u64,
        generator: Box<dyn Generator>,
        variation: f64,
    ) -> Self {
        Self {
            name: name.into(),
            clip_type: ClipType::Hybrid,
            mode: ClipMode::Loop,
            state: ClipState::Stopped,
            length_ticks,
            loop_start: 0,
            loop_end: 0,
            notes: Vec::new(),
            generator: Some(generator),
            position: 0,
            loop_count: 0,
            variation: variation.clamp(0.0, 1.0),
            reverse: false,
        }
    }

    /// Get clip name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get clip type
    pub fn clip_type(&self) -> ClipType {
        self.clip_type
    }

    /// Get playback mode
    pub fn mode(&self) -> ClipMode {
        self.mode
    }

    /// Set playback mode
    pub fn set_mode(&mut self, mode: ClipMode) {
        self.mode = mode;
    }

    /// Get current state
    pub fn state(&self) -> ClipState {
        self.state
    }

    /// Get length in ticks
    pub fn length(&self) -> u64 {
        self.length_ticks
    }

    /// Set length in ticks
    pub fn set_length(&mut self, ticks: u64) {
        self.length_ticks = ticks;
    }

    /// Set loop points
    pub fn set_loop_points(&mut self, start: u64, end: u64) {
        self.loop_start = start.min(self.length_ticks);
        self.loop_end = if end == 0 { 0 } else { end.min(self.length_ticks) };
    }

    /// Get effective loop end (accounting for 0 meaning end of clip)
    fn effective_loop_end(&self) -> u64 {
        if self.loop_end == 0 {
            self.length_ticks
        } else {
            self.loop_end
        }
    }

    /// Add a note to the clip
    pub fn add_note(&mut self, note: ClipNote) {
        self.notes.push(note);
        // Keep notes sorted by start time
        self.notes.sort_by_key(|n| n.start_tick);
    }

    /// Add multiple notes
    pub fn add_notes(&mut self, notes: impl IntoIterator<Item = ClipNote>) {
        for note in notes {
            self.notes.push(note);
        }
        self.notes.sort_by_key(|n| n.start_tick);
    }

    /// Get all notes
    pub fn notes(&self) -> &[ClipNote] {
        &self.notes
    }

    /// Clear all notes
    pub fn clear_notes(&mut self) {
        self.notes.clear();
    }

    /// Get number of notes
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Set the generator
    pub fn set_generator(&mut self, generator: Box<dyn Generator>) {
        self.generator = Some(generator);
    }

    /// Clear the generator
    pub fn clear_generator(&mut self) {
        self.generator = None;
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get loop count
    pub fn loop_count(&self) -> u32 {
        self.loop_count
    }

    /// Start playback
    pub fn play(&mut self) {
        self.state = ClipState::Playing;
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = ClipState::Stopped;
        self.position = 0;
        self.loop_count = 0;
        self.reverse = false;
    }

    /// Queue for playback
    pub fn queue(&mut self) {
        self.state = ClipState::Queued;
    }

    /// Mark as stopping (play until end)
    pub fn stop_at_end(&mut self) {
        if self.state == ClipState::Playing {
            self.state = ClipState::Stopping;
        }
    }

    /// Check if clip is playing
    pub fn is_playing(&self) -> bool {
        self.state == ClipState::Playing || self.state == ClipState::Stopping
    }

    /// Reset clip state
    pub fn reset(&mut self) {
        self.position = 0;
        self.loop_count = 0;
        self.reverse = false;
        self.state = ClipState::Stopped;
        if let Some(ref mut gen) = self.generator {
            gen.reset();
        }
    }

    /// Generate events for the given context
    pub fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        if self.state != ClipState::Playing && self.state != ClipState::Stopping {
            return Vec::new();
        }

        let mut events = Vec::new();
        let ticks = context.ticks_to_generate;
        let loop_end = self.effective_loop_end();

        match self.clip_type {
            ClipType::Sequenced => {
                events = self.generate_sequenced(ticks, loop_end);
            }
            ClipType::Generated => {
                if let Some(ref mut gen) = self.generator {
                    events = gen.generate(context);
                }
            }
            ClipType::Hybrid => {
                // Mix sequenced and generated content
                let sequenced = self.generate_sequenced(ticks, loop_end);

                if let Some(ref mut gen) = self.generator {
                    let generated = gen.generate(context);
                    events = self.mix_events(sequenced, generated);
                } else {
                    events = sequenced;
                }
            }
        }

        // Advance position
        self.advance_position(ticks, loop_end);

        events
    }

    /// Generate events from sequenced notes
    fn generate_sequenced(&self, ticks: u64, loop_end: u64) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let start = self.position;
        let end = start + ticks;

        for note in &self.notes {
            // Check if note starts within our window
            let note_start = if self.reverse {
                loop_end - note.start_tick - note.duration
            } else {
                note.start_tick
            };

            if note_start >= self.loop_start && note_start < loop_end {
                // Calculate position relative to current playback position
                if note_start >= start && note_start < end {
                    let relative_start = note_start - start;
                    events.push(MidiEvent::new(
                        note.note,
                        note.velocity,
                        relative_start,
                        note.duration,
                    ));
                }
            }
        }

        events
    }

    /// Mix sequenced and generated events for hybrid mode
    fn mix_events(&self, sequenced: Vec<MidiEvent>, generated: Vec<MidiEvent>) -> Vec<MidiEvent> {
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;

        let mut rng = StdRng::from_entropy();
        let mut events = Vec::new();

        // Add sequenced events (always)
        for event in sequenced {
            events.push(event);
        }

        // Add generated events based on variation amount
        for event in generated {
            if rng.gen::<f64>() < self.variation {
                events.push(event);
            }
        }

        events
    }

    /// Advance playback position
    fn advance_position(&mut self, ticks: u64, loop_end: u64) {
        let loop_length = loop_end - self.loop_start;

        if self.reverse {
            if self.position < ticks {
                // Reached start
                match self.mode {
                    ClipMode::PingPong => {
                        self.reverse = false;
                        self.position = self.loop_start + (ticks - self.position);
                    }
                    _ => {
                        self.position = loop_end - (ticks - self.position);
                    }
                }
            } else {
                self.position -= ticks;
            }
        } else {
            self.position += ticks;

            if self.position >= loop_end {
                match self.mode {
                    ClipMode::OneShot => {
                        self.state = ClipState::Stopped;
                        self.position = 0;
                    }
                    ClipMode::Loop => {
                        self.position = self.loop_start + ((self.position - self.loop_start) % loop_length);
                        self.loop_count += 1;
                    }
                    ClipMode::LoopCount(max) => {
                        self.loop_count += 1;
                        if self.loop_count >= max {
                            self.state = ClipState::Stopped;
                            self.position = 0;
                        } else {
                            self.position = self.loop_start + ((self.position - self.loop_start) % loop_length);
                        }
                    }
                    ClipMode::PingPong => {
                        self.reverse = true;
                        self.position = loop_end - (self.position - loop_end);
                        self.loop_count += 1;
                    }
                }

                // Check if we should stop
                if self.state == ClipState::Stopping {
                    self.state = ClipState::Stopped;
                    self.position = 0;
                }
            }
        }
    }
}

impl Clone for Clip {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            clip_type: self.clip_type,
            mode: self.mode,
            state: self.state,
            length_ticks: self.length_ticks,
            loop_start: self.loop_start,
            loop_end: self.loop_end,
            notes: self.notes.clone(),
            generator: None, // Generators are not cloneable
            position: self.position,
            loop_count: self.loop_count,
            variation: self.variation,
            reverse: self.reverse,
        }
    }
}

/// Builder for creating clips with a fluent API
pub struct ClipBuilder {
    clip: Clip,
}

impl ClipBuilder {
    /// Start building a new clip
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            clip: Clip::new(name, 96), // Default 1 bar
        }
    }

    /// Set clip length in bars (assuming 4/4 time, 24 PPQN)
    pub fn bars(mut self, bars: u32) -> Self {
        self.clip.length_ticks = bars as u64 * 96;
        self
    }

    /// Set clip length in beats
    pub fn beats(mut self, beats: u32) -> Self {
        self.clip.length_ticks = beats as u64 * 24;
        self
    }

    /// Set clip length in ticks
    pub fn ticks(mut self, ticks: u64) -> Self {
        self.clip.length_ticks = ticks;
        self
    }

    /// Set playback mode
    pub fn mode(mut self, mode: ClipMode) -> Self {
        self.clip.mode = mode;
        self
    }

    /// Set as one-shot
    pub fn one_shot(mut self) -> Self {
        self.clip.mode = ClipMode::OneShot;
        self
    }

    /// Set loop points
    pub fn loop_points(mut self, start: u64, end: u64) -> Self {
        self.clip.set_loop_points(start, end);
        self
    }

    /// Add a note
    pub fn note(mut self, start: u64, duration: u64, pitch: u8, velocity: u8) -> Self {
        self.clip.add_note(ClipNote::new(start, duration, pitch, velocity));
        self
    }

    /// Set generator
    pub fn generator(mut self, gen: Box<dyn Generator>) -> Self {
        self.clip.generator = Some(gen);
        self.clip.clip_type = ClipType::Generated;
        self
    }

    /// Set as hybrid with variation
    pub fn hybrid(mut self, gen: Box<dyn Generator>, variation: f64) -> Self {
        self.clip.generator = Some(gen);
        self.clip.clip_type = ClipType::Hybrid;
        self.clip.variation = variation.clamp(0.0, 1.0);
        self
    }

    /// Build the clip
    pub fn build(self) -> Clip {
        self.clip
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::scale::{Key, Note, ScaleType};

    fn test_context(ticks: u64) -> GeneratorContext {
        GeneratorContext {
            key: Key::new(Note::C, ScaleType::Major),
            ppqn: 24,
            ticks_to_generate: ticks,
            tempo: 120.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_clip_creation() {
        let clip = Clip::new("Test Clip", 96);
        assert_eq!(clip.name(), "Test Clip");
        assert_eq!(clip.length(), 96);
        assert_eq!(clip.state(), ClipState::Stopped);
    }

    #[test]
    fn test_clip_notes() {
        let mut clip = Clip::new("Test", 96);

        clip.add_note(ClipNote::new(0, 24, 60, 100));
        clip.add_note(ClipNote::new(24, 24, 62, 90));

        assert_eq!(clip.note_count(), 2);
        assert_eq!(clip.notes()[0].note, 60);
        assert_eq!(clip.notes()[1].note, 62);
    }

    #[test]
    fn test_clip_playback() {
        let mut clip = Clip::new("Test", 96);
        clip.add_note(ClipNote::new(0, 24, 60, 100));

        assert!(!clip.is_playing());

        clip.play();
        assert!(clip.is_playing());

        clip.stop();
        assert!(!clip.is_playing());
        assert_eq!(clip.position(), 0);
    }

    #[test]
    fn test_clip_generate() {
        let mut clip = Clip::new("Test", 96);
        clip.add_note(ClipNote::new(0, 24, 60, 100));
        clip.add_note(ClipNote::new(24, 24, 62, 90));

        clip.play();
        let ctx = test_context(24);
        let events = clip.generate(&ctx);

        // Should get the first note
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].note, 60);

        // Generate next chunk
        let events = clip.generate(&ctx);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].note, 62);
    }

    #[test]
    fn test_clip_loop() {
        let mut clip = Clip::new("Test", 48); // Half bar
        clip.set_mode(ClipMode::Loop);
        clip.add_note(ClipNote::new(0, 12, 60, 100));

        clip.play();
        let ctx = test_context(24);

        // First chunk
        clip.generate(&ctx);
        assert_eq!(clip.position(), 24);

        // Second chunk (should loop)
        clip.generate(&ctx);
        assert_eq!(clip.position(), 0);
        assert_eq!(clip.loop_count(), 1);
    }

    #[test]
    fn test_clip_one_shot() {
        let mut clip = Clip::new("Test", 24);
        clip.set_mode(ClipMode::OneShot);

        clip.play();
        let ctx = test_context(24);

        clip.generate(&ctx);

        // Should stop after one play
        assert!(!clip.is_playing());
    }

    #[test]
    fn test_clip_loop_count() {
        let mut clip = Clip::new("Test", 24);
        clip.set_mode(ClipMode::LoopCount(2));

        clip.play();
        let ctx = test_context(24);

        // First loop
        clip.generate(&ctx);
        assert!(clip.is_playing());

        // Second loop
        clip.generate(&ctx);
        assert!(!clip.is_playing());
    }

    #[test]
    fn test_clip_loop_points() {
        let mut clip = Clip::new("Test", 96);
        clip.set_loop_points(24, 72); // Loop between beat 2 and 4

        clip.play();

        // Advance to loop end
        clip.position = 72;
        let ctx = test_context(24);
        clip.generate(&ctx);

        // Should have looped back to loop start
        assert!(clip.position() >= 24 && clip.position() < 72);
    }

    #[test]
    fn test_clip_builder() {
        let clip = ClipBuilder::new("Built Clip")
            .bars(2)
            .one_shot()
            .note(0, 24, 60, 100)
            .note(24, 24, 64, 90)
            .build();

        assert_eq!(clip.name(), "Built Clip");
        assert_eq!(clip.length(), 192);
        assert_eq!(clip.mode(), ClipMode::OneShot);
        assert_eq!(clip.note_count(), 2);
    }

    #[test]
    fn test_clip_stop_at_end() {
        let mut clip = Clip::new("Test", 24);
        clip.set_mode(ClipMode::Loop);

        clip.play();
        clip.stop_at_end();

        assert_eq!(clip.state(), ClipState::Stopping);

        let ctx = test_context(24);
        clip.generate(&ctx);

        assert!(!clip.is_playing());
    }

    #[test]
    fn test_clip_reset() {
        let mut clip = Clip::new("Test", 96);
        clip.play();
        clip.position = 48;
        clip.loop_count = 5;

        clip.reset();

        assert_eq!(clip.position(), 0);
        assert_eq!(clip.loop_count(), 0);
        assert!(!clip.is_playing());
    }
}
