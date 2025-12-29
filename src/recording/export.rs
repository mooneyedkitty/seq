// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Standard MIDI file export.
//!
//! Exports clips and arrangements as Type 0 or Type 1 MIDI files.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use super::freeze::FrozenNote;

/// MIDI file format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiFileFormat {
    /// Type 0: Single track with all channels
    Type0,
    /// Type 1: Multiple simultaneous tracks
    Type1,
}

impl Default for MidiFileFormat {
    fn default() -> Self {
        MidiFileFormat::Type0
    }
}

/// A track for export
#[derive(Debug, Clone)]
pub struct ExportTrack {
    /// Track name
    pub name: String,
    /// MIDI channel (0-15)
    pub channel: u8,
    /// Notes in this track
    pub notes: Vec<ExportNote>,
    /// Program change at start (None = no change)
    pub program: Option<u8>,
}

impl ExportTrack {
    /// Create a new export track
    pub fn new(name: impl Into<String>, channel: u8) -> Self {
        Self {
            name: name.into(),
            channel,
            notes: Vec::new(),
            program: None,
        }
    }

    /// Add a note
    pub fn add_note(&mut self, note: ExportNote) {
        self.notes.push(note);
    }

    /// Add notes from frozen notes
    pub fn add_frozen_notes(&mut self, notes: &[FrozenNote]) {
        for frozen in notes {
            self.notes.push(ExportNote {
                tick: frozen.start_tick,
                note: frozen.note,
                velocity: frozen.velocity,
                duration: frozen.duration,
            });
        }
    }

    /// Set program
    pub fn with_program(mut self, program: u8) -> Self {
        self.program = Some(program);
        self
    }

    /// Sort notes by tick
    pub fn sort(&mut self) {
        self.notes.sort_by_key(|n| n.tick);
    }
}

/// A note for export
#[derive(Debug, Clone)]
pub struct ExportNote {
    /// Start tick
    pub tick: u64,
    /// Note number (0-127)
    pub note: u8,
    /// Velocity (1-127)
    pub velocity: u8,
    /// Duration in ticks
    pub duration: u64,
}

impl ExportNote {
    /// Create a new export note
    pub fn new(tick: u64, note: u8, velocity: u8, duration: u64) -> Self {
        Self {
            tick,
            note,
            velocity,
            duration,
        }
    }

    /// End tick
    pub fn end_tick(&self) -> u64 {
        self.tick + self.duration
    }
}

/// MIDI event for export
#[derive(Debug, Clone)]
struct MidiExportEvent {
    /// Absolute tick
    tick: u64,
    /// Event data
    data: Vec<u8>,
}

impl MidiExportEvent {
    fn note_on(tick: u64, channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            tick,
            data: vec![0x90 | (channel & 0x0F), note & 0x7F, velocity & 0x7F],
        }
    }

    fn note_off(tick: u64, channel: u8, note: u8) -> Self {
        Self {
            tick,
            data: vec![0x80 | (channel & 0x0F), note & 0x7F, 0],
        }
    }

    fn program_change(tick: u64, channel: u8, program: u8) -> Self {
        Self {
            tick,
            data: vec![0xC0 | (channel & 0x0F), program & 0x7F],
        }
    }

    fn tempo(tick: u64, bpm: f64) -> Self {
        let microseconds = (60_000_000.0 / bpm) as u32;
        Self {
            tick,
            data: vec![
                0xFF, 0x51, 0x03,
                ((microseconds >> 16) & 0xFF) as u8,
                ((microseconds >> 8) & 0xFF) as u8,
                (microseconds & 0xFF) as u8,
            ],
        }
    }

    fn time_signature(tick: u64, numerator: u8, denominator: u8) -> Self {
        // Denominator is expressed as power of 2
        let denom_power = (denominator as f64).log2() as u8;
        Self {
            tick,
            data: vec![
                0xFF, 0x58, 0x04,
                numerator,
                denom_power,
                24, // MIDI clocks per metronome click
                8,  // 32nd notes per MIDI quarter note
            ],
        }
    }

    fn track_name(tick: u64, name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut data = vec![0xFF, 0x03, bytes.len() as u8];
        data.extend_from_slice(bytes);
        Self { tick, data }
    }

    fn end_of_track() -> Self {
        Self {
            tick: 0, // Will be set correctly during writing
            data: vec![0xFF, 0x2F, 0x00],
        }
    }
}

