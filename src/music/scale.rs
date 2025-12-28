// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Scale and key system for musical operations.
//!
//! Provides scale definitions, note-to-scale-degree mapping,
//! transposition within scales, and key relationships.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// MIDI note number type (0-127)
pub type MidiNote = u8;

/// Semitone offset type
pub type Semitones = i8;

/// Note names (pitch classes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Note {
    C,
    Cs, // C# / Db
    D,
    Ds, // D# / Eb
    E,
    F,
    Fs, // F# / Gb
    G,
    Gs, // G# / Ab
    A,
    As, // A# / Bb
    B,
}

impl Note {
    /// All notes in chromatic order
    pub const ALL: [Note; 12] = [
        Note::C,
        Note::Cs,
        Note::D,
        Note::Ds,
        Note::E,
        Note::F,
        Note::Fs,
        Note::G,
        Note::Gs,
        Note::A,
        Note::As,
        Note::B,
    ];

    /// Get the pitch class (0-11) for this note
    pub fn pitch_class(self) -> u8 {
        match self {
            Note::C => 0,
            Note::Cs => 1,
            Note::D => 2,
            Note::Ds => 3,
            Note::E => 4,
            Note::F => 5,
            Note::Fs => 6,
            Note::G => 7,
            Note::Gs => 8,
            Note::A => 9,
            Note::As => 10,
            Note::B => 11,
        }
    }

    /// Get note from pitch class
    pub fn from_pitch_class(pc: u8) -> Self {
        Note::ALL[(pc % 12) as usize]
    }

    /// Parse note from string (e.g., "C", "C#", "Db", "F#")
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim().to_uppercase();
        match s.as_str() {
            "C" => Some(Note::C),
            "C#" | "CS" | "DB" => Some(Note::Cs),
            "D" => Some(Note::D),
            "D#" | "DS" | "EB" => Some(Note::Ds),
            "E" | "FB" => Some(Note::E),
            "F" | "E#" | "ES" => Some(Note::F),
            "F#" | "FS" | "GB" => Some(Note::Fs),
            "G" => Some(Note::G),
            "G#" | "GS" | "AB" => Some(Note::Gs),
            "A" => Some(Note::A),
            "A#" | "AS" | "BB" => Some(Note::As),
            "B" | "CB" => Some(Note::B),
            _ => None,
        }
    }

    /// Transpose by semitones
    pub fn transpose(self, semitones: Semitones) -> Self {
        let new_pc = (self.pitch_class() as i8 + semitones).rem_euclid(12) as u8;
        Note::from_pitch_class(new_pc)
    }

    /// Get interval in semitones to another note (ascending)
    pub fn interval_to(self, other: Note) -> u8 {
        (other.pitch_class() as i16 - self.pitch_class() as i16).rem_euclid(12) as u8
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Note::C => write!(f, "C"),
            Note::Cs => write!(f, "C#"),
            Note::D => write!(f, "D"),
            Note::Ds => write!(f, "D#"),
            Note::E => write!(f, "E"),
            Note::F => write!(f, "F"),
            Note::Fs => write!(f, "F#"),
            Note::G => write!(f, "G"),
            Note::Gs => write!(f, "G#"),
            Note::A => write!(f, "A"),
            Note::As => write!(f, "A#"),
            Note::B => write!(f, "B"),
        }
    }
}

/// Scale types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaleType {
    // Major scale and modes
    Major,        // Ionian
    Dorian,       // Minor with raised 6th
    Phrygian,     // Minor with lowered 2nd
    Lydian,       // Major with raised 4th
    Mixolydian,   // Major with lowered 7th
    NaturalMinor, // Aeolian
    Locrian,      // Diminished

    // Other minor scales
    HarmonicMinor,
    MelodicMinor, // Ascending form

    // Pentatonic scales
    MajorPentatonic,
    MinorPentatonic,

    // Blues
    Blues,
    MajorBlues,

    // Other common scales
    WholeTone,
    Diminished,     // Half-whole
    DiminishedWH,   // Whole-half
    Chromatic,

    // Custom scale from intervals
    Custom,
}

