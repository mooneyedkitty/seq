// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Configuration system for SEQ.
//!
//! This module provides data structures for loading and managing
//! song configurations, track settings, parts, and controller mappings.

pub mod watcher;

pub use watcher::{ConfigEvent, ConfigWatcher, validate_config};

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Root configuration for a song
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SongFile {
    /// Song metadata and settings
    pub song: SongConfig,
    /// Track definitions
    #[serde(default)]
    pub tracks: Vec<TrackConfig>,
    /// Part definitions (named collections of track states)
    #[serde(default)]
    pub parts: HashMap<String, PartConfig>,
}

impl SongFile {
    /// Load a song configuration from a YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        Self::from_yaml(&contents)
    }

    /// Parse a song configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse YAML configuration")
    }

    /// Serialize to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).context("Failed to serialize configuration to YAML")
    }

    /// Save configuration to a YAML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = self.to_yaml()?;
        fs::write(path.as_ref(), yaml)
            .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))
    }
}

/// Song-level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SongConfig {
    /// Song name
    pub name: String,
    /// Tempo in BPM
    #[serde(default = "default_tempo")]
    pub tempo: f64,
    /// Musical key (e.g., "C", "D", "F#")
    #[serde(default = "default_key")]
    pub key: String,
    /// Scale type (e.g., "major", "minor", "dorian")
    #[serde(default = "default_scale")]
    pub scale: String,
    /// Time signature numerator
    #[serde(default = "default_time_sig_num")]
    pub time_signature_num: u8,
    /// Time signature denominator
    #[serde(default = "default_time_sig_den")]
    pub time_signature_den: u8,
    /// Global swing amount (0.0 - 1.0)
    #[serde(default)]
    pub swing: f64,
}

fn default_tempo() -> f64 {
    120.0
}
fn default_key() -> String {
    "C".to_string()
}
fn default_scale() -> String {
    "major".to_string()
}
fn default_time_sig_num() -> u8 {
    4
}
fn default_time_sig_den() -> u8 {
    4
}

impl Default for SongConfig {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            tempo: default_tempo(),
            key: default_key(),
            scale: default_scale(),
            time_signature_num: default_time_sig_num(),
            time_signature_den: default_time_sig_den(),
            swing: 0.0,
        }
    }
}

/// Track configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackConfig {
    /// Track name (used for reference in parts)
    pub name: String,
    /// MIDI channel (1-16)
    #[serde(default = "default_channel")]
    pub channel: u8,
    /// Generator type (if using algorithmic generation)
    #[serde(default)]
    pub generator: Option<String>,
    /// Generator-specific configuration
    #[serde(default)]
    pub config: GeneratorConfig,
    /// Static clips for this track
    #[serde(default)]
    pub clips: Vec<ClipReference>,
    /// Track transpose in semitones
    #[serde(default)]
    pub transpose: i8,
    /// Track-specific swing override
    #[serde(default)]
    pub swing: Option<f64>,
    /// Velocity scaling (0.0 - 2.0, default 1.0)
    #[serde(default = "default_velocity_scale")]
    pub velocity_scale: f64,
}

fn default_channel() -> u8 {
    1
}
fn default_velocity_scale() -> f64 {
    1.0
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            name: "Track".to_string(),
            channel: default_channel(),
            generator: None,
            config: GeneratorConfig::default(),
            clips: Vec::new(),
            transpose: 0,
            swing: None,
            velocity_scale: default_velocity_scale(),
        }
    }
}

/// Reference to a clip file or inline clip
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipReference {
    /// File path to clip (relative to song file)
    #[serde(default)]
    pub file: Option<String>,
    /// Clip name/identifier
    #[serde(default)]
    pub name: Option<String>,
}

/// Generator-specific configuration (flexible key-value pairs)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct GeneratorConfig {
    /// Configuration parameters as key-value pairs
    pub params: HashMap<String, GeneratorValue>,
}

/// Value types supported in generator configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum GeneratorValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Array of values
    Array(Vec<GeneratorValue>),
}

impl GeneratorConfig {
    /// Get a float parameter with default
    pub fn get_float(&self, key: &str, default: f64) -> f64 {
        match self.params.get(key) {
            Some(GeneratorValue::Float(v)) => *v,
            Some(GeneratorValue::Int(v)) => *v as f64,
            _ => default,
        }
    }

    /// Get an integer parameter with default
    pub fn get_int(&self, key: &str, default: i64) -> i64 {
        match self.params.get(key) {
            Some(GeneratorValue::Int(v)) => *v,
            Some(GeneratorValue::Float(v)) => *v as i64,
            _ => default,
        }
    }

