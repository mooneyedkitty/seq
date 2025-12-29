// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Song mode for linear arrangement playback.
//!
//! Provides ordered arrangement of parts with auto-advance,
//! loop regions, and position tracking.

use std::collections::HashMap;

/// Song playback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongMode {
    /// Stopped
    Stopped,
    /// Playing through arrangement
    Playing,
    /// Looping a section
    Looping,
    /// Recording arrangement
    Recording,
}

impl Default for SongMode {
    fn default() -> Self {
        SongMode::Stopped
    }
}

/// Position within the song
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongPosition {
    /// Current section index
    pub section: usize,
    /// Bar within section (0-indexed)
    pub bar: u32,
    /// Beat within bar (0-indexed)
    pub beat: u32,
    /// Tick within beat
    pub tick: u32,
}

impl Default for SongPosition {
    fn default() -> Self {
        Self {
            section: 0,
            bar: 0,
            beat: 0,
            tick: 0,
        }
    }
}

impl SongPosition {
    /// Create a new position
    pub fn new(section: usize, bar: u32, beat: u32, tick: u32) -> Self {
        Self { section, bar, beat, tick }
    }

    /// Create position at section start
    pub fn at_section(section: usize) -> Self {
        Self::new(section, 0, 0, 0)
    }

    /// Get total ticks from song start
    pub fn to_ticks(&self, ppqn: u32, beats_per_bar: u32, section_lengths: &[u32]) -> u64 {
        let mut total = 0u64;

        // Add complete sections
        for i in 0..self.section {
            if let Some(&bars) = section_lengths.get(i) {
                total += bars as u64 * beats_per_bar as u64 * ppqn as u64;
            }
        }

        // Add bars, beats, ticks in current section
        total += self.bar as u64 * beats_per_bar as u64 * ppqn as u64;
        total += self.beat as u64 * ppqn as u64;
        total += self.tick as u64;

        total
    }

    /// Format position as string
    pub fn format(&self) -> String {
        format!("S{}:{}.{}.{:02}",
            self.section + 1,
            self.bar + 1,
            self.beat + 1,
            self.tick
        )
    }
}

/// A section in the song arrangement
#[derive(Debug, Clone)]
pub struct SongSection {
    /// Part name to play
    part_name: String,
    /// Length in bars
    length_bars: u32,
    /// Scene index to trigger (optional)
    scene_index: Option<usize>,
    /// Tempo for this section (None = keep current)
    tempo: Option<f64>,
    /// Time signature numerator
    time_sig_num: u8,
    /// Time signature denominator
    time_sig_denom: u8,
    /// Whether this section is a loop point
    is_loop_point: bool,
    /// Section color for UI
    color: (u8, u8, u8),
    /// Notes/comments
    notes: String,
}

impl SongSection {
    /// Create a new section
    pub fn new(part_name: impl Into<String>, length_bars: u32) -> Self {
        Self {
            part_name: part_name.into(),
            length_bars,
            scene_index: None,
            tempo: None,
            time_sig_num: 4,
            time_sig_denom: 4,
            is_loop_point: false,
            color: (100, 100, 100),
            notes: String::new(),
        }
    }

    /// Get part name
    pub fn part_name(&self) -> &str {
        &self.part_name
    }

    /// Get length in bars
    pub fn length_bars(&self) -> u32 {
        self.length_bars
    }

    /// Set length in bars
    pub fn set_length(&mut self, bars: u32) {
        self.length_bars = bars.max(1);
    }

    /// Get scene index
    pub fn scene_index(&self) -> Option<usize> {
        self.scene_index
    }

    /// Set scene index
    pub fn set_scene(&mut self, index: Option<usize>) {
        self.scene_index = index;
    }

    /// Get tempo
    pub fn tempo(&self) -> Option<f64> {
        self.tempo
    }

    /// Set tempo
    pub fn set_tempo(&mut self, tempo: Option<f64>) {
        self.tempo = tempo;
    }

    /// Get time signature
    pub fn time_signature(&self) -> (u8, u8) {
        (self.time_sig_num, self.time_sig_denom)
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, num: u8, denom: u8) {
        self.time_sig_num = num.max(1);
        self.time_sig_denom = denom.max(1);
    }

