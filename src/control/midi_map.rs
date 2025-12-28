// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI controller mapping with learn mode.
//!
//! Provides configurable MIDI bindings for notes, CCs, and program changes
//! with support for relative encoders and multiple mapping layers.

use std::collections::HashMap;

use super::ControlAction;

/// MIDI message status bytes
pub mod status {
    pub const NOTE_OFF: u8 = 0x80;
    pub const NOTE_ON: u8 = 0x90;
    pub const POLY_PRESSURE: u8 = 0xA0;
    pub const CONTROL_CHANGE: u8 = 0xB0;
    pub const PROGRAM_CHANGE: u8 = 0xC0;
    pub const CHANNEL_PRESSURE: u8 = 0xD0;
    pub const PITCH_BEND: u8 = 0xE0;
}

/// Type of MIDI binding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MidiBindingType {
    /// Note on/off
    Note,
    /// Control Change (CC)
    ControlChange,
    /// Program Change
    ProgramChange,
    /// Pitch Bend
    PitchBend,
}

/// A MIDI binding identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiBinding {
    /// Binding type
    pub binding_type: MidiBindingType,
    /// MIDI channel (0-15, or None for any channel)
    pub channel: Option<u8>,
    /// Data byte 1 (note number, CC number, etc.)
    pub data1: u8,
}

impl MidiBinding {
    /// Create a note binding
    pub fn note(channel: u8, note: u8) -> Self {
        Self {
            binding_type: MidiBindingType::Note,
            channel: Some(channel),
            data1: note,
        }
    }

    /// Create a note binding for any channel
    pub fn note_any(note: u8) -> Self {
        Self {
            binding_type: MidiBindingType::Note,
            channel: None,
            data1: note,
        }
    }

    /// Create a CC binding
    pub fn cc(channel: u8, cc: u8) -> Self {
        Self {
            binding_type: MidiBindingType::ControlChange,
            channel: Some(channel),
            data1: cc,
        }
    }

    /// Create a CC binding for any channel
    pub fn cc_any(cc: u8) -> Self {
        Self {
            binding_type: MidiBindingType::ControlChange,
            channel: None,
            data1: cc,
        }
    }

    /// Create a program change binding
    pub fn program(channel: u8, program: u8) -> Self {
        Self {
            binding_type: MidiBindingType::ProgramChange,
            channel: Some(channel),
            data1: program,
        }
    }

    /// Create a pitch bend binding
    pub fn pitch_bend(channel: u8) -> Self {
        Self {
            binding_type: MidiBindingType::PitchBend,
            channel: Some(channel),
            data1: 0, // Not used for pitch bend
        }
    }

    /// Check if this binding matches a MIDI message
    pub fn matches(&self, channel: u8, status: u8, data1: u8) -> bool {
        // Check channel
        if let Some(ch) = self.channel {
            if ch != channel {
                return false;
            }
        }

        // Check message type
        let msg_type = status & 0xF0;
        let expected_type = match self.binding_type {
            MidiBindingType::Note => status::NOTE_ON,
            MidiBindingType::ControlChange => status::CONTROL_CHANGE,
            MidiBindingType::ProgramChange => status::PROGRAM_CHANGE,
            MidiBindingType::PitchBend => status::PITCH_BEND,
        };

        if msg_type != expected_type {
            // Also accept note off for note bindings
            if self.binding_type == MidiBindingType::Note && msg_type == status::NOTE_OFF {
                // Continue to data check
            } else {
                return false;
            }
        }

        // Check data1 (except for pitch bend)
        if self.binding_type != MidiBindingType::PitchBend {
            if self.data1 != data1 {
                return false;
            }
        }

        true
    }
}

/// Encoder mode for CC controls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderMode {
    /// Absolute (0-127)
    Absolute,
    /// Relative (64 = no change, <64 = decrement, >64 = increment)
    Relative64,
    /// Relative (0 = no change, 1-63 = increment, 65-127 = decrement)
    RelativeBinary,
    /// Relative (signed, 1-64 = increment, 65-127 = decrement mapped to -63 to -1)
    RelativeSigned,
}

impl Default for EncoderMode {
    fn default() -> Self {
        EncoderMode::Absolute
    }
}

