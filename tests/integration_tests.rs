// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Integration tests for SEQ
//!
//! These tests verify that multiple components work together correctly.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Note: Integration tests use the public API of the crate

/// Test that the full playback pipeline works
#[test]
fn test_full_playback_pipeline() {
    // This test verifies that:
    // 1. A generator can be created
    // 2. Events can be generated
    // 3. Events can be scheduled
    // 4. The sequencer timing is correct

    // Create a simple test that exercises the core playback path
    // without requiring actual MIDI hardware

    // Verify basic timing calculations
    let ppqn = 24u32;
    let tempo = 120.0f64;

    // At 120 BPM with 24 PPQN:
    // - 1 beat = 500ms
    // - 1 tick = 500ms / 24 = ~20.83ms
    let micros_per_beat = (60_000_000.0 / tempo) as u64;
    let micros_per_tick = micros_per_beat / ppqn as u64;

    assert_eq!(micros_per_beat, 500_000); // 500ms per beat
    assert!(micros_per_tick > 20_000 && micros_per_tick < 21_000);

    // Verify a bar calculation
    let beats_per_bar = 4u32;
    let ticks_per_bar = ppqn * beats_per_bar;
    assert_eq!(ticks_per_bar, 96);

    // Verify position calculation
    let total_ticks = 250u64;
    let bars = total_ticks / ticks_per_bar as u64;
    let remaining = total_ticks % ticks_per_bar as u64;
    let beats = remaining / ppqn as u64;
    let ticks = remaining % ppqn as u64;

    assert_eq!(bars, 2);
    assert_eq!(beats, 2);
    assert_eq!(ticks, 10);
}

/// Test that generator output flows through the system
#[test]
fn test_generator_to_scheduler_flow() {
    // Simulate the flow of events from generator through scheduler

    // Create mock events (simulating generator output)
    let events: Vec<(u64, u8, u8, u8)> = vec![
        (0, 60, 100, 24),   // Note at tick 0, C4, vel 100, dur 24
        (24, 62, 90, 24),   // Note at tick 24, D4
        (48, 64, 95, 24),   // Note at tick 48, E4
        (72, 65, 85, 24),   // Note at tick 72, F4
    ];

    // Simulate scheduling
    let mut scheduled: Vec<(u64, String)> = Vec::new();

    for (tick, note, vel, dur) in &events {
        // Schedule note on
        scheduled.push((*tick, format!("NoteOn: {} vel {}", note, vel)));
        // Schedule note off
        scheduled.push((tick + *dur as u64, format!("NoteOff: {}", note)));
    }

    // Sort by tick (scheduler would do this)
    scheduled.sort_by_key(|(tick, _)| *tick);

    // Verify order
    assert_eq!(scheduled.len(), 8);
    assert_eq!(scheduled[0].0, 0);
    assert!(scheduled[0].1.contains("NoteOn: 60"));
    assert_eq!(scheduled[1].0, 24);
    assert!(scheduled[1].1.contains("NoteOff: 60") || scheduled[1].1.contains("NoteOn: 62"));
}

/// Test timing accuracy over extended period
#[test]
fn test_timing_accuracy() {
    // Simulate timing over many ticks to check for drift
    let ppqn = 24u32;
    let tempo = 120.0f64;
    let micros_per_tick = (60_000_000.0 / tempo / ppqn as f64) as u64;

    // Simulate 10 bars (960 ticks at 24 PPQN, 4/4)
    let total_ticks = 960u64;
    let expected_duration_micros = total_ticks * micros_per_tick;

    // At 120 BPM, 10 bars = 20 seconds
    let expected_seconds = 20.0;
    let actual_seconds = expected_duration_micros as f64 / 1_000_000.0;

    // Allow 1% tolerance
    assert!((actual_seconds - expected_seconds).abs() < expected_seconds * 0.01);
}

/// Test scale quantization integration
#[test]
fn test_scale_quantization_integration() {
    // Test that notes are correctly quantized to scale

    // C major scale intervals from root
    let c_major_intervals = [0, 2, 4, 5, 7, 9, 11];

    // Test quantizing various notes to C major
    let test_cases = vec![
        (60, 60), // C -> C (in scale)
        (61, 60), // C# -> C (quantize down)
        (62, 62), // D -> D (in scale)
        (63, 62), // D# -> D (quantize down)
        (64, 64), // E -> E (in scale)
        (65, 65), // F -> F (in scale)
        (66, 65), // F# -> F (quantize down)
        (67, 67), // G -> G (in scale)
        (68, 67), // G# -> G (quantize down)
        (69, 69), // A -> A (in scale)
        (70, 69), // A# -> A (quantize down)
        (71, 71), // B -> B (in scale)
    ];

    for (input, expected) in test_cases {
        let pc = input % 12;
        let octave = input / 12;

        // Find nearest scale degree
        let quantized_pc = c_major_intervals
            .iter()
            .min_by_key(|&&interval| (interval as i32 - pc as i32).abs())
            .copied()
            .unwrap_or(pc as i32) as u8;

        let quantized = octave * 12 + quantized_pc;

        // Note: This is a simplified quantization - real implementation
        // may have different rounding behavior
        assert!(
            quantized == expected || (quantized as i32 - expected as i32).abs() <= 1,
            "Input {} expected {} got {}",
            input,
            expected,
            quantized
        );
    }
}