    /// Get a string parameter with default
    pub fn get_string(&self, key: &str, default: &str) -> String {
        match self.params.get(key) {
            Some(GeneratorValue::String(v)) => v.clone(),
            _ => default.to_string(),
        }
    }

    /// Get a boolean parameter with default
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.params.get(key) {
            Some(GeneratorValue::Bool(v)) => *v,
            _ => default,
        }
    }
}

/// Part configuration (a named state for all tracks)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PartConfig {
    /// Track states within this part
    #[serde(default)]
    pub tracks: HashMap<String, TrackState>,
    /// Tempo override for this part (if any)
    #[serde(default)]
    pub tempo: Option<f64>,
    /// Key override for this part (if any)
    #[serde(default)]
    pub key: Option<String>,
    /// Scale override for this part (if any)
    #[serde(default)]
    pub scale: Option<String>,
}

/// State of a track within a part
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TrackState {
    /// Simple state: "active", "muted", "solo"
    Simple(String),
    /// Clip selection: "clip_1", "clip_2", etc.
    Clip(String),
    /// Detailed state configuration
    Detailed(TrackStateConfig),
}

impl TrackState {
    /// Check if this state means the track is active
    pub fn is_active(&self) -> bool {
        match self {
            TrackState::Simple(s) => s == "active" || s == "solo",
            TrackState::Clip(_) => true,
            TrackState::Detailed(config) => !config.muted,
        }
    }

    /// Check if this state means the track is muted
    pub fn is_muted(&self) -> bool {
        match self {
            TrackState::Simple(s) => s == "muted",
            TrackState::Clip(_) => false,
            TrackState::Detailed(config) => config.muted,
        }
    }

    /// Get the clip name if this state specifies one
    pub fn clip_name(&self) -> Option<&str> {
        match self {
            TrackState::Simple(_) => None,
            TrackState::Clip(s) => Some(s),
            TrackState::Detailed(config) => config.clip.as_deref(),
        }
    }
}

/// Detailed track state configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TrackStateConfig {
    /// Whether the track is muted
    #[serde(default)]
    pub muted: bool,
    /// Whether the track is soloed
    #[serde(default)]
    pub solo: bool,
    /// Active clip name
    #[serde(default)]
    pub clip: Option<String>,
    /// Generator to use (overrides track default)
    #[serde(default)]
    pub generator: Option<String>,
}

/// Controller mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlsFile {
    /// MIDI device configuration
    #[serde(default)]
    pub midi: MidiDeviceConfig,
    /// Controller mappings
    #[serde(default)]
    pub mappings: Vec<ControlMapping>,
    /// Keyboard shortcuts
    #[serde(default)]
    pub keyboard: HashMap<String, String>,
}

impl ControlsFile {
    /// Load controls configuration from a YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read controls file: {:?}", path.as_ref()))?;
        Self::from_yaml(&contents)
    }

    /// Parse controls configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse controls YAML")
    }
}

/// MIDI device configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MidiDeviceConfig {
    /// Device name to connect to
    #[serde(default)]
    pub device: Option<String>,
    /// Input channel filter (if any)
    #[serde(default)]
    pub input_channel: Option<u8>,
}