impl EncoderMode {
    /// Convert raw CC value to normalized change (-1.0 to 1.0 or 0.0 to 1.0)
    pub fn normalize(&self, value: u8, sensitivity: f64) -> f64 {
        match self {
            EncoderMode::Absolute => value as f64 / 127.0,
            EncoderMode::Relative64 => {
                let delta = (value as i8 - 64) as f64;
                delta * sensitivity / 64.0
            }
            EncoderMode::RelativeBinary => {
                if value == 0 {
                    0.0
                } else if value < 64 {
                    value as f64 * sensitivity / 63.0
                } else {
                    -((value - 64) as f64 * sensitivity / 63.0)
                }
            }
            EncoderMode::RelativeSigned => {
                if value <= 64 {
                    value as f64 * sensitivity / 64.0
                } else {
                    -((128 - value) as f64 * sensitivity / 63.0)
                }
            }
        }
    }
}

/// A complete MIDI mapping entry
#[derive(Debug, Clone)]
pub struct MidiMappingEntry {
    /// The binding
    pub binding: MidiBinding,
    /// The action to perform
    pub action: ControlAction,
    /// Encoder mode for CC controls
    pub encoder_mode: EncoderMode,
    /// Sensitivity for relative encoders
    pub sensitivity: f64,
    /// Description
    pub description: String,
    /// Mapping layer (for switching between mapping sets)
    pub layer: u8,
}

impl MidiMappingEntry {
    /// Create a new mapping entry
    pub fn new(binding: MidiBinding, action: ControlAction) -> Self {
        Self {
            binding,
            action,
            encoder_mode: EncoderMode::Absolute,
            sensitivity: 1.0,
            description: String::new(),
            layer: 0,
        }
    }

    /// Set encoder mode
    pub fn encoder_mode(mut self, mode: EncoderMode) -> Self {
        self.encoder_mode = mode;
        self
    }