impl ScaleType {
    /// Get the intervals (semitones from root) for this scale type
    pub fn intervals(self) -> Vec<u8> {
        match self {
            // Major and modes
            ScaleType::Major => vec![0, 2, 4, 5, 7, 9, 11],
            ScaleType::Dorian => vec![0, 2, 3, 5, 7, 9, 10],
            ScaleType::Phrygian => vec![0, 1, 3, 5, 7, 8, 10],
            ScaleType::Lydian => vec![0, 2, 4, 6, 7, 9, 11],
            ScaleType::Mixolydian => vec![0, 2, 4, 5, 7, 9, 10],
            ScaleType::NaturalMinor => vec![0, 2, 3, 5, 7, 8, 10],
            ScaleType::Locrian => vec![0, 1, 3, 5, 6, 8, 10],

            // Other minor scales
            ScaleType::HarmonicMinor => vec![0, 2, 3, 5, 7, 8, 11],
            ScaleType::MelodicMinor => vec![0, 2, 3, 5, 7, 9, 11],

            // Pentatonic
            ScaleType::MajorPentatonic => vec![0, 2, 4, 7, 9],
            ScaleType::MinorPentatonic => vec![0, 3, 5, 7, 10],

            // Blues
            ScaleType::Blues => vec![0, 3, 5, 6, 7, 10],
            ScaleType::MajorBlues => vec![0, 2, 3, 4, 7, 9],

            // Symmetric scales
            ScaleType::WholeTone => vec![0, 2, 4, 6, 8, 10],
            ScaleType::Diminished => vec![0, 1, 3, 4, 6, 7, 9, 10],
            ScaleType::DiminishedWH => vec![0, 2, 3, 5, 6, 8, 9, 11],
            ScaleType::Chromatic => vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],

            ScaleType::Custom => vec![], // Custom scales define their own
        }
    }

    /// Parse scale type from string
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase().replace([' ', '-', '_'], "");
        match s.as_str() {
            "major" | "ionian" => Some(ScaleType::Major),
            "dorian" => Some(ScaleType::Dorian),
            "phrygian" => Some(ScaleType::Phrygian),
            "lydian" => Some(ScaleType::Lydian),
            "mixolydian" => Some(ScaleType::Mixolydian),
            "minor" | "naturalminor" | "aeolian" => Some(ScaleType::NaturalMinor),
            "locrian" => Some(ScaleType::Locrian),
            "harmonicminor" => Some(ScaleType::HarmonicMinor),
            "melodicminor" => Some(ScaleType::MelodicMinor),
            "majorpentatonic" | "pentatonicmajor" => Some(ScaleType::MajorPentatonic),
            "minorpentatonic" | "pentatonicminor" | "pentatonic" => Some(ScaleType::MinorPentatonic),
            "blues" | "minorblues" => Some(ScaleType::Blues),
            "majorblues" => Some(ScaleType::MajorBlues),
            "wholetone" => Some(ScaleType::WholeTone),
            "diminished" | "octatonic" | "halfwhole" => Some(ScaleType::Diminished),
            "diminishedwh" | "wholehalf" => Some(ScaleType::DiminishedWH),
            "chromatic" => Some(ScaleType::Chromatic),
            _ => None,
        }
    }

    /// Get a human-readable name for this scale type
    pub fn name(self) -> &'static str {
        match self {
            ScaleType::Major => "Major",
            ScaleType::Dorian => "Dorian",
            ScaleType::Phrygian => "Phrygian",
            ScaleType::Lydian => "Lydian",
            ScaleType::Mixolydian => "Mixolydian",
            ScaleType::NaturalMinor => "Natural Minor",
            ScaleType::Locrian => "Locrian",
            ScaleType::HarmonicMinor => "Harmonic Minor",
            ScaleType::MelodicMinor => "Melodic Minor",
            ScaleType::MajorPentatonic => "Major Pentatonic",
            ScaleType::MinorPentatonic => "Minor Pentatonic",
            ScaleType::Blues => "Blues",
            ScaleType::MajorBlues => "Major Blues",
            ScaleType::WholeTone => "Whole Tone",
            ScaleType::Diminished => "Diminished",
            ScaleType::DiminishedWH => "Diminished (W-H)",
            ScaleType::Chromatic => "Chromatic",
            ScaleType::Custom => "Custom",
        }
    }

    /// Get the parallel minor/major scale type
    pub fn parallel(self) -> Option<Self> {
        match self {
            ScaleType::Major => Some(ScaleType::NaturalMinor),
            ScaleType::NaturalMinor => Some(ScaleType::Major),
            ScaleType::MajorPentatonic => Some(ScaleType::MinorPentatonic),
            ScaleType::MinorPentatonic => Some(ScaleType::MajorPentatonic),
            _ => None,
        }
    }
}