/// MIDI file exporter
pub struct MidiExporter {
    /// File format
    format: MidiFileFormat,
    /// PPQN (ticks per quarter note)
    ppqn: u16,
    /// Tempo in BPM
    tempo: f64,
    /// Time signature
    time_sig: (u8, u8),
    /// Tracks to export
    tracks: Vec<ExportTrack>,
}

impl MidiExporter {
    /// Create a new exporter
    pub fn new() -> Self {
        Self {
            format: MidiFileFormat::Type0,
            ppqn: 480,
            tempo: 120.0,
            time_sig: (4, 4),
            tracks: Vec::new(),
        }
    }

    /// Get format
    pub fn format(&self) -> MidiFileFormat {
        self.format
    }

    /// Set format
    pub fn set_format(&mut self, format: MidiFileFormat) {
        self.format = format;
    }

    /// Set PPQN
    pub fn set_ppqn(&mut self, ppqn: u16) {
        self.ppqn = ppqn.max(1);
    }

    /// Get PPQN
    pub fn ppqn(&self) -> u16 {
        self.ppqn
    }

    /// Set tempo
    pub fn set_tempo(&mut self, bpm: f64) {
        self.tempo = bpm.clamp(20.0, 300.0);
    }

    /// Get tempo
    pub fn tempo(&self) -> f64 {
        self.tempo
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, numerator: u8, denominator: u8) {
        self.time_sig = (numerator.max(1), denominator.max(1));
    }

    /// Get time signature
    pub fn time_signature(&self) -> (u8, u8) {
        self.time_sig
    }

    /// Add a track
    pub fn add_track(&mut self, track: ExportTrack) {
        self.tracks.push(track);
    }

    /// Clear tracks
    pub fn clear_tracks(&mut self) {
        self.tracks.clear();
    }

    /// Get tracks
    pub fn tracks(&self) -> &[ExportTrack] {
        &self.tracks
    }

