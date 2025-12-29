// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Scene system for track state snapshots.
//!
//! Scenes represent a horizontal row in a track × scene matrix,
//! enabling coordinated triggering of multiple track clips.

use std::collections::HashMap;

use crate::sequencer::trigger::FollowAction;

/// A slot in the scene matrix (track × scene intersection)
#[derive(Debug, Clone, PartialEq)]
pub enum SceneSlot {
    /// Empty slot
    Empty,
    /// Clip index
    Clip(usize),
    /// Generator name
    Generator(String),
    /// Stop playback
    Stop,
    /// No change (keep current)
    Hold,
}

impl Default for SceneSlot {
    fn default() -> Self {
        SceneSlot::Empty
    }
}

/// Scene launch quantization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneLaunchMode {
    /// Launch immediately
    Immediate,
    /// Quantize to beat
    Beat,
    /// Quantize to bar
    Bar,
    /// Quantize to specific beat count
    Beats(u32),
    /// Quantize to specific bar count
    Bars(u32),
}

impl Default for SceneLaunchMode {
    fn default() -> Self {
        SceneLaunchMode::Bar
    }
}

/// A scene (horizontal row of clips/slots)
#[derive(Debug, Clone)]
pub struct Scene {
    /// Scene name
    name: String,
    /// Slots indexed by track
    slots: HashMap<usize, SceneSlot>,
    /// Launch mode for this scene
    launch_mode: SceneLaunchMode,
    /// Follow action after scene plays
    follow_action: FollowAction,
    /// Duration in bars before follow action (None = loop indefinitely)
    follow_after_bars: Option<u32>,
    /// Tempo for this scene (None = keep current)
    tempo: Option<f64>,
    /// Color for UI
    color: (u8, u8, u8),
}

impl Scene {
    /// Create a new scene
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slots: HashMap::new(),
            launch_mode: SceneLaunchMode::default(),
            follow_action: FollowAction::None,
            follow_after_bars: None,
            tempo: None,
            color: (100, 100, 100),
        }
    }

    /// Get scene name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set scene name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Set slot for track
    pub fn set_slot(&mut self, track: usize, slot: SceneSlot) {
        self.slots.insert(track, slot);
    }

    /// Get slot for track
    pub fn slot(&self, track: usize) -> &SceneSlot {
        self.slots.get(&track).unwrap_or(&SceneSlot::Empty)
    }

    /// Get all slots
    pub fn slots(&self) -> &HashMap<usize, SceneSlot> {
        &self.slots
    }

    /// Set launch mode
    pub fn set_launch_mode(&mut self, mode: SceneLaunchMode) {
        self.launch_mode = mode;
    }

    /// Get launch mode
    pub fn launch_mode(&self) -> SceneLaunchMode {
        self.launch_mode
    }

    /// Set follow action
    pub fn set_follow_action(&mut self, action: FollowAction, after_bars: Option<u32>) {
        self.follow_action = action;
        self.follow_after_bars = after_bars;
    }

    /// Get follow action
    pub fn follow_action(&self) -> FollowAction {
        self.follow_action.clone()
    }

    /// Get follow after bars
    pub fn follow_after_bars(&self) -> Option<u32> {
        self.follow_after_bars
    }

    /// Set tempo
    pub fn set_tempo(&mut self, tempo: Option<f64>) {
        self.tempo = tempo;
    }

    /// Get tempo
    pub fn tempo(&self) -> Option<f64> {
        self.tempo
    }

    /// Set color
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color = (r, g, b);
    }

    /// Get color
    pub fn color(&self) -> (u8, u8, u8) {
        self.color
    }

    /// Builder: add slot
    pub fn with_slot(mut self, track: usize, slot: SceneSlot) -> Self {
        self.set_slot(track, slot);
        self
    }

    /// Builder: set launch mode
    pub fn with_launch_mode(mut self, mode: SceneLaunchMode) -> Self {
        self.launch_mode = mode;
        self
    }

    /// Builder: set follow action
    pub fn with_follow(mut self, action: FollowAction, after_bars: Option<u32>) -> Self {
        self.follow_action = action;
        self.follow_after_bars = after_bars;
        self
    }

    /// Builder: set tempo
    pub fn with_tempo(mut self, tempo: f64) -> Self {
        self.tempo = Some(tempo);
        self
    }

    /// Number of non-empty slots
    pub fn slot_count(&self) -> usize {
        self.slots.values().filter(|s| **s != SceneSlot::Empty).count()
    }

    /// Check if scene has any content
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() || self.slots.values().all(|s| *s == SceneSlot::Empty)
    }
}