impl fmt::Display for ScaleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A complete scale with root and type
#[derive(Debug, Clone, PartialEq)]
pub struct Scale {
    root: Note,
    scale_type: ScaleType,
    intervals: Vec<u8>,
    notes: Vec<Note>,
}

impl Scale {
    /// Create a new scale from root and type
    pub fn new(root: Note, scale_type: ScaleType) -> Self {
        let intervals = scale_type.intervals();
        let notes: Vec<Note> = intervals
            .iter()
            .map(|&i| root.transpose(i as Semitones))
            .collect();

        Self {
            root,
            scale_type,
            intervals,
            notes,
        }
    }

    /// Create a custom scale from root and intervals
    pub fn custom(root: Note, intervals: Vec<u8>) -> Self {
        let notes: Vec<Note> = intervals
            .iter()
            .map(|&i| root.transpose(i as Semitones))
            .collect();

        Self {
            root,
            scale_type: ScaleType::Custom,
            intervals,
            notes,
        }
    }

    /// Parse a scale from strings (e.g., "C", "major")
    pub fn parse(root_str: &str, scale_str: &str) -> Option<Self> {
        let root = Note::from_str(root_str)?;
        let scale_type = ScaleType::from_str(scale_str)?;
        Some(Scale::new(root, scale_type))
    }

    /// Get the root note
    pub fn root(&self) -> Note {
        self.root
    }

    /// Get the scale type
    pub fn scale_type(&self) -> ScaleType {
        self.scale_type
    }

    /// Get the intervals (semitones from root)
    pub fn intervals(&self) -> &[u8] {
        &self.intervals
    }

    /// Get the notes in this scale
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Get the number of notes in this scale
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// Check if this scale is empty (shouldn't happen normally)
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Check if a note is in this scale
    pub fn contains(&self, note: Note) -> bool {
        self.notes.contains(&note)
    }

    /// Check if a MIDI note is in this scale
    pub fn contains_midi(&self, midi_note: MidiNote) -> bool {
        let note = Note::from_pitch_class(midi_note % 12);
        self.contains(note)
    }

    /// Get the scale degree (1-based) for a note, if it's in the scale
    pub fn degree_of(&self, note: Note) -> Option<usize> {
        self.notes.iter().position(|&n| n == note).map(|i| i + 1)
    }

    /// Get the note at a given scale degree (1-based)
    pub fn note_at_degree(&self, degree: usize) -> Option<Note> {
        if degree == 0 || degree > self.len() {
            return None;
        }
        Some(self.notes[degree - 1])
    }

    /// Get a MIDI note at a given scale degree and octave
    /// Degree is 1-based, octave uses MIDI convention (middle C = C4 = 60)
    pub fn midi_note_at(&self, degree: usize, octave: i8) -> Option<MidiNote> {
        let note = self.note_at_degree(degree)?;
        let midi = (octave as i16 + 1) * 12 + note.pitch_class() as i16;
        if midi < 0 || midi > 127 {
            return None;
        }
        Some(midi as MidiNote)
    }

    /// Transpose a MIDI note by scale degrees (positive = up, negative = down)
    /// Returns the transposed note, wrapping to stay within scale
    pub fn transpose_in_scale(&self, midi_note: MidiNote, degrees: i32) -> MidiNote {
        if self.is_empty() {
            return midi_note;
        }

        let pitch_class = midi_note % 12;
        let note = Note::from_pitch_class(pitch_class);
        let octave = (midi_note / 12) as i32 - 1;

        // Find current position in scale (or nearest)
        let current_degree = self
            .notes
            .iter()
            .position(|&n| n == note)
            .unwrap_or_else(|| self.nearest_degree(note));

        // Calculate new position
        let scale_len = self.len() as i32;
        let new_pos = current_degree as i32 + degrees;
        let new_degree = new_pos.rem_euclid(scale_len) as usize;
        let octave_change = new_pos.div_euclid(scale_len);

        // Get the new note
        let new_note = self.notes[new_degree];
        let new_octave = octave + octave_change;
        let result = ((new_octave + 1) * 12 + new_note.pitch_class() as i32) as i32;

        result.clamp(0, 127) as MidiNote
    }

