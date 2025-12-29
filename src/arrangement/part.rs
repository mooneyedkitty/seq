// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Parts system for managing track clip/generator states.
//!
//! A Part represents a collection of track states that can be
//! triggered together, enabling quick arrangement changes.

use std::collections::HashMap;

use crate::sequencer::TrackState;

/// State of a clip on a track within a part
#[derive(Debug, Clone, PartialEq)]
pub enum TrackClipState {
    /// No clip assigned
    Empty,
    /// Play specific clip by index
    Clip(usize),
    /// Play specific generator by name
    Generator(String),
    /// Stop playback on this track
    Stop,
    /// Keep current state (no change)
    Hold,
}

impl Default for TrackClipState {
    fn default() -> Self {
        TrackClipState::Hold
    }
}

/// Transition mode between parts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartTransition {
    /// Immediate transition (may cause abrupt changes)
    Immediate,
    /// Wait for next beat
    NextBeat,
    /// Wait for next bar
    NextBar,
    /// Wait for specified number of beats
    Beats(u32),
    /// Wait for specified number of bars
    Bars(u32),
    /// Wait for end of current clip/phrase
    EndOfPhrase,
    /// Crossfade (for audio, velocity ramp for MIDI)
    Crossfade(u32), // Duration in ticks
}

impl Default for PartTransition {
    fn default() -> Self {
        PartTransition::NextBar
    }
}

/// Macro action that can be triggered
#[derive(Debug, Clone)]
pub enum MacroAction {
    /// Set tempo
    SetTempo(f64),
    /// Adjust tempo by delta
    AdjustTempo(f64),
    /// Set parameter value
    SetParameter(String, f64),
    /// Mute track by index
    MuteTrack(usize),
    /// Unmute track by index
    UnmuteTrack(usize),
    /// Solo track by index
    SoloTrack(usize),
    /// Unsolo track by index
    UnsoloTrack(usize),
    /// Send MIDI CC
    SendCC(u8, u8, u8), // channel, cc, value
    /// Send program change
    SendProgramChange(u8, u8), // channel, program
    /// Trigger another part
    TriggerPart(String),
}

/// A part definition with track states and macros
#[derive(Debug, Clone)]
pub struct Part {
    /// Part name
    name: String,
    /// Track states (track index -> clip state)
    track_states: HashMap<usize, TrackClipState>,
    /// Track mute/solo states
    track_playback_states: HashMap<usize, TrackState>,
    /// Macros to execute when part is triggered
    macros: Vec<MacroAction>,
    /// Transition mode for this part
    transition: PartTransition,
    /// Number of bars to play (None = indefinite)
    duration_bars: Option<u32>,
    /// Part to trigger after this one
    follow_part: Option<String>,
    /// Color for UI display
    color: (u8, u8, u8),
}

impl Part {
    /// Create a new empty part
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            track_states: HashMap::new(),
            track_playback_states: HashMap::new(),
            macros: Vec::new(),
            transition: PartTransition::default(),
            duration_bars: None,
            follow_part: None,
            color: (128, 128, 128),
        }
    }

    /// Get part name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set track clip state
    pub fn set_track_state(&mut self, track: usize, state: TrackClipState) {
        self.track_states.insert(track, state);
    }

    /// Get track clip state
    pub fn track_state(&self, track: usize) -> &TrackClipState {
        self.track_states.get(&track).unwrap_or(&TrackClipState::Hold)
    }

    /// Get all track states
    pub fn track_states(&self) -> &HashMap<usize, TrackClipState> {
        &self.track_states
    }

    /// Set track playback state (mute/solo)
    pub fn set_playback_state(&mut self, track: usize, state: TrackState) {
        self.track_playback_states.insert(track, state);
    }

    /// Get track playback state
    pub fn playback_state(&self, track: usize) -> Option<TrackState> {
        self.track_playback_states.get(&track).copied()
    }

    /// Add a macro action
    pub fn add_macro(&mut self, action: MacroAction) {
        self.macros.push(action);
    }

    /// Get macros
    pub fn macros(&self) -> &[MacroAction] {
        &self.macros
    }

    /// Set transition mode
    pub fn set_transition(&mut self, transition: PartTransition) {
        self.transition = transition;
    }

    /// Get transition mode
    pub fn transition(&self) -> PartTransition {
        self.transition
    }

    /// Set duration in bars
    pub fn set_duration(&mut self, bars: Option<u32>) {
        self.duration_bars = bars;
    }

    /// Get duration in bars
    pub fn duration(&self) -> Option<u32> {
        self.duration_bars
    }

    /// Set follow part
    pub fn set_follow_part(&mut self, name: Option<String>) {
        self.follow_part = name;
    }

    /// Get follow part
    pub fn follow_part(&self) -> Option<&str> {
        self.follow_part.as_deref()
    }

    /// Set color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color = (r, g, b);
    }

    /// Get color
    pub fn color(&self) -> (u8, u8, u8) {
        self.color
    }

    /// Builder: set track clip state
    pub fn with_track(mut self, track: usize, state: TrackClipState) -> Self {
        self.set_track_state(track, state);
        self
    }

    /// Builder: set transition
    pub fn with_transition(mut self, transition: PartTransition) -> Self {
        self.transition = transition;
        self
    }

    /// Builder: set duration
    pub fn with_duration(mut self, bars: u32) -> Self {
        self.duration_bars = Some(bars);
        self
    }

    /// Builder: set follow part
    pub fn with_follow(mut self, part: impl Into<String>) -> Self {
        self.follow_part = Some(part.into());
        self
    }

    /// Builder: add macro
    pub fn with_macro(mut self, action: MacroAction) -> Self {
        self.macros.push(action);
        self
    }
}