    /// Check if loop point
    pub fn is_loop_point(&self) -> bool {
        self.is_loop_point
    }

    /// Set loop point
    pub fn set_loop_point(&mut self, is_loop: bool) {
        self.is_loop_point = is_loop;
    }

    /// Get color
    pub fn color(&self) -> (u8, u8, u8) {
        self.color
    }

    /// Set color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color = (r, g, b);
    }

    /// Get notes
    pub fn notes(&self) -> &str {
        &self.notes
    }

    /// Set notes
    pub fn set_notes(&mut self, notes: impl Into<String>) {
        self.notes = notes.into();
    }

    /// Builder: set scene
    pub fn with_scene(mut self, index: usize) -> Self {
        self.scene_index = Some(index);
        self
    }

    /// Builder: set tempo
    pub fn with_tempo(mut self, tempo: f64) -> Self {
        self.tempo = Some(tempo);
        self
    }

    /// Builder: set time signature
    pub fn with_time_sig(mut self, num: u8, denom: u8) -> Self {
        self.time_sig_num = num;
        self.time_sig_denom = denom;
        self
    }

    /// Builder: set as loop point
    pub fn as_loop_point(mut self) -> Self {
        self.is_loop_point = true;
        self
    }
}

/// Loop region for song
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopRegion {
    /// Start section index
    pub start_section: usize,
    /// End section index (inclusive)
    pub end_section: usize,
    /// Number of times to loop (None = infinite)
    pub repeat_count: Option<u32>,
    /// Current repeat number
    pub current_repeat: u32,
}

impl LoopRegion {
    /// Create a new loop region
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start_section: start,
            end_section: end,
            repeat_count: None,
            current_repeat: 0,
        }
    }

    /// Create with repeat count
    pub fn with_count(start: usize, end: usize, count: u32) -> Self {
        Self {
            start_section: start,
            end_section: end,
            repeat_count: Some(count),
            current_repeat: 0,
        }
    }

    /// Check if loop is done
    pub fn is_done(&self) -> bool {
        if let Some(count) = self.repeat_count {
            self.current_repeat >= count
        } else {
            false
        }
    }
}

/// A complete song arrangement
#[derive(Debug, Clone)]
pub struct Song {
    /// Song name
    name: String,
    /// Song sections
    sections: Vec<SongSection>,
    /// Default tempo
    default_tempo: f64,
    /// Default time signature
    default_time_sig: (u8, u8),
    /// Song metadata
    metadata: HashMap<String, String>,
}

