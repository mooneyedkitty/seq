// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI Input handling for receiving messages from controllers.
//!
//! This module provides functionality for receiving MIDI input,
//! parsing messages, MIDI learn mode, and external clock synchronization.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use coremidi::{Client, InputPort, PacketList, Source, Sources};

use super::messages;

/// Parsed MIDI message types
#[derive(Debug, Clone, PartialEq)]
pub enum MidiMessage {
    /// Note On: channel (0-15), note (0-127), velocity (0-127)
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// Note Off: channel (0-15), note (0-127), velocity (0-127)
    NoteOff { channel: u8, note: u8, velocity: u8 },
    /// Control Change: channel (0-15), controller (0-127), value (0-127)
    ControlChange { channel: u8, controller: u8, value: u8 },
    /// Program Change: channel (0-15), program (0-127)
    ProgramChange { channel: u8, program: u8 },
    /// Pitch Bend: channel (0-15), value (-8192 to 8191)
    PitchBend { channel: u8, value: i16 },
    /// Channel Aftertouch: channel (0-15), pressure (0-127)
    ChannelAftertouch { channel: u8, pressure: u8 },
    /// Poly Aftertouch: channel (0-15), note (0-127), pressure (0-127)
    PolyAftertouch { channel: u8, note: u8, pressure: u8 },
    /// MIDI Clock tick
    TimingClock,
    /// Start playback
    Start,
    /// Continue playback
    Continue,
    /// Stop playback
    Stop,
    /// Unknown/unparsed message
    Unknown(Vec<u8>),
}

impl MidiMessage {
    /// Parse raw MIDI bytes into a MidiMessage
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let status = data[0];

        // System Real-Time messages (single byte)
        match status {
            messages::TIMING_CLOCK => return Some(MidiMessage::TimingClock),
            messages::START => return Some(MidiMessage::Start),
            messages::CONTINUE => return Some(MidiMessage::Continue),
            messages::STOP => return Some(MidiMessage::Stop),
            _ => {}
        }

        // Channel messages
        let msg_type = status & 0xF0;
        let channel = status & 0x0F;

        match msg_type {
            messages::NOTE_OFF if data.len() >= 3 => Some(MidiMessage::NoteOff {
                channel,
                note: data[1] & 0x7F,
                velocity: data[2] & 0x7F,
            }),
            messages::NOTE_ON if data.len() >= 3 => {
                let velocity = data[2] & 0x7F;
                // Note On with velocity 0 is equivalent to Note Off
                if velocity == 0 {
                    Some(MidiMessage::NoteOff {
                        channel,
                        note: data[1] & 0x7F,
                        velocity: 0,
                    })
                } else {
                    Some(MidiMessage::NoteOn {
                        channel,
                        note: data[1] & 0x7F,
                        velocity,
                    })
                }
            }
            messages::CONTROL_CHANGE if data.len() >= 3 => Some(MidiMessage::ControlChange {
                channel,
                controller: data[1] & 0x7F,
                value: data[2] & 0x7F,
            }),
            messages::PROGRAM_CHANGE if data.len() >= 2 => Some(MidiMessage::ProgramChange {
                channel,
                program: data[1] & 0x7F,
            }),
            messages::PITCH_BEND if data.len() >= 3 => {
                let lsb = data[1] as i16;
                let msb = data[2] as i16;
                let value = ((msb << 7) | lsb) - 8192;
                Some(MidiMessage::PitchBend { channel, value })
            }
            messages::CHANNEL_AFTERTOUCH if data.len() >= 2 => {
                Some(MidiMessage::ChannelAftertouch {
                    channel,
                    pressure: data[1] & 0x7F,
                })
            }
            messages::POLY_AFTERTOUCH if data.len() >= 3 => Some(MidiMessage::PolyAftertouch {
                channel,
                note: data[1] & 0x7F,
                pressure: data[2] & 0x7F,
            }),
            _ => Some(MidiMessage::Unknown(data.to_vec())),
        }
    }

    /// Check if this is a clock-related message
    pub fn is_clock_message(&self) -> bool {
        matches!(
            self,
            MidiMessage::TimingClock
                | MidiMessage::Start
                | MidiMessage::Continue
                | MidiMessage::Stop
        )
    }
}

/// MIDI Learn state for capturing controller assignments
#[derive(Debug, Clone)]
pub struct MidiLearnCapture {
    /// The captured message (if any)
    pub message: Option<MidiMessage>,
    /// Whether we're actively learning
    pub active: bool,
}

impl MidiLearnCapture {
    pub fn new() -> Self {
        Self {
            message: None,
            active: false,
        }
    }

    /// Start learning mode
    pub fn start(&mut self) {
        self.active = true;
        self.message = None;
    }

