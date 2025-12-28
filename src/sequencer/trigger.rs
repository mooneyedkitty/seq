// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Pattern triggering with quantization and follow actions.
//!
//! Provides instant and quantized triggering of clips and patterns,
//! with a queue system and follow actions.

use std::collections::VecDeque;

use super::SequencerTiming;

/// Quantization mode for triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizeMode {
    /// Trigger immediately
    Immediate,
    /// Quantize to next tick
    Tick,
    /// Quantize to next beat
    Beat,
    /// Quantize to next bar
    Bar,
    /// Quantize to next N beats
    Beats(u8),
    /// Quantize to next N bars
    Bars(u8),
    /// Quantize to next phrase (typically 4 or 8 bars)
    Phrase,
}

impl Default for QuantizeMode {
    fn default() -> Self {
        QuantizeMode::Bar
    }
}

impl QuantizeMode {
    /// Calculate ticks until trigger based on current timing
    pub fn ticks_until(&self, timing: &SequencerTiming) -> u64 {
        match self {
            QuantizeMode::Immediate => 0,
            QuantizeMode::Tick => 1,
            QuantizeMode::Beat => timing.ticks_to_next_beat(),
            QuantizeMode::Bar => timing.ticks_to_next_bar(),
            QuantizeMode::Beats(n) => {
                let ticks_per_beat = timing.ticks_per_beat();
                let to_next_beat = timing.ticks_to_next_beat();
                to_next_beat + ((*n as u64).saturating_sub(1)) * ticks_per_beat
            }
            QuantizeMode::Bars(n) => {
                let ticks_per_bar = timing.ticks_per_bar();
                let to_next_bar = timing.ticks_to_next_bar();
                to_next_bar + ((*n as u64).saturating_sub(1)) * ticks_per_bar
            }
            QuantizeMode::Phrase => {
                // Phrase = 4 bars by default
                let ticks_per_bar = timing.ticks_per_bar();
                let phrase_length = ticks_per_bar * 4;
                let position_in_phrase = timing.position_ticks % phrase_length;
                if position_in_phrase == 0 {
                    0
                } else {
                    phrase_length - position_in_phrase
                }
            }
        }
    }
}

/// Follow action - what to do when a clip finishes
#[derive(Debug, Clone, PartialEq)]
pub enum FollowAction {
    /// Do nothing, let clip handle looping
    None,
    /// Stop the clip
    Stop,
    /// Play the same clip again
    Again,
    /// Play the next clip in sequence
    Next,
    /// Play the previous clip
    Previous,
    /// Play the first clip
    First,
    /// Play the last clip
    Last,
    /// Play a random clip
    Random,
    /// Play a specific clip by index
    Specific(usize),
    /// Choose between two actions with probability
    Either {
        action_a: Box<FollowAction>,
        action_b: Box<FollowAction>,
        probability_a: f64,
    },
}

impl Default for FollowAction {
    fn default() -> Self {
        FollowAction::None
    }
}

impl FollowAction {
    /// Create an either action
    pub fn either(a: FollowAction, b: FollowAction, prob_a: f64) -> Self {
        FollowAction::Either {
            action_a: Box::new(a),
            action_b: Box::new(b),
            probability_a: prob_a.clamp(0.0, 1.0),
        }
    }

    /// Resolve the action to a concrete clip index
    pub fn resolve(&self, current: usize, total: usize) -> Option<usize> {
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;

        match self {
            FollowAction::None => None,
            FollowAction::Stop => None,
            FollowAction::Again => Some(current),
            FollowAction::Next => {
                if current + 1 < total {
                    Some(current + 1)
                } else {
                    Some(0) // Wrap to first
                }
            }
            FollowAction::Previous => {
                if current > 0 {
                    Some(current - 1)
                } else {
                    Some(total.saturating_sub(1)) // Wrap to last
                }
            }
            FollowAction::First => Some(0),
            FollowAction::Last => Some(total.saturating_sub(1)),
            FollowAction::Random => {
                if total > 0 {
                    let mut rng = StdRng::from_entropy();
                    Some(rng.gen_range(0..total))
                } else {
                    None
                }
            }
            FollowAction::Specific(idx) => {
                if *idx < total {
                    Some(*idx)
                } else {
                    None
                }
            }
            FollowAction::Either {
                action_a,
                action_b,
                probability_a,
            } => {
                let mut rng = StdRng::from_entropy();
                if rng.gen::<f64>() < *probability_a {
                    action_a.resolve(current, total)
                } else {
                    action_b.resolve(current, total)
                }
            }
        }
    }
}