    /// Set sensitivity
    pub fn sensitivity(mut self, sens: f64) -> Self {
        self.sensitivity = sens;
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set layer
    pub fn layer(mut self, layer: u8) -> Self {
        self.layer = layer;
        self
    }
}

/// Configuration for MIDI mappings (serializable)
#[derive(Debug, Clone)]
pub struct MidiMapConfig {
    /// Default encoder mode
    pub default_encoder_mode: EncoderMode,
    /// Default sensitivity
    pub default_sensitivity: f64,
    /// Number of layers
    pub num_layers: u8,
}

impl Default for MidiMapConfig {
    fn default() -> Self {
        Self {
            default_encoder_mode: EncoderMode::Absolute,
            default_sensitivity: 1.0,
            num_layers: 1,
        }
    }
}

/// MIDI controller with configurable mappings
pub struct MidiController {
    /// Mapping entries
    mappings: Vec<MidiMappingEntry>,
    /// Current active layer
    current_layer: u8,
    /// Learn mode active
    learn_mode: bool,
    /// Last received message (for learn mode)
    last_message: Option<(MidiBinding, u8)>,
    /// Configuration
    config: MidiMapConfig,
}

impl MidiController {
    /// Create a new MIDI controller
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
            current_layer: 0,
            learn_mode: false,
            last_message: None,
            config: MidiMapConfig::default(),
        }
    }

    /// Create with configuration
    pub fn with_config(config: MidiMapConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Add a mapping
    pub fn add_mapping(&mut self, entry: MidiMappingEntry) {
        self.mappings.push(entry);
    }

    /// Add a simple binding
    pub fn add_binding(&mut self, binding: MidiBinding, action: ControlAction) {
        self.mappings.push(MidiMappingEntry::new(binding, action));
    }

    /// Remove mappings for a binding
    pub fn remove_binding(&mut self, binding: &MidiBinding) {
        self.mappings.retain(|m| &m.binding != binding);
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.mappings.clear();
    }

    /// Set current layer
    pub fn set_layer(&mut self, layer: u8) {
        self.current_layer = layer % self.config.num_layers;
    }

    /// Get current layer
    pub fn current_layer(&self) -> u8 {
        self.current_layer
    }

    /// Toggle learn mode
    pub fn toggle_learn(&mut self) {
        self.learn_mode = !self.learn_mode;
        if !self.learn_mode {
            self.last_message = None;
        }
    }

    /// Check if in learn mode
    pub fn is_learning(&self) -> bool {
        self.learn_mode
    }

    /// Get last received message (for learn mode display)
    pub fn last_message(&self) -> Option<&(MidiBinding, u8)> {
        self.last_message.as_ref()
    }

    /// Process a MIDI message and return action
    pub fn process_message(
        &self,
        channel: u8,
        status: u8,
        data1: u8,
        data2: u8,
    ) -> Option<ControlAction> {
        // Find matching mapping on current layer
        for entry in &self.mappings {
            if entry.layer != self.current_layer {
                continue;
            }

            if entry.binding.matches(channel, status, data1) {
                return Some(self.apply_mapping(entry, data2));
            }
        }

        None
    }

    /// Process a MIDI message and update learn state
    pub fn process_message_learn(
        &mut self,
        channel: u8,
        status: u8,
        data1: u8,
        data2: u8,
    ) -> Option<ControlAction> {
        let msg_type = status & 0xF0;

        // Update last message for learn mode
        let binding = match msg_type {
            status::NOTE_ON | status::NOTE_OFF => MidiBinding::note(channel, data1),
            status::CONTROL_CHANGE => MidiBinding::cc(channel, data1),
            status::PROGRAM_CHANGE => MidiBinding::program(channel, data1),
            status::PITCH_BEND => MidiBinding::pitch_bend(channel),
            _ => return None,
        };

        self.last_message = Some((binding, data2));

        if self.learn_mode {
            return None; // In learn mode, don't trigger actions
        }

        self.process_message(channel, status, data1, data2)
    }

    /// Apply a mapping to get the action with value
    fn apply_mapping(&self, entry: &MidiMappingEntry, value: u8) -> ControlAction {
        match &entry.action {
            ControlAction::SetParameter(name, _) => {
                let normalized = entry.encoder_mode.normalize(value, entry.sensitivity);
                ControlAction::SetParameter(name.clone(), normalized)
            }
            ControlAction::AdjustParameter(name, _) => {
                let delta = match entry.encoder_mode {
                    EncoderMode::Absolute => (value as f64 / 127.0 - 0.5) * entry.sensitivity,
                    _ => entry.encoder_mode.normalize(value, entry.sensitivity),
                };
                ControlAction::AdjustParameter(name.clone(), delta)
            }
            ControlAction::SetTrackVolume(track, _) => {
                let vol = value as f64 / 127.0;
                ControlAction::SetTrackVolume(*track, vol)
            }
            ControlAction::AdjustTempo(_) => {
                let delta = match entry.encoder_mode {
                    EncoderMode::Absolute => (value as f64 - 64.0) / 64.0 * entry.sensitivity * 10.0,
                    _ => entry.encoder_mode.normalize(value, entry.sensitivity) * 10.0,
                };
                ControlAction::AdjustTempo(delta)
            }
            // For non-value actions (notes, triggers), only act on non-zero values
            other => {
                if value > 0 {
                    other.clone()
                } else {
                    ControlAction::None
                }
            }
        }
    }

    /// Get all mappings for display
    pub fn mappings(&self) -> &[MidiMappingEntry] {
        &self.mappings
    }

    /// Get mappings for current layer
    pub fn active_mappings(&self) -> impl Iterator<Item = &MidiMappingEntry> {
        self.mappings.iter().filter(|m| m.layer == self.current_layer)
    }
}