    /// Export to file
    pub fn export<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        self.write(&mut file)
    }

    /// Export to bytes
    pub fn export_to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.write(&mut buffer).expect("Write to vec should not fail");
        buffer
    }

    /// Write MIDI data to writer
    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self.format {
            MidiFileFormat::Type0 => self.write_type0(writer),
            MidiFileFormat::Type1 => self.write_type1(writer),
        }
    }

    /// Write Type 0 MIDI file (single track)
    fn write_type0<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Collect all events
        let mut events = Vec::new();

        // Add tempo and time signature
        events.push(MidiExportEvent::tempo(0, self.tempo));
        events.push(MidiExportEvent::time_signature(0, self.time_sig.0, self.time_sig.1));

        // Add all track events
        for track in &self.tracks {
            if let Some(program) = track.program {
                events.push(MidiExportEvent::program_change(0, track.channel, program));
            }

            for note in &track.notes {
                events.push(MidiExportEvent::note_on(
                    note.tick,
                    track.channel,
                    note.note,
                    note.velocity,
                ));
                events.push(MidiExportEvent::note_off(
                    note.end_tick(),
                    track.channel,
                    note.note,
                ));
            }
        }

        // Sort by tick
        events.sort_by_key(|e| e.tick);

        // Write header
        self.write_header(writer, 0, 1)?;

        // Write single track
        self.write_track(writer, &events)?;

        Ok(())
    }

    /// Write Type 1 MIDI file (multiple tracks)
    fn write_type1<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let num_tracks = self.tracks.len() + 1; // +1 for tempo track

        // Write header
        self.write_header(writer, 1, num_tracks as u16)?;

        // Write tempo track
        let mut tempo_events = Vec::new();
        tempo_events.push(MidiExportEvent::tempo(0, self.tempo));
        tempo_events.push(MidiExportEvent::time_signature(0, self.time_sig.0, self.time_sig.1));
        tempo_events.push(MidiExportEvent::track_name(0, "Tempo"));
        self.write_track(writer, &tempo_events)?;

        // Write each track
        for track in &self.tracks {
            let mut events = Vec::new();

            events.push(MidiExportEvent::track_name(0, &track.name));

            if let Some(program) = track.program {
                events.push(MidiExportEvent::program_change(0, track.channel, program));
            }

            for note in &track.notes {
                events.push(MidiExportEvent::note_on(
                    note.tick,
                    track.channel,
                    note.note,
                    note.velocity,
                ));
                events.push(MidiExportEvent::note_off(
                    note.end_tick(),
                    track.channel,
                    note.note,
                ));
            }

            events.sort_by_key(|e| e.tick);
            self.write_track(writer, &events)?;
        }

        Ok(())
    }

    /// Write MIDI file header chunk
    fn write_header<W: Write>(&self, writer: &mut W, format: u16, num_tracks: u16) -> io::Result<()> {
        // MThd
        writer.write_all(b"MThd")?;
        // Chunk length (always 6)
        writer.write_all(&[0, 0, 0, 6])?;
        // Format type
        writer.write_all(&format.to_be_bytes())?;
        // Number of tracks
        writer.write_all(&num_tracks.to_be_bytes())?;
        // PPQN
        writer.write_all(&self.ppqn.to_be_bytes())?;
        Ok(())
    }

    /// Write a track chunk
    fn write_track<W: Write>(&self, writer: &mut W, events: &[MidiExportEvent]) -> io::Result<()> {
        // Build track data
        let mut track_data = Vec::new();
        let mut last_tick = 0u64;

        for event in events {
            let delta = event.tick.saturating_sub(last_tick);
            self.write_variable_length(&mut track_data, delta as u32)?;
            track_data.extend_from_slice(&event.data);
            last_tick = event.tick;
        }

        // End of track
        let end_event = MidiExportEvent::end_of_track();
        self.write_variable_length(&mut track_data, 0)?;
        track_data.extend_from_slice(&end_event.data);

        // MTrk
        writer.write_all(b"MTrk")?;
        // Track length
        let length = track_data.len() as u32;
        writer.write_all(&length.to_be_bytes())?;
        // Track data
        writer.write_all(&track_data)?;

        Ok(())
    }

    /// Write variable-length quantity
    fn write_variable_length<W: Write>(&self, writer: &mut W, mut value: u32) -> io::Result<()> {
        let mut bytes = Vec::new();

        bytes.push((value & 0x7F) as u8);
        value >>= 7;

        while value > 0 {
            bytes.push((value & 0x7F) as u8 | 0x80);
            value >>= 7;
        }

        bytes.reverse();
        writer.write_all(&bytes)
    }

    /// Scale ticks from source PPQN to export PPQN
    pub fn scale_ticks(&self, tick: u64, source_ppqn: u32) -> u64 {
        if source_ppqn == self.ppqn as u32 {
            tick
        } else {
            (tick as f64 * self.ppqn as f64 / source_ppqn as f64) as u64
        }
    }
}