    /// Find the nearest scale degree for a note not in the scale
    fn nearest_degree(&self, note: Note) -> usize {
        let pc = note.pitch_class();
        let mut min_dist = 12u8;
        let mut nearest = 0usize;

        for (i, &scale_note) in self.notes.iter().enumerate() {
            let spc = scale_note.pitch_class();
            let dist = ((pc as i8 - spc as i8).abs()).min(12 - (pc as i8 - spc as i8).abs()) as u8;
            if dist < min_dist {
                min_dist = dist;
                nearest = i;
            }
        }

        nearest
    }

    /// Quantize a MIDI note to the nearest note in the scale
    pub fn quantize(&self, midi_note: MidiNote) -> MidiNote {
        if self.is_empty() {
            return midi_note;
        }

        let pitch_class = midi_note % 12;
        let octave = midi_note / 12;
        let note = Note::from_pitch_class(pitch_class);

        // If already in scale, return as-is
        if self.contains(note) {
            return midi_note;
        }

        // Find nearest scale note
        let nearest_idx = self.nearest_degree(note);
        let nearest_note = self.notes[nearest_idx];

        octave * 12 + nearest_note.pitch_class()
    }

    /// Get the parallel scale (major <-> minor)
    pub fn parallel(&self) -> Option<Scale> {
        self.scale_type.parallel().map(|st| Scale::new(self.root, st))
    }

    /// Get the relative scale (e.g., C major -> A minor)
    pub fn relative(&self) -> Option<Scale> {
        match self.scale_type {
            ScaleType::Major => {
                let relative_root = self.root.transpose(-3); // Down a minor 3rd
                Some(Scale::new(relative_root, ScaleType::NaturalMinor))
            }
            ScaleType::NaturalMinor => {
                let relative_root = self.root.transpose(3); // Up a minor 3rd
                Some(Scale::new(relative_root, ScaleType::Major))
            }
            _ => None,
        }
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.root, self.scale_type)
    }
}

/// A musical key with root and mode
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    root: Note,
    scale: Scale,
}

impl Key {
    /// Create a new key
    pub fn new(root: Note, scale_type: ScaleType) -> Self {
        Self {
            root,
            scale: Scale::new(root, scale_type),
        }
    }

    /// Parse a key from strings
    pub fn parse(root_str: &str, scale_str: &str) -> Option<Self> {
        let root = Note::from_str(root_str)?;
        let scale_type = ScaleType::from_str(scale_str)?;
        Some(Key::new(root, scale_type))
    }

    /// Get the root note
    pub fn root(&self) -> Note {
        self.root
    }

    /// Get the scale
    pub fn scale(&self) -> &Scale {
        &self.scale
    }

    /// Transpose the key by semitones
    pub fn transpose(&self, semitones: Semitones) -> Self {
        let new_root = self.root.transpose(semitones);
        Key::new(new_root, self.scale.scale_type())
    }

    /// Get the relative key
    pub fn relative(&self) -> Option<Key> {
        self.scale.relative().map(|s| Key {
            root: s.root(),
            scale: s,
        })
    }

    /// Get the parallel key
    pub fn parallel(&self) -> Option<Key> {
        self.scale.parallel().map(|s| Key {
            root: s.root(),
            scale: s,
        })
    }

    /// Get the dominant key (V)
    pub fn dominant(&self) -> Key {
        Key::new(self.root.transpose(7), self.scale.scale_type())
    }

    /// Get the subdominant key (IV)
    pub fn subdominant(&self) -> Key {
        Key::new(self.root.transpose(5), self.scale.scale_type())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.scale)
    }
}

/// Custom scale definitions that can be loaded from config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomScaleDefinition {
    /// Name of the scale
    pub name: String,
    /// Intervals from root (semitones)
    pub intervals: Vec<u8>,
}

impl CustomScaleDefinition {
    /// Create a scale from this definition
    pub fn to_scale(&self, root: Note) -> Scale {
        Scale::custom(root, self.intervals.clone())
    }
}

/// Registry for custom scale definitions
#[derive(Debug, Clone, Default)]
pub struct ScaleRegistry {
    custom_scales: HashMap<String, CustomScaleDefinition>,
}