/// Pending part transition
#[derive(Debug, Clone)]
pub struct PendingTransition {
    /// Target part name
    pub target: String,
    /// Scheduled tick for transition
    pub scheduled_tick: u64,
    /// Original transition mode
    pub transition: PartTransition,
}

/// Manages parts and transitions
pub struct PartManager {
    /// All parts by name
    parts: HashMap<String, Part>,
    /// Part order for navigation
    part_order: Vec<String>,
    /// Currently active part
    current_part: Option<String>,
    /// Pending transition (if any)
    pending: Option<PendingTransition>,
    /// Number of tracks
    track_count: usize,
}

impl PartManager {
    /// Create a new part manager
    pub fn new(track_count: usize) -> Self {
        Self {
            parts: HashMap::new(),
            part_order: Vec::new(),
            current_part: None,
            pending: None,
            track_count,
        }
    }

    /// Add a part
    pub fn add_part(&mut self, part: Part) {
        let name = part.name().to_string();
        self.parts.insert(name.clone(), part);
        if !self.part_order.contains(&name) {
            self.part_order.push(name);
        }
    }

    /// Get a part by name
    pub fn get_part(&self, name: &str) -> Option<&Part> {
        self.parts.get(name)
    }

    /// Get a mutable part by name
    pub fn get_part_mut(&mut self, name: &str) -> Option<&mut Part> {
        self.parts.get_mut(name)
    }

    /// Remove a part
    pub fn remove_part(&mut self, name: &str) -> Option<Part> {
        self.part_order.retain(|n| n != name);
        self.parts.remove(name)
    }

    /// List all part names in order
    pub fn part_names(&self) -> &[String] {
        &self.part_order
    }

    /// Get current part name
    pub fn current_part(&self) -> Option<&str> {
        self.current_part.as_deref()
    }

    /// Get current part
    pub fn current(&self) -> Option<&Part> {
        self.current_part.as_ref().and_then(|n| self.parts.get(n))
    }

    /// Trigger a part transition
    pub fn trigger_part(&mut self, name: &str, current_tick: u64, ppqn: u32, beats_per_bar: u32) -> bool {
        if let Some(part) = self.parts.get(name) {
            let scheduled_tick = self.calculate_transition_tick(
                current_tick,
                part.transition(),
                ppqn,
                beats_per_bar,
            );

            if scheduled_tick == current_tick {
                // Immediate transition
                self.current_part = Some(name.to_string());
                self.pending = None;
            } else {
                // Queue transition
                self.pending = Some(PendingTransition {
                    target: name.to_string(),
                    scheduled_tick,
                    transition: part.transition(),
                });
            }
            true
        } else {
            false
        }
    }