/// A queued trigger action
#[derive(Debug, Clone)]
pub struct QueuedTrigger {
    /// Track index
    pub track_index: usize,
    /// Clip index (None = stop)
    pub clip_index: Option<usize>,
    /// Tick when trigger should fire
    pub trigger_tick: u64,
    /// Follow action after clip completes
    pub follow_action: FollowAction,
    /// Optional description
    pub description: String,
}

impl QueuedTrigger {
    /// Create a new queued trigger
    pub fn new(
        track_index: usize,
        clip_index: Option<usize>,
        trigger_tick: u64,
    ) -> Self {
        Self {
            track_index,
            clip_index,
            trigger_tick,
            follow_action: FollowAction::None,
            description: String::new(),
        }
    }

    /// Set follow action
    pub fn with_follow_action(mut self, action: FollowAction) -> Self {
        self.follow_action = action;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// Queue of pending triggers
pub struct TriggerQueue {
    /// Pending triggers
    queue: VecDeque<QueuedTrigger>,
    /// Default quantization mode
    default_quantize: QuantizeMode,
    /// Phrase length in bars (for phrase quantization)
    phrase_bars: u8,
}

impl TriggerQueue {
    /// Create a new trigger queue
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            default_quantize: QuantizeMode::Bar,
            phrase_bars: 4,
        }
    }

    /// Set default quantization mode
    pub fn set_default_quantize(&mut self, mode: QuantizeMode) {
        self.default_quantize = mode;
    }

    /// Get default quantization mode
    pub fn default_quantize(&self) -> QuantizeMode {
        self.default_quantize
    }

    /// Set phrase length in bars
    pub fn set_phrase_bars(&mut self, bars: u8) {
        self.phrase_bars = bars.max(1);
    }

    /// Queue a trigger with default quantization
    pub fn queue(
        &mut self,
        track_index: usize,
        clip_index: Option<usize>,
        timing: &SequencerTiming,
    ) {
        self.queue_with_quantize(track_index, clip_index, timing, self.default_quantize);
    }

    /// Queue a trigger with specific quantization
    pub fn queue_with_quantize(
        &mut self,
        track_index: usize,
        clip_index: Option<usize>,
        timing: &SequencerTiming,
        quantize: QuantizeMode,
    ) {
        let trigger_tick = timing.position_ticks + quantize.ticks_until(timing);
        let trigger = QueuedTrigger::new(track_index, clip_index, trigger_tick);
        self.insert_sorted(trigger);
    }

    /// Queue an immediate trigger
    pub fn queue_immediate(
        &mut self,
        track_index: usize,
        clip_index: Option<usize>,
        timing: &SequencerTiming,
    ) {
        self.queue_with_quantize(track_index, clip_index, timing, QuantizeMode::Immediate);
    }

    /// Queue a trigger with follow action
    pub fn queue_with_follow(
        &mut self,
        track_index: usize,
        clip_index: Option<usize>,
        timing: &SequencerTiming,
        follow_action: FollowAction,
    ) {
        let trigger_tick = timing.position_ticks + self.default_quantize.ticks_until(timing);
        let trigger = QueuedTrigger::new(track_index, clip_index, trigger_tick)
            .with_follow_action(follow_action);
        self.insert_sorted(trigger);
    }

    /// Insert trigger maintaining time order
    fn insert_sorted(&mut self, trigger: QueuedTrigger) {
        // Find insertion point to maintain sorted order
        let pos = self.queue
            .iter()
            .position(|t| t.trigger_tick > trigger.trigger_tick)
            .unwrap_or(self.queue.len());

        self.queue.insert(pos, trigger);
    }

    /// Get triggers due at or before the given tick
    pub fn poll(&mut self, current_tick: u64) -> Vec<QueuedTrigger> {
        let mut triggered = Vec::new();

        while let Some(trigger) = self.queue.front() {
            if trigger.trigger_tick <= current_tick {
                triggered.push(self.queue.pop_front().unwrap());
            } else {
                break;
            }
        }

        triggered
    }

    /// Peek at next trigger without removing
    pub fn peek(&self) -> Option<&QueuedTrigger> {
        self.queue.front()
    }