impl Song {
    /// Create a new empty song
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sections: Vec::new(),
            default_tempo: 120.0,
            default_time_sig: (4, 4),
            metadata: HashMap::new(),
        }
    }

    /// Get song name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set song name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Add a section
    pub fn add_section(&mut self, section: SongSection) {
        self.sections.push(section);
    }

    /// Insert section at index
    pub fn insert_section(&mut self, index: usize, section: SongSection) {
        if index <= self.sections.len() {
            self.sections.insert(index, section);
        } else {
            self.sections.push(section);
        }
    }

    /// Remove section at index
    pub fn remove_section(&mut self, index: usize) -> Option<SongSection> {
        if index < self.sections.len() {
            Some(self.sections.remove(index))
        } else {
            None
        }
    }

    /// Get section at index
    pub fn get_section(&self, index: usize) -> Option<&SongSection> {
        self.sections.get(index)
    }

    /// Get mutable section at index
    pub fn get_section_mut(&mut self, index: usize) -> Option<&mut SongSection> {
        self.sections.get_mut(index)
    }

    /// Get all sections
    pub fn sections(&self) -> &[SongSection] {
        &self.sections
    }

    /// Number of sections
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Get total length in bars
    pub fn total_bars(&self) -> u32 {
        self.sections.iter().map(|s| s.length_bars()).sum()
    }

    /// Get section lengths as vec
    pub fn section_lengths(&self) -> Vec<u32> {
        self.sections.iter().map(|s| s.length_bars()).collect()
    }

    /// Get default tempo
    pub fn default_tempo(&self) -> f64 {
        self.default_tempo
    }

    /// Set default tempo
    pub fn set_default_tempo(&mut self, tempo: f64) {
        self.default_tempo = tempo.clamp(20.0, 300.0);
    }

    /// Get default time signature
    pub fn default_time_signature(&self) -> (u8, u8) {
        self.default_time_sig
    }

    /// Set default time signature
    pub fn set_default_time_signature(&mut self, num: u8, denom: u8) {
        self.default_time_sig = (num.max(1), denom.max(1));
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Get all metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Calculate position from tick
    pub fn position_from_tick(&self, tick: u64, ppqn: u32) -> SongPosition {
        let beats_per_bar = self.default_time_sig.0 as u32;
        let ticks_per_beat = ppqn as u64;
        let ticks_per_bar = ticks_per_beat * beats_per_bar as u64;

        let mut remaining = tick;
        let mut section_idx = 0;

        for (i, section) in self.sections.iter().enumerate() {
            let section_ticks = section.length_bars() as u64 * ticks_per_bar;
            if remaining < section_ticks {
                section_idx = i;
                break;
            }
            remaining -= section_ticks;
            section_idx = i + 1;
        }

        // If past end, stay at last section
        if section_idx >= self.sections.len() {
            section_idx = self.sections.len().saturating_sub(1);
        }

        let bars = (remaining / ticks_per_bar) as u32;
        remaining %= ticks_per_bar;
        let beats = (remaining / ticks_per_beat) as u32;
        let ticks = (remaining % ticks_per_beat) as u32;

        SongPosition {
            section: section_idx,
            bar: bars,
            beat: beats,
            tick: ticks,
        }
    }

    /// Builder: add section
    pub fn with_section(mut self, section: SongSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Builder: set default tempo
    pub fn with_tempo(mut self, tempo: f64) -> Self {
        self.default_tempo = tempo;
        self
    }

    /// Builder: set metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Song player for arrangement playback
pub struct SongPlayer {
    /// Current song
    song: Option<Song>,
    /// Playback mode
    mode: SongMode,
    /// Current position in ticks
    position_ticks: u64,
    /// Current section index
    current_section: usize,
    /// Loop region (if any)
    loop_region: Option<LoopRegion>,
    /// PPQN for timing calculations
    ppqn: u32,
    /// Beats per bar (default time sig)
    beats_per_bar: u32,
}

impl SongPlayer {
    /// Create a new song player
    pub fn new(ppqn: u32) -> Self {
        Self {
            song: None,
            mode: SongMode::Stopped,
            position_ticks: 0,
            current_section: 0,
            loop_region: None,
            ppqn,
            beats_per_bar: 4,
        }
    }

    /// Load a song
    pub fn load(&mut self, song: Song) {
        self.beats_per_bar = song.default_time_signature().0 as u32;
        self.song = Some(song);
        self.stop();
    }

    /// Unload current song
    pub fn unload(&mut self) {
        self.song = None;
        self.stop();
    }

    /// Get current song
    pub fn song(&self) -> Option<&Song> {
        self.song.as_ref()
    }

    /// Get playback mode
    pub fn mode(&self) -> SongMode {
        self.mode
    }

    /// Start playback
    pub fn play(&mut self) {
        if self.song.is_some() {
            self.mode = SongMode::Playing;
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.mode = SongMode::Stopped;
        self.position_ticks = 0;
        self.current_section = 0;
        if let Some(loop_region) = &mut self.loop_region {
            loop_region.current_repeat = 0;
        }
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.mode == SongMode::Playing {
            self.mode = SongMode::Stopped;
        }
    }

    /// Resume playback
    pub fn resume(&mut self) {
        if self.song.is_some() && self.mode == SongMode::Stopped {
            self.mode = SongMode::Playing;
        }
    }

    /// Get current position
    pub fn position(&self) -> SongPosition {
        if let Some(song) = &self.song {
            song.position_from_tick(self.position_ticks, self.ppqn)
        } else {
            SongPosition::default()
        }
    }

    /// Get position in ticks
    pub fn position_ticks(&self) -> u64 {
        self.position_ticks
    }

    /// Get current section
    pub fn current_section(&self) -> usize {
        self.current_section
    }

    /// Jump to section
    pub fn goto_section(&mut self, index: usize) {
        if let Some(song) = &self.song {
            if index < song.section_count() {
                self.current_section = index;
                self.position_ticks = SongPosition::at_section(index)
                    .to_ticks(self.ppqn, self.beats_per_bar, &song.section_lengths());
            }
        }
    }

    /// Set loop region
    pub fn set_loop(&mut self, start: usize, end: usize, count: Option<u32>) {
        self.loop_region = Some(LoopRegion {
            start_section: start,
            end_section: end,
            repeat_count: count,
            current_repeat: 0,
        });
        self.mode = SongMode::Looping;
    }

    /// Clear loop region
    pub fn clear_loop(&mut self) {
        self.loop_region = None;
        if self.mode == SongMode::Looping {
            self.mode = SongMode::Playing;
        }
    }

    /// Get loop region
    pub fn loop_region(&self) -> Option<&LoopRegion> {
        self.loop_region.as_ref()
    }

    /// Update player, returns section index if changed
    pub fn update(&mut self, ticks: u64) -> Option<usize> {
        if self.mode == SongMode::Stopped {
            return None;
        }

        let song = self.song.as_ref()?;
        if song.sections.is_empty() {
            return None;
        }

        self.position_ticks += ticks;

        // Calculate what section we should be in
        let new_position = song.position_from_tick(self.position_ticks, self.ppqn);
        let section_count = song.section_count();
        let section_lengths = song.section_lengths();

        if new_position.section != self.current_section {
            // Section changed
            let old_section = self.current_section;
            self.current_section = new_position.section;

            // Check for loop
            if let Some(loop_region) = &mut self.loop_region {
                if old_section == loop_region.end_section &&
                   new_position.section > loop_region.end_section {
                    // Reached end of loop region
                    loop_region.current_repeat += 1;

                    if !loop_region.is_done() {
                        // Jump back to loop start
                        self.current_section = loop_region.start_section;
                        self.position_ticks = SongPosition::at_section(loop_region.start_section)
                            .to_ticks(self.ppqn, self.beats_per_bar, &section_lengths);
                    } else {
                        // Loop finished, continue or stop
                        self.mode = SongMode::Playing;
                    }
                }
            }

            // Check for end of song
            if self.current_section >= section_count {
                // Inline stop to avoid borrow issues
                self.mode = SongMode::Stopped;
                self.position_ticks = 0;
                self.current_section = 0;
                if let Some(loop_region) = &mut self.loop_region {
                    loop_region.current_repeat = 0;
                }
                return None;
            }

            return Some(self.current_section);
        }

        None
    }

    /// Get section at index from current song
    pub fn get_section(&self, index: usize) -> Option<&SongSection> {
        self.song.as_ref().and_then(|s| s.get_section(index))
    }

    /// Check if at end of song
    pub fn is_at_end(&self) -> bool {
        if let Some(song) = &self.song {
            self.current_section >= song.section_count()
        } else {
            true
        }
    }
}

impl Default for SongPlayer {
    fn default() -> Self {
        Self::new(24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_creation() {
        let song = Song::new("Test Song");
        assert_eq!(song.name(), "Test Song");
        assert!(song.sections().is_empty());
        assert_eq!(song.default_tempo(), 120.0);
    }

    #[test]
    fn test_song_sections() {
        let song = Song::new("Test")
            .with_section(SongSection::new("Intro", 4))
            .with_section(SongSection::new("Verse", 8))
            .with_section(SongSection::new("Chorus", 8));

        assert_eq!(song.section_count(), 3);
        assert_eq!(song.total_bars(), 20);
        assert_eq!(song.get_section(1).unwrap().part_name(), "Verse");
    }

    #[test]
    fn test_section_builder() {
        let section = SongSection::new("Bridge", 4)
            .with_tempo(140.0)
            .with_scene(2)
            .with_time_sig(3, 4)
            .as_loop_point();

        assert_eq!(section.tempo(), Some(140.0));
        assert_eq!(section.scene_index(), Some(2));
        assert_eq!(section.time_signature(), (3, 4));
        assert!(section.is_loop_point());
    }

    #[test]
    fn test_song_position() {
        let pos = SongPosition::new(2, 3, 1, 12);
        assert_eq!(pos.format(), "S3:4.2.12");

        let section_lengths = vec![4, 8, 8];
        let ticks = pos.to_ticks(24, 4, &section_lengths);
        // Section 0: 4 bars = 4 * 4 * 24 = 384
        // Section 1: 8 bars = 8 * 4 * 24 = 768
        // Current: 3 bars + 1 beat + 12 ticks = 3*96 + 24 + 12 = 324
        assert_eq!(ticks, 384 + 768 + 324);
    }

    #[test]
    fn test_position_from_tick() {
        let song = Song::new("Test")
            .with_section(SongSection::new("A", 4))
            .with_section(SongSection::new("B", 4))
            .with_section(SongSection::new("C", 4));

        let ppqn = 24;
        // Position in section B (tick 400 = past 4 bars of A = 384)
        let pos = song.position_from_tick(400, ppqn);
        assert_eq!(pos.section, 1);
        assert_eq!(pos.bar, 0);
        assert_eq!(pos.beat, 0);
        assert_eq!(pos.tick, 16);
    }

    #[test]
    fn test_song_player_basic() {
        let mut player = SongPlayer::new(24);

        let song = Song::new("Test")
            .with_section(SongSection::new("A", 2))
            .with_section(SongSection::new("B", 2));

        player.load(song);
        assert_eq!(player.mode(), SongMode::Stopped);

        player.play();
        assert_eq!(player.mode(), SongMode::Playing);

        player.pause();
        assert_eq!(player.mode(), SongMode::Stopped);

        player.resume();
        assert_eq!(player.mode(), SongMode::Playing);

        player.stop();
        assert_eq!(player.mode(), SongMode::Stopped);
        assert_eq!(player.position_ticks(), 0);
    }

    #[test]
    fn test_song_player_section_change() {
        let mut player = SongPlayer::new(24);

        let song = Song::new("Test")
            .with_section(SongSection::new("A", 1)) // 1 bar = 96 ticks
            .with_section(SongSection::new("B", 1));

        player.load(song);
        player.play();

        assert_eq!(player.current_section(), 0);

        // Move past section A
        let section_idx = player.update(100);
        assert!(section_idx.is_some());
        assert_eq!(section_idx.unwrap(), 1);
        assert_eq!(player.get_section(1).unwrap().part_name(), "B");
        assert_eq!(player.current_section(), 1);
    }

    #[test]
    fn test_song_player_goto() {
        let mut player = SongPlayer::new(24);

        let song = Song::new("Test")
            .with_section(SongSection::new("A", 4))
            .with_section(SongSection::new("B", 4))
            .with_section(SongSection::new("C", 4));

        player.load(song);
        player.goto_section(2);

        assert_eq!(player.current_section(), 2);
        // Should be at start of section C = 8 bars = 8 * 4 * 24 = 768 ticks
        assert_eq!(player.position_ticks(), 768);
    }

    #[test]
    fn test_loop_region() {
        let mut player = SongPlayer::new(24);

        let song = Song::new("Test")
            .with_section(SongSection::new("A", 1))
            .with_section(SongSection::new("B", 1))
            .with_section(SongSection::new("C", 1));

        player.load(song);
        player.play();
        player.set_loop(1, 1, Some(2)); // Loop section B twice

        // Move to section B
        player.goto_section(1);
        assert_eq!(player.current_section(), 1);

        // Complete first loop
        player.update(100); // Past section B
        // Should loop back
        assert_eq!(player.loop_region().unwrap().current_repeat, 1);

        // Complete second loop
        player.update(96); // One more bar
        assert_eq!(player.loop_region().unwrap().current_repeat, 2);
        // Should now continue to C
        assert_eq!(player.mode(), SongMode::Playing);
    }

    #[test]
    fn test_song_metadata() {
        let song = Song::new("Test")
            .with_metadata("artist", "Test Artist")
            .with_metadata("genre", "Electronic");

        assert_eq!(song.get_metadata("artist"), Some("Test Artist"));
        assert_eq!(song.get_metadata("genre"), Some("Electronic"));
        assert_eq!(song.get_metadata("unknown"), None);
    }
}