/// Pending scene launch
#[derive(Debug, Clone)]
pub struct PendingSceneLaunch {
    /// Scene index
    pub scene_index: usize,
    /// Scheduled tick
    pub scheduled_tick: u64,
}

/// Manages scenes in a matrix layout
pub struct SceneManager {
    /// All scenes
    scenes: Vec<Scene>,
    /// Number of tracks
    track_count: usize,
    /// Currently playing scene index
    current_scene: Option<usize>,
    /// Pending scene launch
    pending: Option<PendingSceneLaunch>,
    /// Tick when current scene started
    scene_start_tick: u64,
}

impl SceneManager {
    /// Create a new scene manager
    pub fn new(track_count: usize) -> Self {
        Self {
            scenes: Vec::new(),
            track_count,
            current_scene: None,
            pending: None,
            scene_start_tick: 0,
        }
    }

    /// Add a scene
    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }

    /// Insert scene at index
    pub fn insert_scene(&mut self, index: usize, scene: Scene) {
        if index <= self.scenes.len() {
            self.scenes.insert(index, scene);
        } else {
            self.scenes.push(scene);
        }
    }

    /// Remove scene at index
    pub fn remove_scene(&mut self, index: usize) -> Option<Scene> {
        if index < self.scenes.len() {
            Some(self.scenes.remove(index))
        } else {
            None
        }
    }

    /// Get scene at index
    pub fn get_scene(&self, index: usize) -> Option<&Scene> {
        self.scenes.get(index)
    }

    /// Get mutable scene at index
    pub fn get_scene_mut(&mut self, index: usize) -> Option<&mut Scene> {
        self.scenes.get_mut(index)
    }

    /// Get all scenes
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Number of scenes
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// Get current scene index
    pub fn current_scene(&self) -> Option<usize> {
        self.current_scene
    }

    /// Get current scene
    pub fn current(&self) -> Option<&Scene> {
        self.current_scene.and_then(|i| self.scenes.get(i))
    }

    /// Launch a scene
    pub fn launch_scene(&mut self, index: usize, current_tick: u64, ppqn: u32, beats_per_bar: u32) -> bool {
        if let Some(scene) = self.scenes.get(index) {
            let scheduled_tick = self.calculate_launch_tick(
                current_tick,
                scene.launch_mode(),
                ppqn,
                beats_per_bar,
            );

            if scheduled_tick == current_tick {
                self.current_scene = Some(index);
                self.scene_start_tick = current_tick;
                self.pending = None;
            } else {
                self.pending = Some(PendingSceneLaunch {
                    scene_index: index,
                    scheduled_tick,
                });
            }
            true
        } else {
            false
        }
    }

    /// Calculate launch tick based on mode
    fn calculate_launch_tick(
        &self,
        current_tick: u64,
        mode: SceneLaunchMode,
        ppqn: u32,
        beats_per_bar: u32,
    ) -> u64 {
        let ticks_per_beat = ppqn as u64;
        let ticks_per_bar = ticks_per_beat * beats_per_bar as u64;

        match mode {
            SceneLaunchMode::Immediate => current_tick,
            SceneLaunchMode::Beat => {
                let beat_pos = current_tick % ticks_per_beat;
                if beat_pos == 0 {
                    current_tick
                } else {
                    current_tick + (ticks_per_beat - beat_pos)
                }
            }
            SceneLaunchMode::Bar => {
                let bar_pos = current_tick % ticks_per_bar;
                if bar_pos == 0 {
                    current_tick
                } else {
                    current_tick + (ticks_per_bar - bar_pos)
                }
            }
            SceneLaunchMode::Beats(n) => {
                current_tick + (n as u64 * ticks_per_beat)
            }
            SceneLaunchMode::Bars(n) => {
                current_tick + (n as u64 * ticks_per_bar)
            }
        }
    }

    /// Update scene manager, return scene if transition occurred
    pub fn update(&mut self, current_tick: u64) -> Option<&Scene> {
        // Check for pending launch
        if let Some(pending) = &self.pending {
            if current_tick >= pending.scheduled_tick {
                let index = pending.scene_index;
                self.pending = None;
                self.current_scene = Some(index);
                self.scene_start_tick = current_tick;
                return self.scenes.get(index);
            }
        }
        None
    }

    /// Check if follow action should trigger
    pub fn check_follow_action(&self, current_tick: u64, ppqn: u32, beats_per_bar: u32) -> Option<FollowAction> {
        if let Some(scene) = self.current() {
            if let Some(bars) = scene.follow_after_bars() {
                let ticks_per_bar = ppqn as u64 * beats_per_bar as u64;
                let elapsed = current_tick - self.scene_start_tick;
                let target_ticks = bars as u64 * ticks_per_bar;

                if elapsed >= target_ticks {
                    return Some(scene.follow_action());
                }
            }
        }
        None
    }

    /// Get pending scene launch
    pub fn pending_launch(&self) -> Option<&PendingSceneLaunch> {
        self.pending.as_ref()
    }

    /// Cancel pending launch
    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }

    /// Stop current scene
    pub fn stop(&mut self) {
        self.current_scene = None;
        self.pending = None;
    }

    /// Get slot at track × scene intersection
    pub fn get_slot(&self, track: usize, scene: usize) -> Option<&SceneSlot> {
        self.scenes.get(scene).map(|s| s.slot(track))
    }

    /// Set slot at track × scene intersection
    pub fn set_slot(&mut self, track: usize, scene: usize, slot: SceneSlot) {
        if let Some(s) = self.scenes.get_mut(scene) {
            s.set_slot(track, slot);
        }
    }

    /// Get track count
    pub fn track_count(&self) -> usize {
        self.track_count
    }

    /// Set track count (clips matrix with empty slots if needed)
    pub fn set_track_count(&mut self, count: usize) {
        self.track_count = count;
    }

    /// Launch next scene
    pub fn launch_next(&mut self, current_tick: u64, ppqn: u32, beats_per_bar: u32) -> bool {
        let next = match self.current_scene {
            Some(i) => (i + 1) % self.scenes.len(),
            None => 0,
        };
        if !self.scenes.is_empty() {
            self.launch_scene(next, current_tick, ppqn, beats_per_bar)
        } else {
            false
        }
    }

    /// Launch previous scene
    pub fn launch_prev(&mut self, current_tick: u64, ppqn: u32, beats_per_bar: u32) -> bool {
        let prev = match self.current_scene {
            Some(i) => {
                if i == 0 {
                    self.scenes.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.scenes.len().saturating_sub(1),
        };
        if !self.scenes.is_empty() {
            self.launch_scene(prev, current_tick, ppqn, beats_per_bar)
        } else {
            false
        }
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new("Scene 1");
        assert_eq!(scene.name(), "Scene 1");
        assert!(scene.is_empty());
    }

    #[test]
    fn test_scene_slots() {
        let scene = Scene::new("Test")
            .with_slot(0, SceneSlot::Clip(0))
            .with_slot(1, SceneSlot::Generator("arp".into()))
            .with_slot(2, SceneSlot::Stop);

        assert_eq!(scene.slot(0), &SceneSlot::Clip(0));
        assert_eq!(scene.slot(1), &SceneSlot::Generator("arp".into()));
        assert_eq!(scene.slot(2), &SceneSlot::Stop);
        assert_eq!(scene.slot(3), &SceneSlot::Empty);

        assert!(!scene.is_empty());
        assert_eq!(scene.slot_count(), 3);
    }

    #[test]
    fn test_scene_follow_action() {
        let scene = Scene::new("Verse")
            .with_follow(FollowAction::Next, Some(4));

        assert_eq!(scene.follow_action(), FollowAction::Next);
        assert_eq!(scene.follow_after_bars(), Some(4));
    }

    #[test]
    fn test_scene_manager_add_remove() {
        let mut manager = SceneManager::new(4);

        manager.add_scene(Scene::new("A"));
        manager.add_scene(Scene::new("B"));
        manager.add_scene(Scene::new("C"));

        assert_eq!(manager.scene_count(), 3);
        assert_eq!(manager.get_scene(1).unwrap().name(), "B");

        manager.remove_scene(1);
        assert_eq!(manager.scene_count(), 2);
        assert_eq!(manager.get_scene(1).unwrap().name(), "C");
    }

    #[test]
    fn test_scene_immediate_launch() {
        let mut manager = SceneManager::new(4);
        manager.add_scene(Scene::new("A").with_launch_mode(SceneLaunchMode::Immediate));

        assert!(manager.launch_scene(0, 50, 24, 4));
        assert_eq!(manager.current_scene(), Some(0));
        assert!(manager.pending_launch().is_none());
    }

    #[test]
    fn test_scene_quantized_launch() {
        let mut manager = SceneManager::new(4);
        manager.add_scene(Scene::new("A").with_launch_mode(SceneLaunchMode::Bar));

        // Launch mid-bar
        assert!(manager.launch_scene(0, 50, 24, 4));
        assert!(manager.current_scene().is_none());
        assert!(manager.pending_launch().is_some());

        let pending = manager.pending_launch().unwrap();
        assert_eq!(pending.scene_index, 0);
        assert_eq!(pending.scheduled_tick, 96);

        // Update at transition
        let transitioned = manager.update(96);
        assert!(transitioned.is_some());
        assert_eq!(manager.current_scene(), Some(0));
    }

    #[test]
    fn test_scene_navigation() {
        let mut manager = SceneManager::new(4);
        manager.add_scene(Scene::new("A").with_launch_mode(SceneLaunchMode::Immediate));
        manager.add_scene(Scene::new("B").with_launch_mode(SceneLaunchMode::Immediate));
        manager.add_scene(Scene::new("C").with_launch_mode(SceneLaunchMode::Immediate));

        manager.launch_scene(0, 0, 24, 4);
        assert_eq!(manager.current_scene(), Some(0));

        manager.launch_next(0, 24, 4);
        assert_eq!(manager.current_scene(), Some(1));

        manager.launch_next(0, 24, 4);
        assert_eq!(manager.current_scene(), Some(2));

        manager.launch_next(0, 24, 4);
        assert_eq!(manager.current_scene(), Some(0)); // Wraps

        manager.launch_prev(0, 24, 4);
        assert_eq!(manager.current_scene(), Some(2)); // Wraps back
    }

    #[test]
    fn test_scene_follow_action_timing() {
        let mut manager = SceneManager::new(4);

        let scene = Scene::new("Test")
            .with_launch_mode(SceneLaunchMode::Immediate)
            .with_follow(FollowAction::Next, Some(2));

        manager.add_scene(scene);
        manager.launch_scene(0, 0, 24, 4);

        let ppqn = 24;
        let beats_per_bar = 4;
        let ticks_per_bar = ppqn * beats_per_bar;

        // Before 2 bars
        assert!(manager.check_follow_action(ticks_per_bar as u64, ppqn, beats_per_bar as u32).is_none());

        // At 2 bars
        let action = manager.check_follow_action((ticks_per_bar * 2) as u64, ppqn, beats_per_bar as u32);
        assert_eq!(action, Some(FollowAction::Next));
    }

    #[test]
    fn test_matrix_access() {
        let mut manager = SceneManager::new(4);
        manager.add_scene(Scene::new("Scene 0"));
        manager.add_scene(Scene::new("Scene 1"));

        manager.set_slot(0, 0, SceneSlot::Clip(0));
        manager.set_slot(1, 0, SceneSlot::Clip(1));
        manager.set_slot(0, 1, SceneSlot::Clip(2));

        assert_eq!(manager.get_slot(0, 0), Some(&SceneSlot::Clip(0)));
        assert_eq!(manager.get_slot(1, 0), Some(&SceneSlot::Clip(1)));
        assert_eq!(manager.get_slot(0, 1), Some(&SceneSlot::Clip(2)));
        assert_eq!(manager.get_slot(2, 0), Some(&SceneSlot::Empty));
    }

    #[test]
    fn test_stop_and_cancel() {
        let mut manager = SceneManager::new(4);
        manager.add_scene(Scene::new("A").with_launch_mode(SceneLaunchMode::Bar));

        // Launch with pending
        manager.launch_scene(0, 50, 24, 4);
        assert!(manager.pending_launch().is_some());

        // Cancel
        manager.cancel_pending();
        assert!(manager.pending_launch().is_none());

        // Launch and complete
        manager.launch_scene(0, 0, 24, 4);
        manager.update(96);
        assert_eq!(manager.current_scene(), Some(0));

        // Stop
        manager.stop();
        assert!(manager.current_scene().is_none());
    }
}