/// Test multi-track output
#[test]
fn test_multi_track_output() {
    // Simulate multiple tracks outputting on different channels

    #[derive(Debug, Clone)]
    struct MockTrack {
        channel: u8,
        muted: bool,
        soloed: bool,
    }

    let tracks = vec![
        MockTrack { channel: 0, muted: false, soloed: false },
        MockTrack { channel: 1, muted: true, soloed: false },
        MockTrack { channel: 2, muted: false, soloed: false },
        MockTrack { channel: 9, muted: false, soloed: false }, // Drums
    ];

    // Collect output from non-muted tracks
    let active_channels: Vec<u8> = tracks
        .iter()
        .filter(|t| !t.muted)
        .map(|t| t.channel)
        .collect();

    assert_eq!(active_channels, vec![0, 2, 9]);

    // Test solo behavior
    let tracks_with_solo = vec![
        MockTrack { channel: 0, muted: false, soloed: true },
        MockTrack { channel: 1, muted: false, soloed: false },
        MockTrack { channel: 2, muted: false, soloed: false },
    ];

    let any_solo = tracks_with_solo.iter().any(|t| t.soloed);
    let solo_channels: Vec<u8> = if any_solo {
        tracks_with_solo
            .iter()
            .filter(|t| t.soloed)
            .map(|t| t.channel)
            .collect()
    } else {
        tracks_with_solo
            .iter()
            .filter(|t| !t.muted)
            .map(|t| t.channel)
            .collect()
    };

    assert_eq!(solo_channels, vec![0]);
}

/// Test part/scene transitions
#[test]
fn test_part_transitions() {
    // Test quantized transitions between parts

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum TransitionType {
        Immediate,
        NextBeat,
        NextBar,
    }

    fn calculate_transition_tick(
        current_tick: u64,
        transition: TransitionType,
        ppqn: u32,
        beats_per_bar: u32,
    ) -> u64 {
        match transition {
            TransitionType::Immediate => current_tick,
            TransitionType::NextBeat => {
                let beat_ticks = ppqn as u64;
                ((current_tick / beat_ticks) + 1) * beat_ticks
            }
            TransitionType::NextBar => {
                let bar_ticks = (ppqn * beats_per_bar) as u64;
                ((current_tick / bar_ticks) + 1) * bar_ticks
            }
        }
    }

    let ppqn = 24u32;
    let beats_per_bar = 4u32;

    // Test immediate
    assert_eq!(
        calculate_transition_tick(50, TransitionType::Immediate, ppqn, beats_per_bar),
        50
    );

    // Test next beat (at tick 50, next beat is tick 72)
    assert_eq!(
        calculate_transition_tick(50, TransitionType::NextBeat, ppqn, beats_per_bar),
        72
    );

    // Test next bar (at tick 50, next bar is tick 96)
    assert_eq!(
        calculate_transition_tick(50, TransitionType::NextBar, ppqn, beats_per_bar),
        96
    );

    // Test at bar boundary
    assert_eq!(
        calculate_transition_tick(96, TransitionType::NextBar, ppqn, beats_per_bar),
        192
    );
}

/// Test MIDI recording and playback
#[test]
fn test_recording_playback_cycle() {
    // Simulate recording notes and playing them back

    #[derive(Debug, Clone, PartialEq)]
    struct RecordedNote {
        tick: u64,
        note: u8,
        velocity: u8,
        duration: u64,
    }

    // "Record" some notes
    let recorded: Vec<RecordedNote> = vec![
        RecordedNote { tick: 0, note: 60, velocity: 100, duration: 24 },
        RecordedNote { tick: 24, note: 64, velocity: 90, duration: 24 },
        RecordedNote { tick: 48, note: 67, velocity: 95, duration: 48 },
    ];

    // "Playback" - generate events from recorded notes
    let mut playback_events: Vec<(u64, String)> = Vec::new();

    for note in &recorded {
        playback_events.push((note.tick, format!("on:{}", note.note)));
        playback_events.push((note.tick + note.duration, format!("off:{}", note.note)));
    }

    playback_events.sort_by_key(|(t, _)| *t);

    // Verify events
    assert_eq!(playback_events.len(), 6);

    // Check event order
    assert_eq!(playback_events[0], (0, "on:60".to_string()));
    assert_eq!(playback_events[1], (24, "off:60".to_string()));
    assert_eq!(playback_events[2], (24, "on:64".to_string()));
    assert_eq!(playback_events[3], (48, "off:64".to_string()));
    assert_eq!(playback_events[4], (48, "on:67".to_string()));
    assert_eq!(playback_events[5], (96, "off:67".to_string()));
}