impl Default for MidiExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_creation() {
        let exporter = MidiExporter::new();
        assert_eq!(exporter.format(), MidiFileFormat::Type0);
        assert_eq!(exporter.ppqn(), 480);
        assert_eq!(exporter.tempo(), 120.0);
    }

    #[test]
    fn test_export_note() {
        let note = ExportNote::new(0, 60, 100, 480);
        assert_eq!(note.end_tick(), 480);
    }

    #[test]
    fn test_export_track() {
        let mut track = ExportTrack::new("Piano", 0);
        track.add_note(ExportNote::new(0, 60, 100, 480));
        track.add_note(ExportNote::new(480, 64, 90, 480));

        assert_eq!(track.notes.len(), 2);
    }

    #[test]
    fn test_export_type0() {
        let mut exporter = MidiExporter::new();
        exporter.set_format(MidiFileFormat::Type0);
        exporter.set_ppqn(24);
        exporter.set_tempo(120.0);

        let mut track = ExportTrack::new("Test", 0);
        track.add_note(ExportNote::new(0, 60, 100, 24));
        exporter.add_track(track);

        let bytes = exporter.export_to_bytes();

        // Check header
        assert_eq!(&bytes[0..4], b"MThd");
        assert_eq!(bytes[9], 0); // Format 0
        assert_eq!(&bytes[12..14], &24u16.to_be_bytes()); // PPQN

        // Check track chunk exists
        assert_eq!(&bytes[14..18], b"MTrk");
    }

    #[test]
    fn test_export_type1() {
        let mut exporter = MidiExporter::new();
        exporter.set_format(MidiFileFormat::Type1);
        exporter.set_ppqn(24);

        let mut track1 = ExportTrack::new("Track 1", 0);
        track1.add_note(ExportNote::new(0, 60, 100, 24));
        exporter.add_track(track1);

        let mut track2 = ExportTrack::new("Track 2", 1);
        track2.add_note(ExportNote::new(0, 64, 90, 24));
        exporter.add_track(track2);

        let bytes = exporter.export_to_bytes();

        // Check header
        assert_eq!(&bytes[0..4], b"MThd");
        assert_eq!(bytes[9], 1); // Format 1
        assert_eq!(&bytes[10..12], &3u16.to_be_bytes()); // 3 tracks
    }

    #[test]
    fn test_variable_length() {
        let exporter = MidiExporter::new();
        let mut buffer = Vec::new();

        // Test various values
        exporter.write_variable_length(&mut buffer, 0).unwrap();
        assert_eq!(buffer, vec![0x00]);

        buffer.clear();
        exporter.write_variable_length(&mut buffer, 127).unwrap();
        assert_eq!(buffer, vec![0x7F]);

        buffer.clear();
        exporter.write_variable_length(&mut buffer, 128).unwrap();
        assert_eq!(buffer, vec![0x81, 0x00]);

        buffer.clear();
        exporter.write_variable_length(&mut buffer, 16383).unwrap();
        assert_eq!(buffer, vec![0xFF, 0x7F]);
    }

    #[test]
    fn test_scale_ticks() {
        let mut exporter = MidiExporter::new();
        exporter.set_ppqn(480);

        // 24 PPQN -> 480 PPQN
        assert_eq!(exporter.scale_ticks(24, 24), 480);
        assert_eq!(exporter.scale_ticks(48, 24), 960);

        // Same PPQN
        assert_eq!(exporter.scale_ticks(480, 480), 480);
    }

    #[test]
    fn test_add_frozen_notes() {
        let frozen = vec![
            FrozenNote {
                channel: 0,
                note: 60,
                velocity: 100,
                start_tick: 0,
                duration: 24,
            },
            FrozenNote {
                channel: 0,
                note: 64,
                velocity: 90,
                start_tick: 24,
                duration: 24,
            },
        ];

        let mut track = ExportTrack::new("Test", 0);
        track.add_frozen_notes(&frozen);

        assert_eq!(track.notes.len(), 2);
        assert_eq!(track.notes[0].note, 60);
        assert_eq!(track.notes[1].note, 64);
    }

    #[test]
    fn test_tempo_event() {
        let event = MidiExportEvent::tempo(0, 120.0);
        // 120 BPM = 500000 microseconds per beat
        assert_eq!(event.data[0], 0xFF);
        assert_eq!(event.data[1], 0x51);
        assert_eq!(event.data[2], 0x03);
        // 500000 = 0x07A120
        assert_eq!(event.data[3], 0x07);
        assert_eq!(event.data[4], 0xA1);
        assert_eq!(event.data[5], 0x20);
    }

    #[test]
    fn test_track_with_program() {
        let track = ExportTrack::new("Strings", 0).with_program(48);
        assert_eq!(track.program, Some(48));
    }

    #[test]
    fn test_time_signature() {
        let mut exporter = MidiExporter::new();
        exporter.set_time_signature(3, 4);
        assert_eq!(exporter.time_signature(), (3, 4));
    }
}