    /// Get all pending triggers for a track
    pub fn pending_for_track(&self, track_index: usize) -> Vec<&QueuedTrigger> {
        self.queue
            .iter()
            .filter(|t| t.track_index == track_index)
            .collect()
    }

    /// Cancel all pending triggers for a track
    pub fn cancel_for_track(&mut self, track_index: usize) {
        self.queue.retain(|t| t.track_index != track_index);
    }

    /// Cancel all pending triggers
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Get number of pending triggers
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get iterator over pending triggers
    pub fn iter(&self) -> impl Iterator<Item = &QueuedTrigger> {
        self.queue.iter()
    }
}

impl Default for TriggerQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene - a collection of clip states for all tracks
#[derive(Debug, Clone)]
pub struct Scene {
    /// Scene name
    pub name: String,
    /// Clip indices for each track (None = stop track)
    pub clips: Vec<Option<usize>>,
    /// Follow action for scene
    pub follow_action: FollowAction,
    /// Quantization for scene trigger
    pub quantize: QuantizeMode,
}

impl Scene {
    /// Create a new scene
    pub fn new(name: impl Into<String>, track_count: usize) -> Self {
        Self {
            name: name.into(),
            clips: vec![None; track_count],
            follow_action: FollowAction::None,
            quantize: QuantizeMode::Bar,
        }
    }

    /// Set clip for a track
    pub fn set_clip(&mut self, track: usize, clip: Option<usize>) {
        if track < self.clips.len() {
            self.clips[track] = clip;
        }
    }

    /// Get clip for a track
    pub fn clip(&self, track: usize) -> Option<usize> {
        self.clips.get(track).copied().flatten()
    }

    /// Queue this scene's clips
    pub fn queue_all(&self, queue: &mut TriggerQueue, timing: &SequencerTiming) {
        for (track_index, clip) in self.clips.iter().enumerate() {
            queue.queue_with_quantize(track_index, *clip, timing, self.quantize);
        }
    }
}

/// Manager for scenes
pub struct SceneManager {
    scenes: Vec<Scene>,
    current_scene: Option<usize>,
}

