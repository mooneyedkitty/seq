// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Control system for keyboard and MIDI controller input.
//!
//! This module provides:
//! - Keyboard shortcut handling
//! - MIDI controller mapping with learn mode
//! - Parameter registry with smoothing

pub mod keyboard;
pub mod midi_map;
pub mod params;

pub use keyboard::{KeyBinding, KeyboardController, Shortcut};
pub use midi_map::{MidiBinding, MidiController, MidiMapConfig};
pub use params::{Parameter, ParameterRegistry, ParameterValue};

use std::sync::{Arc, Mutex};

/// Action that can be triggered by controls
#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    /// No action
    None,

    // Transport
    /// Toggle play/pause
    TogglePlay,
    /// Stop playback
    Stop,
    /// Start playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle record
    ToggleRecord,

    // Tempo
    /// Set tempo to specific value
    SetTempo(f64),
    /// Adjust tempo by delta
    AdjustTempo(f64),
    /// Nudge tempo temporarily
    NudgeTempo(f64),
    /// Tap tempo
    TapTempo,

    // Track control
    /// Toggle track mute
    ToggleMute(usize),
    /// Toggle track solo
    ToggleSolo(usize),
    /// Set track volume
    SetTrackVolume(usize, f64),
    /// Select track
    SelectTrack(usize),

    // Clip/Scene
    /// Trigger clip on track
    TriggerClip(usize, usize),
    /// Stop clip on track
    StopClip(usize),
    /// Trigger scene
    TriggerScene(usize),
    /// Stop all clips
    StopAllClips,

    // Parameters
    /// Set parameter value
    SetParameter(String, f64),
    /// Adjust parameter by delta
    AdjustParameter(String, f64),

    // UI
    /// Toggle help display
    ToggleHelp,
    /// Toggle MIDI learn mode
    ToggleLearn,
    /// Quit application
    Quit,

    // Navigation
    /// Move selection up
    NavigateUp,
    /// Move selection down
    NavigateDown,
    /// Move selection left
    NavigateLeft,
    /// Move selection right
    NavigateRight,
    /// Confirm/Enter
    Confirm,
    /// Cancel/Back
    Cancel,
}

impl ControlAction {
    /// Check if this is a transport action
    pub fn is_transport(&self) -> bool {
        matches!(
            self,
            ControlAction::TogglePlay
                | ControlAction::Stop
                | ControlAction::Play
                | ControlAction::Pause
                | ControlAction::ToggleRecord
        )
    }

    /// Check if this is a tempo action
    pub fn is_tempo(&self) -> bool {
        matches!(
            self,
            ControlAction::SetTempo(_)
                | ControlAction::AdjustTempo(_)
                | ControlAction::NudgeTempo(_)
                | ControlAction::TapTempo
        )
    }

    /// Check if this is a track action
    pub fn is_track(&self) -> bool {
        matches!(
            self,
            ControlAction::ToggleMute(_)
                | ControlAction::ToggleSolo(_)
                | ControlAction::SetTrackVolume(_, _)
                | ControlAction::SelectTrack(_)
        )
    }
}

/// Controller manager combining keyboard and MIDI
pub struct ControllerManager {
    keyboard: KeyboardController,
    midi: MidiController,
    params: Arc<Mutex<ParameterRegistry>>,
    learn_mode: bool,
    pending_learn: Option<String>,
}

impl ControllerManager {
    /// Create a new controller manager
    pub fn new() -> Self {
        Self {
            keyboard: KeyboardController::with_defaults(),
            midi: MidiController::new(),
            params: Arc::new(Mutex::new(ParameterRegistry::new())),
            learn_mode: false,
            pending_learn: None,
        }
    }

    /// Get keyboard controller
    pub fn keyboard(&self) -> &KeyboardController {
        &self.keyboard
    }

    /// Get mutable keyboard controller
    pub fn keyboard_mut(&mut self) -> &mut KeyboardController {
        &mut self.keyboard
    }

    /// Get MIDI controller
    pub fn midi(&self) -> &MidiController {
        &self.midi
    }

    /// Get mutable MIDI controller
    pub fn midi_mut(&mut self) -> &mut MidiController {
        &mut self.midi
    }

    /// Get parameter registry
    pub fn params(&self) -> Arc<Mutex<ParameterRegistry>> {
        Arc::clone(&self.params)
    }

    /// Toggle learn mode
    pub fn toggle_learn(&mut self) {
        self.learn_mode = !self.learn_mode;
        if !self.learn_mode {
            self.pending_learn = None;
        }
    }

    /// Check if in learn mode
    pub fn is_learning(&self) -> bool {
        self.learn_mode
    }

    /// Start learning a parameter
    pub fn start_learn(&mut self, param_name: impl Into<String>) {
        self.learn_mode = true;
        self.pending_learn = Some(param_name.into());
    }

    /// Complete learning with MIDI binding
    pub fn complete_learn(&mut self, binding: MidiBinding) -> bool {
        if let Some(ref param) = self.pending_learn {
            self.midi.add_binding(binding, ControlAction::SetParameter(param.clone(), 0.0));
            self.learn_mode = false;
            self.pending_learn = None;
            true
        } else {
            false
        }
    }

    /// Process a MIDI message
    pub fn process_midi(&self, channel: u8, status: u8, data1: u8, data2: u8) -> Option<ControlAction> {
        self.midi.process_message(channel, status, data1, data2)
    }

    /// Update parameter smoothing (call each frame)
    pub fn update(&mut self, delta_time: f64) {
        if let Ok(mut params) = self.params.lock() {
            params.update(delta_time);
        }
    }
}

impl Default for ControllerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_action_categories() {
        assert!(ControlAction::TogglePlay.is_transport());
        assert!(ControlAction::Stop.is_transport());
        assert!(!ControlAction::SetTempo(120.0).is_transport());

        assert!(ControlAction::SetTempo(120.0).is_tempo());
        assert!(ControlAction::TapTempo.is_tempo());
        assert!(!ControlAction::TogglePlay.is_tempo());

        assert!(ControlAction::ToggleMute(0).is_track());
        assert!(ControlAction::SelectTrack(1).is_track());
        assert!(!ControlAction::Stop.is_track());
    }

    #[test]
    fn test_controller_manager() {
        let mut manager = ControllerManager::new();

        assert!(!manager.is_learning());
        manager.toggle_learn();
        assert!(manager.is_learning());
        manager.toggle_learn();
        assert!(!manager.is_learning());
    }

    #[test]
    fn test_learn_mode() {
        let mut manager = ControllerManager::new();

        manager.start_learn("tempo");
        assert!(manager.is_learning());
        assert_eq!(manager.pending_learn, Some("tempo".to_string()));

        let binding = MidiBinding::cc(0, 1);
        assert!(manager.complete_learn(binding));
        assert!(!manager.is_learning());
    }
}