    /// Stop learning mode
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Capture a message if in learn mode
    /// Returns true if the message was captured
    pub fn capture(&mut self, message: &MidiMessage) -> bool {
        if !self.active {
            return false;
        }

        // Only capture CC, Note, and Program Change for learning
        match message {
            MidiMessage::ControlChange { .. }
            | MidiMessage::NoteOn { .. }
            | MidiMessage::NoteOff { .. }
            | MidiMessage::ProgramChange { .. } => {
                self.message = Some(message.clone());
                self.active = false;
                true
            }
            _ => false,
        }
    }
}

impl Default for MidiLearnCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// External clock synchronization state
#[derive(Debug, Clone)]
pub struct ExternalClockSync {
    /// Whether sync is enabled
    pub enabled: bool,
    /// Count of clock ticks received
    pub tick_count: u64,
    /// Whether we've received a start message
    pub running: bool,
}

impl ExternalClockSync {
    pub fn new() -> Self {
        Self {
            enabled: false,
            tick_count: 0,
            running: false,
        }
    }

    /// Enable external clock sync
    pub fn enable(&mut self) {
        self.enabled = true;
        self.tick_count = 0;
        self.running = false;
    }

    /// Disable external clock sync
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Process a clock message
    pub fn process(&mut self, message: &MidiMessage) {
        if !self.enabled {
            return;
        }

        match message {
            MidiMessage::TimingClock => {
                if self.running {
                    self.tick_count += 1;
                }
            }
            MidiMessage::Start => {
                self.running = true;
                self.tick_count = 0;
            }
            MidiMessage::Continue => {
                self.running = true;
            }
            MidiMessage::Stop => {
                self.running = false;
            }
            _ => {}
        }
    }

    /// Get current beat (based on 24 PPQN)
    pub fn current_beat(&self) -> u64 {
        self.tick_count / 24
    }

    /// Get current pulse within beat (0-23)
    pub fn current_pulse(&self) -> u64 {
        self.tick_count % 24
    }
}

impl Default for ExternalClockSync {
    fn default() -> Self {
        Self::new()
    }
}

/// MIDI Input handler using Core MIDI
pub struct MidiInput {
    _client: Client,
    _input_port: InputPort,
    receiver: Receiver<MidiMessage>,
    midi_learn: Arc<Mutex<MidiLearnCapture>>,
    clock_sync: Arc<Mutex<ExternalClockSync>>,
}

impl MidiInput {
    /// Create a new MIDI input connected to the specified source
    pub fn new(source_index: usize) -> Result<Self> {
        let client = Client::new("SEQ Input")
            .map_err(|e| anyhow!("Failed to create MIDI client: {:?}", e))?;

        let source = Source::from_index(source_index)
            .ok_or_else(|| anyhow!("MIDI source {} not found", source_index))?;

        let (tx, rx): (Sender<MidiMessage>, Receiver<MidiMessage>) = mpsc::channel();

        let midi_learn = Arc::new(Mutex::new(MidiLearnCapture::new()));
        let clock_sync = Arc::new(Mutex::new(ExternalClockSync::new()));

        let learn_clone = midi_learn.clone();
        let sync_clone = clock_sync.clone();

        // Create input port with callback
        let input_port = client
            .input_port("SEQ Input Port", move |packet_list: &PacketList| {
                for packet in packet_list.iter() {
                    let data = packet.data();
                    if let Some(msg) = MidiMessage::parse(data) {
                        // Process MIDI learn
                        if let Ok(mut learn) = learn_clone.lock() {
                            learn.capture(&msg);
                        }

                        // Process clock sync
                        if let Ok(mut sync) = sync_clone.lock() {
                            sync.process(&msg);
                        }

                        // Send to receiver
                        let _ = tx.send(msg);
                    }
                }
            })
            .map_err(|e| anyhow!("Failed to create input port: {:?}", e))?;

        // Connect the input port to the source
        input_port
            .connect_source(&source)
            .map_err(|e| anyhow!("Failed to connect to source: {:?}", e))?;

        Ok(Self {
            _client: client,
            _input_port: input_port,
            receiver: rx,
            midi_learn,
            clock_sync,
        })
    }

    /// Try to receive the next MIDI message (non-blocking)
    pub fn try_recv(&self) -> Option<MidiMessage> {
        self.receiver.try_recv().ok()
    }

    /// Receive all pending MIDI messages
    pub fn recv_all(&self) -> Vec<MidiMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Start MIDI learn mode
    pub fn start_learn(&self) {
        if let Ok(mut learn) = self.midi_learn.lock() {
            learn.start();
        }
    }

    /// Stop MIDI learn mode
    pub fn stop_learn(&self) {
        if let Ok(mut learn) = self.midi_learn.lock() {
            learn.stop();
        }
    }

    /// Get the learned message (if any)
    pub fn get_learned(&self) -> Option<MidiMessage> {
        self.midi_learn.lock().ok()?.message.clone()
    }