impl SceneManager {
    /// Create a new scene manager
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            current_scene: None,
        }
    }

    /// Add a scene
    pub fn add_scene(&mut self, scene: Scene) -> usize {
        self.scenes.push(scene);
        self.scenes.len() - 1
    }

    /// Get a scene by index
    pub fn scene(&self, index: usize) -> Option<&Scene> {
        self.scenes.get(index)
    }

    /// Get a mutable scene by index
    pub fn scene_mut(&mut self, index: usize) -> Option<&mut Scene> {
        self.scenes.get_mut(index)
    }

    /// Get number of scenes
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// Get current scene index
    pub fn current_scene(&self) -> Option<usize> {
        self.current_scene
    }

    /// Trigger a scene
    pub fn trigger_scene(
        &mut self,
        index: usize,
        queue: &mut TriggerQueue,
        timing: &SequencerTiming,
    ) {
        if let Some(scene) = self.scenes.get(index) {
            scene.queue_all(queue, timing);
            self.current_scene = Some(index);
        }
    }

    /// Trigger next scene
    pub fn trigger_next(&mut self, queue: &mut TriggerQueue, timing: &SequencerTiming) {
        let next = match self.current_scene {
            Some(idx) => (idx + 1) % self.scenes.len(),
            None => 0,
        };
        self.trigger_scene(next, queue, timing);
    }

    /// Trigger previous scene
    pub fn trigger_previous(&mut self, queue: &mut TriggerQueue, timing: &SequencerTiming) {
        let prev = match self.current_scene {
            Some(idx) if idx > 0 => idx - 1,
            _ => self.scenes.len().saturating_sub(1),
        };
        self.trigger_scene(prev, queue, timing);
    }

    /// Iterate over scenes
    pub fn iter(&self) -> impl Iterator<Item = &Scene> {
        self.scenes.iter()
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_timing() -> SequencerTiming {
        SequencerTiming {
            tempo: 120.0,
            ppqn: 24,
            position_ticks: 0,
            beats_per_bar: 4,
            beat_unit: 4,
        }
    }

    #[test]
    fn test_quantize_immediate() {
        let timing = test_timing();
        assert_eq!(QuantizeMode::Immediate.ticks_until(&timing), 0);
    }

    #[test]
    fn test_quantize_beat() {
        let mut timing = test_timing();
        timing.position_ticks = 10; // 10 ticks into first beat

        let ticks = QuantizeMode::Beat.ticks_until(&timing);
        assert_eq!(ticks, 14); // 24 - 10 = 14 ticks to next beat
    }

    #[test]
    fn test_quantize_bar() {
        let mut timing = test_timing();
        timing.position_ticks = 50; // 50 ticks into first bar

        let ticks = QuantizeMode::Bar.ticks_until(&timing);
        assert_eq!(ticks, 46); // 96 - 50 = 46 ticks to next bar
    }

    #[test]
    fn test_quantize_at_boundary() {
        let timing = test_timing();
        assert_eq!(QuantizeMode::Beat.ticks_until(&timing), 0);
        assert_eq!(QuantizeMode::Bar.ticks_until(&timing), 0);
    }

    #[test]
    fn test_follow_action_next() {
        let action = FollowAction::Next;

        assert_eq!(action.resolve(0, 5), Some(1));
        assert_eq!(action.resolve(4, 5), Some(0)); // Wrap
    }

    #[test]
    fn test_follow_action_previous() {
        let action = FollowAction::Previous;

        assert_eq!(action.resolve(2, 5), Some(1));
        assert_eq!(action.resolve(0, 5), Some(4)); // Wrap
    }

    #[test]
    fn test_follow_action_specific() {
        let action = FollowAction::Specific(3);

        assert_eq!(action.resolve(0, 5), Some(3));
        assert_eq!(action.resolve(0, 2), None); // Out of range
    }

    #[test]
    fn test_trigger_queue() {
        let mut queue = TriggerQueue::new();
        let timing = test_timing();

        queue.queue(0, Some(1), &timing);
        queue.queue(1, Some(2), &timing);

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_trigger_queue_poll() {
        let mut queue = TriggerQueue::new();
        let mut timing = test_timing();
        timing.position_ticks = 10; // Not at a bar boundary

        // Queue trigger at next bar (tick 96)
        queue.queue(0, Some(0), &timing);

        // Not yet triggered at tick 50
        let triggered = queue.poll(50);
        assert!(triggered.is_empty());

        // Triggered at tick 100 (past bar boundary at 96)
        let triggered = queue.poll(100);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].track_index, 0);
    }

    #[test]
    fn test_trigger_queue_immediate() {
        let mut queue = TriggerQueue::new();
        let timing = test_timing();

        queue.queue_immediate(0, Some(0), &timing);

        // Should trigger immediately
        let triggered = queue.poll(0);
        assert_eq!(triggered.len(), 1);
    }

    #[test]
    fn test_cancel_for_track() {
        let mut queue = TriggerQueue::new();
        let timing = test_timing();

        queue.queue(0, Some(0), &timing);
        queue.queue(1, Some(1), &timing);
        queue.queue(0, Some(2), &timing);

        queue.cancel_for_track(0);

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek().unwrap().track_index, 1);
    }

    #[test]
    fn test_scene() {
        let mut scene = Scene::new("Intro", 4);
        scene.set_clip(0, Some(0));
        scene.set_clip(1, Some(1));
        scene.set_clip(2, None); // Stop track 2

        assert_eq!(scene.clip(0), Some(0));
        assert_eq!(scene.clip(1), Some(1));
        assert_eq!(scene.clip(2), None);
    }

    #[test]
    fn test_scene_manager() {
        let mut manager = SceneManager::new();
        let mut queue = TriggerQueue::new();
        let timing = test_timing();

        let mut scene1 = Scene::new("Intro", 2);
        scene1.set_clip(0, Some(0));

        let mut scene2 = Scene::new("Verse", 2);
        scene2.set_clip(0, Some(1));

        manager.add_scene(scene1);
        manager.add_scene(scene2);

        manager.trigger_scene(0, &mut queue, &timing);
        assert_eq!(manager.current_scene(), Some(0));
        assert_eq!(queue.len(), 2); // One trigger per track

        queue.clear();
        manager.trigger_next(&mut queue, &timing);
        assert_eq!(manager.current_scene(), Some(1));
    }

    #[test]
    fn test_quantize_phrase() {
        let mut timing = test_timing();
        timing.position_ticks = 100; // Past first bar

        // Phrase = 4 bars = 384 ticks
        let ticks = QuantizeMode::Phrase.ticks_until(&timing);
        assert_eq!(ticks, 284); // 384 - 100
    }
}