/// A single controller mapping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlMapping {
    /// MIDI note number (for note-based triggers)
    #[serde(default)]
    pub note: Option<u8>,
    /// MIDI CC number (for continuous controls)
    #[serde(default)]
    pub cc: Option<u8>,
    /// Action to perform
    pub action: String,
    /// Target of the action (part name, parameter path, etc.)
    #[serde(default)]
    pub target: Option<String>,
    /// Value range for CC mappings [min, max]
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// MIDI channel filter (if any)
    #[serde(default)]
    pub channel: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_song_config() {
        let yaml = r#"
song:
  name: "Test Song"
  tempo: 120
  key: "C"
  scale: "major"

tracks:
  - name: "Pad"
    channel: 1
    generator: drone
    config:
      density: 0.5
      voices: 3

  - name: "Bass"
    channel: 2
    clips:
      - file: "clips/bass_1.yaml"
"#;

        let config = SongFile::from_yaml(yaml).unwrap();
        assert_eq!(config.song.name, "Test Song");
        assert_eq!(config.song.tempo, 120.0);
        assert_eq!(config.song.key, "C");
        assert_eq!(config.tracks.len(), 2);
        assert_eq!(config.tracks[0].name, "Pad");
        assert_eq!(config.tracks[0].generator, Some("drone".to_string()));
        assert_eq!(config.tracks[1].name, "Bass");
        assert_eq!(config.tracks[1].clips.len(), 1);
    }

    #[test]
    fn test_parse_parts() {
        let yaml = r#"
song:
  name: "Test"
  tempo: 100
  key: "D"
  scale: "minor"

tracks:
  - name: "Pad"
    channel: 1
  - name: "Arp"
    channel: 2

parts:
  intro:
    tracks:
      Pad: active
      Arp: muted
  main:
    tracks:
      Pad: active
      Arp: active
"#;

        let config = SongFile::from_yaml(yaml).unwrap();
        assert_eq!(config.parts.len(), 2);

        let intro = config.parts.get("intro").unwrap();
        assert!(intro.tracks.get("Pad").unwrap().is_active());
        assert!(intro.tracks.get("Arp").unwrap().is_muted());

        let main = config.parts.get("main").unwrap();
        assert!(main.tracks.get("Pad").unwrap().is_active());
        assert!(main.tracks.get("Arp").unwrap().is_active());
    }

    #[test]
    fn test_generator_config() {
        let yaml = r#"
song:
  name: "Test"
  tempo: 120
  key: "C"
  scale: "major"

tracks:
  - name: "Arp"
    channel: 1
    generator: arpeggio
    config:
      pattern: "up-down"
      octaves: 2
      rate: "1/16"
      density: 0.8
"#;

        let config = SongFile::from_yaml(yaml).unwrap();
        let track = &config.tracks[0];
        assert_eq!(track.config.get_string("pattern", "up"), "up-down");
        assert_eq!(track.config.get_int("octaves", 1), 2);
        assert_eq!(track.config.get_float("density", 0.5), 0.8);
    }

    #[test]
    fn test_parse_controls() {
        let yaml = r#"
midi:
  device: "Launchpad Mini"

mappings:
  - note: 36
    action: trigger_part
    target: intro

  - cc: 1
    action: set_param
    target: Arp.density
    range: [0.1, 1.0]

keyboard:
  space: toggle_play
  q: trigger_part:intro
"#;

        let controls = ControlsFile::from_yaml(yaml).unwrap();
        assert_eq!(controls.midi.device, Some("Launchpad Mini".to_string()));
        assert_eq!(controls.mappings.len(), 2);
        assert_eq!(controls.mappings[0].note, Some(36));
        assert_eq!(controls.mappings[0].action, "trigger_part");
        assert_eq!(controls.mappings[1].cc, Some(1));
        assert_eq!(controls.mappings[1].range, Some([0.1, 1.0]));
        assert_eq!(controls.keyboard.get("space"), Some(&"toggle_play".to_string()));
    }

    #[test]
    fn test_round_trip() {
        let original = SongFile {
            song: SongConfig {
                name: "Round Trip Test".to_string(),
                tempo: 140.0,
                key: "G".to_string(),
                scale: "dorian".to_string(),
                time_signature_num: 4,
                time_signature_den: 4,
                swing: 0.2,
            },
            tracks: vec![TrackConfig {
                name: "Lead".to_string(),
                channel: 3,
                generator: Some("melody".to_string()),
                config: GeneratorConfig::default(),
                clips: Vec::new(),
                transpose: 0,
                swing: None,
                velocity_scale: 1.0,
            }],
            parts: HashMap::new(),
        };

        let yaml = original.to_yaml().unwrap();
        let parsed = SongFile::from_yaml(&yaml).unwrap();

        assert_eq!(original.song.name, parsed.song.name);
        assert_eq!(original.song.tempo, parsed.song.tempo);
        assert_eq!(original.song.key, parsed.song.key);
        assert_eq!(original.tracks.len(), parsed.tracks.len());
        assert_eq!(original.tracks[0].name, parsed.tracks[0].name);
    }

    #[test]
    fn test_default_values() {
        let yaml = r#"
song:
  name: "Minimal"
"#;

        let config = SongFile::from_yaml(yaml).unwrap();
        assert_eq!(config.song.tempo, 120.0);
        assert_eq!(config.song.key, "C");
        assert_eq!(config.song.scale, "major");
        assert_eq!(config.song.time_signature_num, 4);
        assert_eq!(config.song.time_signature_den, 4);
    }

    #[test]
    fn test_track_state() {
        let active = TrackState::Simple("active".to_string());
        let muted = TrackState::Simple("muted".to_string());
        let clip = TrackState::Clip("clip_1".to_string());

        assert!(active.is_active());
        assert!(!active.is_muted());

        assert!(!muted.is_active());
        assert!(muted.is_muted());

        assert!(clip.is_active());
        assert_eq!(clip.clip_name(), Some("clip_1"));
    }
}