    /// Check if learn mode is active
    pub fn is_learning(&self) -> bool {
        self.midi_learn.lock().map(|l| l.active).unwrap_or(false)
    }

    /// Enable external clock sync
    pub fn enable_clock_sync(&self) {
        if let Ok(mut sync) = self.clock_sync.lock() {
            sync.enable();
        }
    }

    /// Disable external clock sync
    pub fn disable_clock_sync(&self) {
        if let Ok(mut sync) = self.clock_sync.lock() {
            sync.disable();
        }
    }

    /// Get current clock sync state
    pub fn clock_sync_state(&self) -> Option<ExternalClockSync> {
        self.clock_sync.lock().ok().map(|s| s.clone())
    }
}

/// List all available MIDI sources
pub fn list_sources() -> Vec<(usize, String)> {
    let mut result = Vec::new();

    for (i, source) in Sources.into_iter().enumerate() {
        let name = source.display_name().unwrap_or_else(|| format!("Unknown {}", i));
        result.push((i, name));
    }

    result
}

/// Get the number of available MIDI sources
pub fn source_count() -> usize {
    Sources::count()
}

/// Print all available MIDI sources to stdout
pub fn print_sources() {
    let sources = list_sources();
    if sources.is_empty() {
        println!("No MIDI sources found.");
    } else {
        println!("Available MIDI sources (inputs):");
        for (i, name) in sources {
            println!("  {}: {}", i, name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let msg = MidiMessage::parse(&[0x90, 60, 100]);
        assert_eq!(
            msg,
            Some(MidiMessage::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );
    }

    #[test]
    fn test_parse_note_on_velocity_zero() {
        // Note On with velocity 0 should be treated as Note Off
        let msg = MidiMessage::parse(&[0x90, 60, 0]);
        assert_eq!(
            msg,
            Some(MidiMessage::NoteOff {
                channel: 0,
                note: 60,
                velocity: 0
            })
        );
    }

    #[test]
    fn test_parse_note_off() {
        let msg = MidiMessage::parse(&[0x80, 60, 64]);
        assert_eq!(
            msg,
            Some(MidiMessage::NoteOff {
                channel: 0,
                note: 60,
                velocity: 64
            })
        );
    }

    #[test]
    fn test_parse_control_change() {
        let msg = MidiMessage::parse(&[0xB0, 1, 64]); // Mod wheel
        assert_eq!(
            msg,
            Some(MidiMessage::ControlChange {
                channel: 0,
                controller: 1,
                value: 64
            })
        );
    }

    #[test]
    fn test_parse_program_change() {
        let msg = MidiMessage::parse(&[0xC0, 5]);
        assert_eq!(
            msg,
            Some(MidiMessage::ProgramChange {
                channel: 0,
                program: 5
            })
        );
    }

    #[test]
    fn test_parse_pitch_bend() {
        // Center position (0)
        let msg = MidiMessage::parse(&[0xE0, 0x00, 0x40]);
        assert_eq!(
            msg,
            Some(MidiMessage::PitchBend {
                channel: 0,
                value: 0
            })
        );
    }

    #[test]
    fn test_parse_clock_messages() {
        assert_eq!(
            MidiMessage::parse(&[0xF8]),
            Some(MidiMessage::TimingClock)
        );
        assert_eq!(MidiMessage::parse(&[0xFA]), Some(MidiMessage::Start));
        assert_eq!(MidiMessage::parse(&[0xFB]), Some(MidiMessage::Continue));
        assert_eq!(MidiMessage::parse(&[0xFC]), Some(MidiMessage::Stop));
    }

    #[test]
    fn test_midi_learn() {
        let mut learn = MidiLearnCapture::new();
        assert!(!learn.active);

        learn.start();
        assert!(learn.active);

        let msg = MidiMessage::ControlChange {
            channel: 0,
            controller: 1,
            value: 64,
        };

        assert!(learn.capture(&msg));
        assert!(!learn.active);
        assert_eq!(learn.message, Some(msg));
    }

    #[test]
    fn test_external_clock_sync() {
        let mut sync = ExternalClockSync::new();
        sync.enable();

        // Start
        sync.process(&MidiMessage::Start);
        assert!(sync.running);
        assert_eq!(sync.tick_count, 0);

        // 24 ticks = 1 beat
        for _ in 0..24 {
            sync.process(&MidiMessage::TimingClock);
        }
        assert_eq!(sync.current_beat(), 1);
        assert_eq!(sync.current_pulse(), 0);

        // Stop
        sync.process(&MidiMessage::Stop);
        assert!(!sync.running);
    }

    #[test]
    fn test_list_sources() {
        // Just verify it doesn't panic
        let sources = list_sources();
        println!("Found {} sources", sources.len());
    }
}