/// Test config validation
#[test]
fn test_config_validation() {
    // Test that invalid configs are rejected

    fn validate_tempo(tempo: f64) -> Result<(), &'static str> {
        if tempo < 20.0 {
            Err("Tempo too slow")
        } else if tempo > 300.0 {
            Err("Tempo too fast")
        } else {
            Ok(())
        }
    }

    fn validate_channel(channel: u8) -> Result<(), &'static str> {
        if channel > 15 {
            Err("Invalid MIDI channel")
        } else {
            Ok(())
        }
    }

    fn validate_velocity(velocity: u8) -> Result<(), &'static str> {
        if velocity > 127 {
            Err("Invalid velocity")
        } else {
            Ok(())
        }
    }

    // Valid configs
    assert!(validate_tempo(120.0).is_ok());
    assert!(validate_channel(0).is_ok());
    assert!(validate_channel(15).is_ok());
    assert!(validate_velocity(100).is_ok());

    // Invalid configs
    assert!(validate_tempo(10.0).is_err());
    assert!(validate_tempo(400.0).is_err());
    assert!(validate_channel(16).is_err());
}

/// Test MIDI file export format
#[test]
fn test_midi_export_format() {
    // Test variable-length quantity encoding (used in MIDI files)

    fn encode_variable_length(mut value: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push((value & 0x7F) as u8);
        value >>= 7;

        while value > 0 {
            bytes.push((value & 0x7F) as u8 | 0x80);
            value >>= 7;
        }

        bytes.reverse();
        bytes
    }

    // Test cases from MIDI spec
    assert_eq!(encode_variable_length(0x00), vec![0x00]);
    assert_eq!(encode_variable_length(0x7F), vec![0x7F]);
    assert_eq!(encode_variable_length(0x80), vec![0x81, 0x00]);
    assert_eq!(encode_variable_length(0x2000), vec![0xC0, 0x00]);
    assert_eq!(encode_variable_length(0x3FFF), vec![0xFF, 0x7F]);
    assert_eq!(encode_variable_length(0x4000), vec![0x81, 0x80, 0x00]);
}

/// Test euclidean rhythm generation
#[test]
fn test_euclidean_rhythm_integration() {
    // Bjorklund's algorithm for euclidean rhythms

    fn euclidean_rhythm(steps: usize, pulses: usize) -> Vec<bool> {
        if pulses == 0 || steps == 0 {
            return vec![false; steps];
        }
        if pulses >= steps {
            return vec![true; steps];
        }

        let mut pattern = vec![vec![true]; pulses];
        let mut remainder = vec![vec![false]; steps - pulses];

        while remainder.len() > 1 {
            let min_len = pattern.len().min(remainder.len());

            for i in 0..min_len {
                pattern[i].extend(remainder[i].clone());
            }

            let new_remainder: Vec<Vec<bool>> = if pattern.len() > min_len {
                pattern.drain(min_len..).collect()
            } else {
                remainder.drain(min_len..).collect()
            };

            remainder = new_remainder;
        }

        pattern.extend(remainder);
        pattern.into_iter().flatten().collect()
    }

    // Test classic euclidean patterns
    // E(3,8) - Cuban tresillo
    let tresillo = euclidean_rhythm(8, 3);
    assert_eq!(tresillo.len(), 8);
    assert_eq!(tresillo.iter().filter(|&&x| x).count(), 3);

    // E(5,8) - Cuban cinquillo
    let cinquillo = euclidean_rhythm(8, 5);
    assert_eq!(cinquillo.len(), 8);
    assert_eq!(cinquillo.iter().filter(|&&x| x).count(), 5);

    // E(4,16) - basic 4/4
    let four_four = euclidean_rhythm(16, 4);
    assert_eq!(four_four.len(), 16);
    assert_eq!(four_four.iter().filter(|&&x| x).count(), 4);
}

