// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Recording and export system.
//!
//! This module provides:
//! - MIDI recording to clips
//! - Generator output freezing
//! - Standard MIDI file export

pub mod capture;
pub mod export;
pub mod freeze;

pub use capture::{MidiRecorder, RecordMode, RecordedNote, RecordingState};
pub use export::{MidiExporter, MidiFileFormat};
pub use freeze::{ClipFreezer, FreezeOptions};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let recorder = MidiRecorder::new(24);
        assert_eq!(recorder.state(), RecordingState::Idle);
    }

    #[test]
    fn test_exporter_creation() {
        let exporter = MidiExporter::new();
        assert_eq!(exporter.format(), MidiFileFormat::Type0);
    }

    #[test]
    fn test_freezer_creation() {
        let freezer = ClipFreezer::new(24);
        assert!(!freezer.is_freezing());
    }
}