impl ScaleRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom scale
    pub fn register(&mut self, def: CustomScaleDefinition) {
        self.custom_scales.insert(def.name.clone(), def);
    }

    /// Get a scale by name (checks custom first, then built-in)
    pub fn get_scale(&self, root: Note, name: &str) -> Option<Scale> {
        // Check custom scales first
        if let Some(def) = self.custom_scales.get(name) {
            return Some(def.to_scale(root));
        }

        // Fall back to built-in
        ScaleType::from_str(name).map(|st| Scale::new(root, st))
    }

    /// List all available scale names
    pub fn available_scales(&self) -> Vec<String> {
        let mut names: Vec<String> = self.custom_scales.keys().cloned().collect();

        // Add built-in scale names
        let built_in = [
            "major",
            "dorian",
            "phrygian",
            "lydian",
            "mixolydian",
            "minor",
            "locrian",
            "harmonic_minor",
            "melodic_minor",
            "major_pentatonic",
            "minor_pentatonic",
            "blues",
            "major_blues",
            "whole_tone",
            "diminished",
            "chromatic",
        ];

        names.extend(built_in.iter().map(|s| s.to_string()));
        names.sort();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_pitch_class() {
        assert_eq!(Note::C.pitch_class(), 0);
        assert_eq!(Note::A.pitch_class(), 9);
        assert_eq!(Note::B.pitch_class(), 11);
    }

    #[test]
    fn test_note_from_str() {
        assert_eq!(Note::from_str("C"), Some(Note::C));
        assert_eq!(Note::from_str("C#"), Some(Note::Cs));
        assert_eq!(Note::from_str("Db"), Some(Note::Cs));
        assert_eq!(Note::from_str("F#"), Some(Note::Fs));
        assert_eq!(Note::from_str("Bb"), Some(Note::As));
        assert_eq!(Note::from_str("X"), None);
    }

    #[test]
    fn test_note_transpose() {
        assert_eq!(Note::C.transpose(2), Note::D);
        assert_eq!(Note::C.transpose(12), Note::C);
        assert_eq!(Note::C.transpose(-1), Note::B);
        assert_eq!(Note::G.transpose(5), Note::C);
    }

    #[test]
    fn test_note_interval() {
        assert_eq!(Note::C.interval_to(Note::G), 7);
        assert_eq!(Note::C.interval_to(Note::C), 0);
        assert_eq!(Note::G.interval_to(Note::C), 5);
    }

    #[test]
    fn test_scale_type_intervals() {
        let major = ScaleType::Major.intervals();
        assert_eq!(major, vec![0, 2, 4, 5, 7, 9, 11]);

        let minor = ScaleType::NaturalMinor.intervals();
        assert_eq!(minor, vec![0, 2, 3, 5, 7, 8, 10]);

        let pentatonic = ScaleType::MinorPentatonic.intervals();
        assert_eq!(pentatonic, vec![0, 3, 5, 7, 10]);
    }

    #[test]
    fn test_scale_type_from_str() {
        assert_eq!(ScaleType::from_str("major"), Some(ScaleType::Major));
        assert_eq!(ScaleType::from_str("Minor"), Some(ScaleType::NaturalMinor));
        assert_eq!(ScaleType::from_str("dorian"), Some(ScaleType::Dorian));
        assert_eq!(ScaleType::from_str("harmonic_minor"), Some(ScaleType::HarmonicMinor));
        assert_eq!(ScaleType::from_str("unknown"), None);
    }

    #[test]
    fn test_scale_notes() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        assert_eq!(
            c_major.notes(),
            &[Note::C, Note::D, Note::E, Note::F, Note::G, Note::A, Note::B]
        );

        let a_minor = Scale::new(Note::A, ScaleType::NaturalMinor);
        assert_eq!(
            a_minor.notes(),
            &[Note::A, Note::B, Note::C, Note::D, Note::E, Note::F, Note::G]
        );
    }

    #[test]
    fn test_scale_contains() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        assert!(c_major.contains(Note::C));
        assert!(c_major.contains(Note::G));
        assert!(!c_major.contains(Note::Cs));
        assert!(!c_major.contains(Note::Fs));
    }

    #[test]
    fn test_scale_degree() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        assert_eq!(c_major.degree_of(Note::C), Some(1));
        assert_eq!(c_major.degree_of(Note::E), Some(3));
        assert_eq!(c_major.degree_of(Note::B), Some(7));
        assert_eq!(c_major.degree_of(Note::Fs), None);
    }

    #[test]
    fn test_scale_note_at_degree() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        assert_eq!(c_major.note_at_degree(1), Some(Note::C));
        assert_eq!(c_major.note_at_degree(5), Some(Note::G));
        assert_eq!(c_major.note_at_degree(0), None);
        assert_eq!(c_major.note_at_degree(8), None);
    }

    #[test]
    fn test_transpose_in_scale() {
        let c_major = Scale::new(Note::C, ScaleType::Major);

        // Middle C (60) up 3 scale degrees = F4 (65)
        assert_eq!(c_major.transpose_in_scale(60, 3), 65);

        // Middle C down 1 scale degree = B3 (59)
        assert_eq!(c_major.transpose_in_scale(60, -1), 59);

        // G4 (67) up 1 = A4 (69)
        assert_eq!(c_major.transpose_in_scale(67, 1), 69);
    }

    #[test]
    fn test_transpose_in_d_minor() {
        let d_minor = Scale::new(Note::D, ScaleType::NaturalMinor);

        // D4 (62) up 3 scale degrees should be G4 (67)
        // D minor: D, E, F, G, A, Bb, C
        assert_eq!(d_minor.transpose_in_scale(62, 3), 67);
    }

    #[test]
    fn test_scale_quantize() {
        let c_major = Scale::new(Note::C, ScaleType::Major);

        // C stays C
        assert_eq!(c_major.quantize(60), 60);

        // C# should quantize to C or D
        let result = c_major.quantize(61);
        assert!(result == 60 || result == 62);

        // F# should quantize to F or G
        let result = c_major.quantize(66);
        assert!(result == 65 || result == 67);
    }

    #[test]
    fn test_scale_relative() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        let relative = c_major.relative().unwrap();
        assert_eq!(relative.root(), Note::A);
        assert_eq!(relative.scale_type(), ScaleType::NaturalMinor);

        let a_minor = Scale::new(Note::A, ScaleType::NaturalMinor);
        let relative = a_minor.relative().unwrap();
        assert_eq!(relative.root(), Note::C);
        assert_eq!(relative.scale_type(), ScaleType::Major);
    }

    #[test]
    fn test_scale_parallel() {
        let c_major = Scale::new(Note::C, ScaleType::Major);
        let parallel = c_major.parallel().unwrap();
        assert_eq!(parallel.root(), Note::C);
        assert_eq!(parallel.scale_type(), ScaleType::NaturalMinor);
    }

    #[test]
    fn test_key_transpose() {
        let c_major = Key::new(Note::C, ScaleType::Major);
        let transposed = c_major.transpose(7);
        assert_eq!(transposed.root(), Note::G);
    }

    #[test]
    fn test_key_dominant_subdominant() {
        let c_major = Key::new(Note::C, ScaleType::Major);
        assert_eq!(c_major.dominant().root(), Note::G);
        assert_eq!(c_major.subdominant().root(), Note::F);
    }

    #[test]
    fn test_custom_scale() {
        // Whole-half diminished manually
        let custom = Scale::custom(Note::C, vec![0, 2, 3, 5, 6, 8, 9, 11]);
        assert_eq!(custom.len(), 8);
        assert!(custom.contains(Note::C));
        assert!(custom.contains(Note::Ds));
        assert!(!custom.contains(Note::E));
    }

    #[test]
    fn test_scale_registry() {
        let mut registry = ScaleRegistry::new();

        // Register a custom "Super Locrian" scale
        registry.register(CustomScaleDefinition {
            name: "super_locrian".to_string(),
            intervals: vec![0, 1, 3, 4, 6, 8, 10],
        });

        // Get custom scale
        let custom = registry.get_scale(Note::C, "super_locrian");
        assert!(custom.is_some());
        let custom = custom.unwrap();
        assert_eq!(custom.len(), 7);

        // Get built-in scale
        let major = registry.get_scale(Note::C, "major");
        assert!(major.is_some());
    }

    #[test]
    fn test_midi_note_at() {
        let c_major = Scale::new(Note::C, ScaleType::Major);

        // Middle C is C4 in MIDI = 60
        assert_eq!(c_major.midi_note_at(1, 4), Some(60));

        // E4 = 64
        assert_eq!(c_major.midi_note_at(3, 4), Some(64));

        // G5 = 79
        assert_eq!(c_major.midi_note_at(5, 5), Some(79));
    }
}