    /// Calculate when transition should occur
    fn calculate_transition_tick(
        &self,
        current_tick: u64,
        transition: PartTransition,
        ppqn: u32,
        beats_per_bar: u32,
    ) -> u64 {
        let ticks_per_beat = ppqn as u64;
        let ticks_per_bar = ticks_per_beat * beats_per_bar as u64;

        match transition {
            PartTransition::Immediate => current_tick,
            PartTransition::NextBeat => {
                let beat_pos = current_tick % ticks_per_beat;
                if beat_pos == 0 {
                    current_tick
                } else {
                    current_tick + (ticks_per_beat - beat_pos)
                }
            }
            PartTransition::NextBar => {
                let bar_pos = current_tick % ticks_per_bar;
                if bar_pos == 0 {
                    current_tick
                } else {
                    current_tick + (ticks_per_bar - bar_pos)
                }
            }
            PartTransition::Beats(n) => {
                current_tick + (n as u64 * ticks_per_beat)
            }
            PartTransition::Bars(n) => {
                current_tick + (n as u64 * ticks_per_bar)
            }
            PartTransition::EndOfPhrase => {
                // Default to 4 bars for phrase
                let phrase_ticks = ticks_per_bar * 4;
                let phrase_pos = current_tick % phrase_ticks;
                if phrase_pos == 0 {
                    current_tick
                } else {
                    current_tick + (phrase_ticks - phrase_pos)
                }
            }
            PartTransition::Crossfade(duration) => {
                // Start crossfade immediately
                current_tick + duration as u64
            }
        }
    }

    /// Check for pending transitions
    pub fn update(&mut self, current_tick: u64) -> Option<&Part> {
        if let Some(pending) = &self.pending {
            if current_tick >= pending.scheduled_tick {
                let target = pending.target.clone();
                self.pending = None;
                self.current_part = Some(target.clone());
                return self.parts.get(&target);
            }
        }
        None
    }

    /// Get pending transition
    pub fn pending_transition(&self) -> Option<&PendingTransition> {
        self.pending.as_ref()
    }

    /// Cancel pending transition
    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }

    /// Get next part in order
    pub fn next_part(&self) -> Option<&str> {
        if let Some(current) = &self.current_part {
            if let Some(idx) = self.part_order.iter().position(|n| n == current) {
                let next_idx = (idx + 1) % self.part_order.len();
                return Some(&self.part_order[next_idx]);
            }
        }
        self.part_order.first().map(|s| s.as_str())
    }

    /// Get previous part in order
    pub fn prev_part(&self) -> Option<&str> {
        if let Some(current) = &self.current_part {
            if let Some(idx) = self.part_order.iter().position(|n| n == current) {
                let prev_idx = if idx == 0 {
                    self.part_order.len() - 1
                } else {
                    idx - 1
                };
                return Some(&self.part_order[prev_idx]);
            }
        }
        self.part_order.last().map(|s| s.as_str())
    }

    /// Number of parts
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

impl Default for PartManager {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_creation() {
        let part = Part::new("Intro");
        assert_eq!(part.name(), "Intro");
        assert!(part.track_states().is_empty());
        assert!(part.macros().is_empty());
    }

    #[test]
    fn test_part_with_tracks() {
        let part = Part::new("Verse")
            .with_track(0, TrackClipState::Clip(0))
            .with_track(1, TrackClipState::Generator("arpeggio".into()))
            .with_track(2, TrackClipState::Stop);

        assert_eq!(part.track_state(0), &TrackClipState::Clip(0));
        assert_eq!(part.track_state(1), &TrackClipState::Generator("arpeggio".into()));
        assert_eq!(part.track_state(2), &TrackClipState::Stop);
        assert_eq!(part.track_state(3), &TrackClipState::Hold);
    }

    #[test]
    fn test_part_builder() {
        let part = Part::new("Chorus")
            .with_transition(PartTransition::NextBar)
            .with_duration(8)
            .with_follow("Bridge")
            .with_macro(MacroAction::SetTempo(130.0));

        assert_eq!(part.transition(), PartTransition::NextBar);
        assert_eq!(part.duration(), Some(8));
        assert_eq!(part.follow_part(), Some("Bridge"));
        assert_eq!(part.macros().len(), 1);
    }

    #[test]
    fn test_part_manager_add_remove() {
        let mut manager = PartManager::new(4);

        manager.add_part(Part::new("Intro"));
        manager.add_part(Part::new("Verse"));
        manager.add_part(Part::new("Chorus"));

        assert_eq!(manager.len(), 3);
        assert_eq!(manager.part_names(), &["Intro", "Verse", "Chorus"]);

        assert!(manager.get_part("Verse").is_some());
        assert!(manager.get_part("Unknown").is_none());

        manager.remove_part("Verse");
        assert_eq!(manager.len(), 2);
        assert!(manager.get_part("Verse").is_none());
    }