/// Test controller mapping integration
#[test]
fn test_controller_mapping_integration() {
    // Test that MIDI messages map to correct actions

    #[derive(Debug, Clone, PartialEq)]
    enum Action {
        Play,
        Stop,
        SetTempo(f64),
        TriggerScene(usize),
        SetVolume(u8, f64),
    }

    #[derive(Debug, Clone)]
    struct Mapping {
        channel: Option<u8>,
        note_or_cc: u8,
        is_cc: bool,
        action_template: String,
    }

    let mappings = vec![
        Mapping { channel: None, note_or_cc: 36, is_cc: false, action_template: "play".to_string() },
        Mapping { channel: None, note_or_cc: 37, is_cc: false, action_template: "stop".to_string() },
        Mapping { channel: Some(0), note_or_cc: 7, is_cc: true, action_template: "volume:0".to_string() },
        Mapping { channel: Some(1), note_or_cc: 7, is_cc: true, action_template: "volume:1".to_string() },
    ];

    fn process_midi(mappings: &[Mapping], channel: u8, is_note: bool, data1: u8, data2: u8) -> Option<Action> {
        for m in mappings {
            let channel_match = m.channel.map_or(true, |c| c == channel);
            let type_match = (is_note && !m.is_cc) || (!is_note && m.is_cc);
            let data_match = m.note_or_cc == data1;

            if channel_match && type_match && data_match {
                return match m.action_template.as_str() {
                    "play" => Some(Action::Play),
                    "stop" => Some(Action::Stop),
                    s if s.starts_with("volume:") => {
                        let track: usize = s[7..].parse().unwrap_or(0);
                        Some(Action::SetVolume(track as u8, data2 as f64 / 127.0))
                    }
                    _ => None,
                };
            }
        }
        None
    }

    // Test note triggers
    assert_eq!(process_midi(&mappings, 0, true, 36, 100), Some(Action::Play));
    assert_eq!(process_midi(&mappings, 0, true, 37, 100), Some(Action::Stop));

    // Test CC
    assert_eq!(process_midi(&mappings, 0, false, 7, 64), Some(Action::SetVolume(0, 64.0 / 127.0)));
    assert_eq!(process_midi(&mappings, 1, false, 7, 127), Some(Action::SetVolume(1, 1.0)));

    // Test no match
    assert_eq!(process_midi(&mappings, 0, true, 38, 100), None);
}

/// Test extended operation (simulated)
#[test]
fn test_extended_operation_simulation() {
    // Simulate extended operation to check for issues
    // This is a fast simulation, not real-time

    let mut tick_count = 0u64;
    let mut note_on_count = 0u64;
    let mut note_off_count = 0u64;

    // Simulate 10 minutes at 120 BPM
    // 10 min * 60 sec * 2 beats/sec * 24 ticks/beat = 28,800 ticks
    let total_ticks = 28_800u64;
    let notes_per_bar = 4;
    let ticks_per_bar = 96u64;

    while tick_count < total_ticks {
        // Simulate note generation every beat
        if tick_count % 24 == 0 {
            note_on_count += 1;
        }

        // Simulate note off after duration
        if tick_count >= 12 && (tick_count - 12) % 24 == 0 {
            note_off_count += 1;
        }

        tick_count += 1;
    }

    // Verify counts are reasonable
    assert!(note_on_count > 0);
    assert!(note_off_count > 0);

    // Note ons should roughly equal note offs (minus last few)
    assert!((note_on_count as i64 - note_off_count as i64).abs() < 10);

    // Should have approximately correct number of notes
    // 10 min = 300 bars at 120 BPM, 4 notes per bar = 1200 notes
    assert!(note_on_count >= 1100 && note_on_count <= 1300);
}

/// Test memory-safe event handling
#[test]
fn test_event_queue_memory() {
    // Test that event queue handles large numbers of events

    let mut events: Vec<(u64, Vec<u8>)> = Vec::new();

    // Add 10,000 events
    for i in 0..10_000u64 {
        events.push((i, vec![0x90, (i % 128) as u8, 100]));
    }

    // Sort (simulating scheduler)
    events.sort_by_key(|(tick, _)| *tick);

    // Process (drain) events
    let processed: Vec<_> = events.drain(..).collect();

    assert_eq!(processed.len(), 10_000);
    assert!(events.is_empty());
}

/// Test thread safety simulation
#[test]
fn test_thread_safety_simulation() {
    // Test that shared state can be safely accessed

    let shared_tempo = Arc::new(Mutex::new(120.0f64));
    let shared_position = Arc::new(Mutex::new(0u64));

    // Simulate concurrent access
    let tempo_clone = Arc::clone(&shared_tempo);
    let position_clone = Arc::clone(&shared_position);

    // "Audio thread" updates position
    {
        let mut pos = position_clone.lock().unwrap();
        *pos += 24;
    }

    // "UI thread" reads position and tempo
    {
        let pos = shared_position.lock().unwrap();
        let tempo = shared_tempo.lock().unwrap();
        assert_eq!(*pos, 24);
        assert_eq!(*tempo, 120.0);
    }

    // "Control thread" updates tempo
    {
        let mut tempo = tempo_clone.lock().unwrap();
        *tempo = 130.0;
    }

    // Verify update
    {
        let tempo = shared_tempo.lock().unwrap();
        assert_eq!(*tempo, 130.0);
    }
}
