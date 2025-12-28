// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI I/O abstraction layer.
//!
//! This module provides a trait-based abstraction for MIDI input and output,
//! allowing different backends (Core MIDI, midir, etc.) to be used
//! interchangeably.

pub mod coremidi_backend;
pub mod input;

use anyhow::Result;

pub use coremidi_backend::{CoreMidiOutput, list_destinations, print_destinations};
pub use input::{
    list_sources, print_sources, ExternalClockSync, MidiInput, MidiLearnCapture, MidiMessage,
};

/// Trait for MIDI output implementations.
///
/// This trait abstracts over different MIDI backends, providing a unified
/// interface for sending MIDI messages with optional timestamps.
pub trait MidiOutput: Send {
    /// Send a MIDI message immediately.
    ///
    /// # Arguments
    /// * `message` - Raw MIDI bytes (e.g., `[0x90, 60, 127]` for Note On)
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if the message could not be sent
    fn send(&mut self, message: &[u8]) -> Result<()>;

    /// Send a MIDI message at a specific timestamp.
    ///
    /// # Arguments
    /// * `message` - Raw MIDI bytes
    /// * `timestamp` - Timestamp in microseconds (host time)
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if the message could not be sent
    fn send_at(&mut self, message: &[u8], timestamp: u64) -> Result<()>;
}

/// MIDI message constants
pub mod messages {
    // Channel Voice Messages (upper nibble, lower nibble is channel 0-15)
    pub const NOTE_OFF: u8 = 0x80;
    pub const NOTE_ON: u8 = 0x90;
    pub const POLY_AFTERTOUCH: u8 = 0xA0;
    pub const CONTROL_CHANGE: u8 = 0xB0;
    pub const PROGRAM_CHANGE: u8 = 0xC0;
    pub const CHANNEL_AFTERTOUCH: u8 = 0xD0;
    pub const PITCH_BEND: u8 = 0xE0;

    // System Real-Time Messages
    pub const TIMING_CLOCK: u8 = 0xF8;
    pub const START: u8 = 0xFA;
    pub const CONTINUE: u8 = 0xFB;
    pub const STOP: u8 = 0xFC;

    // System Common Messages
    pub const SYSEX_START: u8 = 0xF0;
    pub const SYSEX_END: u8 = 0xF7;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock MIDI output for testing
    struct MockMidiOutput {
        messages: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl MockMidiOutput {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_messages(&self) -> Vec<Vec<u8>> {
            self.messages.lock().unwrap().clone()
        }
    }

    impl MidiOutput for MockMidiOutput {
        fn send(&mut self, message: &[u8]) -> Result<()> {
            self.messages.lock().unwrap().push(message.to_vec());
            Ok(())
        }

        fn send_at(&mut self, message: &[u8], _timestamp: u64) -> Result<()> {
            self.messages.lock().unwrap().push(message.to_vec());
            Ok(())
        }
    }

    #[test]
    fn test_mock_midi_output_send() {
        let mut output = MockMidiOutput::new();

        // Send a Note On message
        output.send(&[messages::NOTE_ON, 60, 127]).unwrap();

        let messages = output.get_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], vec![0x90, 60, 127]);
    }

    #[test]
    fn test_mock_midi_output_send_at() {
        let mut output = MockMidiOutput::new();

        // Send a Note Off message with timestamp
        output.send_at(&[messages::NOTE_OFF, 60, 0], 1000000).unwrap();

        let messages = output.get_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], vec![0x80, 60, 0]);
    }

    #[test]
    fn test_midi_message_constants() {
        assert_eq!(messages::NOTE_ON, 0x90);
        assert_eq!(messages::NOTE_OFF, 0x80);
        assert_eq!(messages::TIMING_CLOCK, 0xF8);
        assert_eq!(messages::START, 0xFA);
        assert_eq!(messages::STOP, 0xFC);
    }
}