    #[test]
    fn test_part_immediate_transition() {
        let mut manager = PartManager::new(4);

        let part = Part::new("Part A").with_transition(PartTransition::Immediate);
        manager.add_part(part);

        assert!(manager.trigger_part("Part A", 100, 24, 4));
        assert_eq!(manager.current_part(), Some("Part A"));
        assert!(manager.pending_transition().is_none());
    }

    #[test]
    fn test_part_quantized_transition() {
        let mut manager = PartManager::new(4);

        let part = Part::new("Part B").with_transition(PartTransition::NextBar);
        manager.add_part(part);

        // Trigger mid-bar
        let ppqn = 24;
        let beats_per_bar = 4;
        let current_tick = 50; // In the middle of first bar

        assert!(manager.trigger_part("Part B", current_tick, ppqn, beats_per_bar));

        // Should be pending, not immediate
        assert!(manager.current_part().is_none());
        assert!(manager.pending_transition().is_some());

        let pending = manager.pending_transition().unwrap();
        assert_eq!(pending.target, "Part B");
        assert_eq!(pending.scheduled_tick, 96); // Next bar at tick 96

        // Update before transition
        assert!(manager.update(50).is_none());
        assert!(manager.pending_transition().is_some());

        // Update at transition
        let transitioned = manager.update(96);
        assert!(transitioned.is_some());
        assert_eq!(manager.current_part(), Some("Part B"));
        assert!(manager.pending_transition().is_none());
    }

    #[test]
    fn test_part_navigation() {
        let mut manager = PartManager::new(4);

        manager.add_part(Part::new("A"));
        manager.add_part(Part::new("B"));
        manager.add_part(Part::new("C"));

        // No current part
        assert_eq!(manager.next_part(), Some("A"));
        assert_eq!(manager.prev_part(), Some("C"));

        // Set current to B
        manager.trigger_part("B", 0, 24, 4);
        manager.update(0);

        assert_eq!(manager.current_part(), Some("B"));
        assert_eq!(manager.next_part(), Some("C"));
        assert_eq!(manager.prev_part(), Some("A"));
    }

    #[test]
    fn test_transition_timing() {
        let manager = PartManager::new(4);
        let ppqn = 24;
        let beats_per_bar = 4;

        // Test NextBeat at beat boundary
        let tick = manager.calculate_transition_tick(0, PartTransition::NextBeat, ppqn, beats_per_bar);
        assert_eq!(tick, 0);

        // Test NextBeat mid-beat
        let tick = manager.calculate_transition_tick(10, PartTransition::NextBeat, ppqn, beats_per_bar);
        assert_eq!(tick, 24);

        // Test NextBar at bar boundary
        let tick = manager.calculate_transition_tick(0, PartTransition::NextBar, ppqn, beats_per_bar);
        assert_eq!(tick, 0);

        // Test NextBar mid-bar
        let tick = manager.calculate_transition_tick(50, PartTransition::NextBar, ppqn, beats_per_bar);
        assert_eq!(tick, 96);

        // Test Beats(2)
        let tick = manager.calculate_transition_tick(100, PartTransition::Beats(2), ppqn, beats_per_bar);
        assert_eq!(tick, 148);

        // Test Bars(2)
        let tick = manager.calculate_transition_tick(100, PartTransition::Bars(2), ppqn, beats_per_bar);
        assert_eq!(tick, 292);
    }

    #[test]
    fn test_cancel_pending() {
        let mut manager = PartManager::new(4);

        manager.add_part(Part::new("Part").with_transition(PartTransition::NextBar));
        manager.trigger_part("Part", 50, 24, 4);

        assert!(manager.pending_transition().is_some());
        manager.cancel_pending();
        assert!(manager.pending_transition().is_none());
    }

    #[test]
    fn test_macro_actions() {
        let part = Part::new("Test")
            .with_macro(MacroAction::SetTempo(140.0))
            .with_macro(MacroAction::MuteTrack(0))
            .with_macro(MacroAction::SendCC(0, 7, 100));

        assert_eq!(part.macros().len(), 3);

        match &part.macros()[0] {
            MacroAction::SetTempo(t) => assert_eq!(*t, 140.0),
            _ => panic!("Expected SetTempo"),
        }
    }

    #[test]
    fn test_track_playback_states() {
        let mut part = Part::new("Test");

        part.set_playback_state(0, TrackState::Muted);
        part.set_playback_state(1, TrackState::Soloed);

        assert_eq!(part.playback_state(0), Some(TrackState::Muted));
        assert_eq!(part.playback_state(1), Some(TrackState::Soloed));
        assert_eq!(part.playback_state(2), None);
    }
}