impl Default for MidiController {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a MIDI binding for display
pub fn format_binding(binding: &MidiBinding) -> String {
    let channel = binding
        .channel
        .map(|c| format!("Ch{} ", c + 1))
        .unwrap_or_default();

    match binding.binding_type {
        MidiBindingType::Note => {
            let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
            let octave = (binding.data1 / 12) as i8 - 1;
            let name = note_names[(binding.data1 % 12) as usize];
            format!("{}Note {}{}", channel, name, octave)
        }
        MidiBindingType::ControlChange => {
            format!("{}CC {}", channel, binding.data1)
        }
        MidiBindingType::ProgramChange => {
            format!("{}PC {}", channel, binding.data1)
        }
        MidiBindingType::PitchBend => {
            format!("{}PitchBend", channel)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_binding_note() {
        let binding = MidiBinding::note(0, 60);
        assert!(binding.matches(0, status::NOTE_ON, 60));
        assert!(binding.matches(0, status::NOTE_OFF, 60));
        assert!(!binding.matches(1, status::NOTE_ON, 60));
        assert!(!binding.matches(0, status::NOTE_ON, 61));
    }

    #[test]
    fn test_midi_binding_cc() {
        let binding = MidiBinding::cc(0, 1);
        assert!(binding.matches(0, status::CONTROL_CHANGE, 1));
        assert!(!binding.matches(0, status::NOTE_ON, 1));
        assert!(!binding.matches(0, status::CONTROL_CHANGE, 2));
    }

    #[test]
    fn test_midi_binding_any_channel() {
        let binding = MidiBinding::cc_any(1);
        assert!(binding.matches(0, status::CONTROL_CHANGE, 1));
        assert!(binding.matches(5, status::CONTROL_CHANGE, 1));
        assert!(binding.matches(15, status::CONTROL_CHANGE, 1));
    }

    #[test]
    fn test_encoder_mode_absolute() {
        let mode = EncoderMode::Absolute;
        assert!((mode.normalize(0, 1.0) - 0.0).abs() < 0.01);
        assert!((mode.normalize(127, 1.0) - 1.0).abs() < 0.01);
        assert!((mode.normalize(64, 1.0) - 0.504).abs() < 0.01);
    }

    #[test]
    fn test_encoder_mode_relative64() {
        let mode = EncoderMode::Relative64;
        assert!((mode.normalize(64, 1.0) - 0.0).abs() < 0.01);
        assert!(mode.normalize(65, 1.0) > 0.0);
        assert!(mode.normalize(63, 1.0) < 0.0);
    }

    #[test]
    fn test_midi_controller_process() {
        let mut controller = MidiController::new();

        controller.add_binding(
            MidiBinding::note(0, 60),
            ControlAction::TriggerScene(0),
        );

        // Note on should trigger
        let action = controller.process_message(0, status::NOTE_ON, 60, 100);
        assert_eq!(action, Some(ControlAction::TriggerScene(0)));

        // Wrong note should not trigger
        let action = controller.process_message(0, status::NOTE_ON, 61, 100);
        assert_eq!(action, None);
    }

    #[test]
    fn test_midi_controller_layers() {
        let mut controller = MidiController::with_config(MidiMapConfig {
            num_layers: 2,
            ..Default::default()
        });

        controller.add_mapping(
            MidiMappingEntry::new(
                MidiBinding::note(0, 60),
                ControlAction::TriggerScene(0),
            ).layer(0)
        );

        controller.add_mapping(
            MidiMappingEntry::new(
                MidiBinding::note(0, 60),
                ControlAction::TriggerScene(1),
            ).layer(1)
        );

        // Layer 0
        controller.set_layer(0);
        let action = controller.process_message(0, status::NOTE_ON, 60, 100);
        assert_eq!(action, Some(ControlAction::TriggerScene(0)));

        // Layer 1
        controller.set_layer(1);
        let action = controller.process_message(0, status::NOTE_ON, 60, 100);
        assert_eq!(action, Some(ControlAction::TriggerScene(1)));
    }

    #[test]
    fn test_format_binding() {
        let binding = MidiBinding::note(0, 60);
        assert_eq!(format_binding(&binding), "Ch1 Note C4");

        let binding = MidiBinding::cc(1, 7);
        assert_eq!(format_binding(&binding), "Ch2 CC 7");

        let binding = MidiBinding::cc_any(1);
        assert_eq!(format_binding(&binding), "CC 1");
    }

    #[test]
    fn test_learn_mode() {
        let mut controller = MidiController::new();
        assert!(!controller.is_learning());

        controller.toggle_learn();
        assert!(controller.is_learning());

        // In learn mode, messages should not trigger actions
        controller.add_binding(
            MidiBinding::note(0, 60),
            ControlAction::TriggerScene(0),
        );
        let action = controller.process_message_learn(0, status::NOTE_ON, 60, 100);
        assert_eq!(action, None);

        // But should capture the message
        assert!(controller.last_message().is_some());

        controller.toggle_learn();
        assert!(!controller.is_learning());
    }
}
