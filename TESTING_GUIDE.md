# SEQ Testing Guide

A comprehensive step-by-step guide to testing all features of the SEQ algorithmic MIDI sequencer.

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Building and Running Tests](#2-building-and-running-tests)
3. [MIDI Device Testing](#3-midi-device-testing)
4. [Timing and Clock Testing](#4-timing-and-clock-testing)
5. [Generator Testing](#5-generator-testing)
6. [Sequencer Testing](#6-sequencer-testing)
7. [Arrangement Testing](#7-arrangement-testing)
8. [Recording Testing](#8-recording-testing)
9. [Configuration Testing](#9-configuration-testing)
10. [Control System Testing](#10-control-system-testing)
11. [Audio Engine Testing](#11-audio-engine-testing)
12. [Performance Testing](#12-performance-testing)
13. [Integration Testing](#13-integration-testing)

---

## 1. Prerequisites

### 1.1 System Requirements

- macOS (for Core MIDI support)
- Rust 1.70 or later
- At least one MIDI device (hardware or virtual)

### 1.2 Install Rust

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### 1.3 Clone and Build

```bash
cd /Users/rsnyder/Development/seq
cargo build
```

### 1.4 Virtual MIDI Setup (Optional)

If you don't have hardware MIDI devices, create a virtual MIDI bus:

1. Open **Audio MIDI Setup** (Applications > Utilities)
2. Go to **Window > Show MIDI Studio**
3. Double-click **IAC Driver**
4. Check **Device is online**
5. Add a port named "SEQ Test"

This creates a virtual MIDI loopback for testing.

---

## 2. Building and Running Tests

### 2.1 Run All Unit Tests

```bash
# Run all 282 unit tests
cargo test

# Expected output: "test result: ok. 282 passed"
```

### 2.2 Run Tests with Output

```bash
# See println! output from tests
cargo test -- --nocapture
```

### 2.3 Run Specific Test Modules

```bash
# Test generators only
cargo test generators::

# Test sequencer only
cargo test sequencer::

# Test timing/clock
cargo test timing::

# Test recording
cargo test recording::

# Test arrangement
cargo test arrangement::

# Test configuration
cargo test config::

# Test control system
cargo test control::

# Test audio
cargo test audio::

# Test UI
cargo test ui::

# Test music theory
cargo test music::
```

### 2.4 Run Integration Tests

```bash
# Run the 14 integration tests
cargo test --test integration_tests

# Expected output: "test result: ok. 14 passed"
```

### 2.5 Run a Single Test

```bash
# Run one specific test by name
cargo test test_clock_creation

# Run tests matching a pattern
cargo test arpeggio
```

---

## 3. MIDI Device Testing

### 3.1 List Available MIDI Outputs

```bash
cargo run -- --list-midi
```

**Expected output:**
```
Available MIDI Destinations:
  0: IAC Driver SEQ Test
  1: Your Hardware Synth
  ...
```

**What to verify:**
- Your MIDI devices appear in the list
- Virtual MIDI ports (IAC Driver) are visible
- Device indices are numbered correctly

### 3.2 List Available MIDI Inputs

```bash
cargo run -- --list-sources
```

**Expected output:**
```
Available MIDI Sources:
  0: IAC Driver SEQ Test
  1: Your MIDI Controller
  ...
```

**What to verify:**
- MIDI input devices appear
- Controllers and keyboards are listed

### 3.3 Send Test Note

```bash
# Replace 0 with your device index
cargo run -- --test-note 0
```

**Expected output:**
```
Connecting to MIDI destination 0...
Sending test note (Middle C, velocity 100)...
Note On sent
Note Off sent
Test complete!
```

**What to verify:**
- If using a synth: You hear Middle C (C4) for 500ms
- If using MIDI Monitor app: See Note On 60 vel 100, then Note Off

### 3.4 Send MIDI Clock

```bash
# Send clock at 120 BPM to destination 0 for 4 beats
cargo run -- --test-clock 0 120
```

**Expected output:**
```
Connecting to MIDI destination 0...
Sending MIDI clock at 120 BPM for 4 beats...
PPQN: 24, Pulse interval: 20.83ms
START sent
Beat 1
Beat 2
Beat 3
Beat 4
STOP sent
Clock test complete! Sent 4 beats at 120 BPM
```

**What to verify:**
- Clock messages are sent at correct rate
- External gear syncs to the clock
- 4 beats take exactly 2 seconds at 120 BPM

### 3.5 Monitor MIDI Input

```bash
# Monitor input from source 0 for 30 seconds
cargo run -- --monitor 0
```

**What to do:**
1. Play notes on your MIDI controller
2. Turn knobs/move faders
3. Press buttons

**Expected output:**
```
Connecting to MIDI source 0...
Monitoring MIDI input (press Ctrl+C to stop)...

NoteOn { channel: 0, note: 60, velocity: 100 }
NoteOff { channel: 0, note: 60, velocity: 0 }
ControlChange { channel: 0, controller: 1, value: 64 }
...
```

**What to verify:**
- Note On/Off messages are captured correctly
- CC messages show correct controller numbers
- Channel numbers are correct (0-15)

---

## 4. Timing and Clock Testing

### 4.1 Test Clock Unit Tests

```bash
cargo test timing::clock::tests
```

**Tests include:**
- `test_clock_creation` - Clock initializes correctly
- `test_clock_bpm_clamping` - BPM stays within 20-300 range
- `test_clock_start_stop` - Start/stop behavior
- `test_clock_pause_continue` - Pause/continue behavior
- `test_clock_tick` - Tick generation
- `test_pulse_interval` - Timing accuracy
- `test_nudge_bpm` - Tempo nudging
- `test_tap_tempo` - Tap tempo calculation
- `test_tempo_ramp` - Gradual tempo changes

### 4.2 Verify Timing Calculations

The timing system uses 24 PPQN. Verify these calculations:

| BPM | Beat Duration | Tick Duration | Bar Duration (4/4) |
|-----|---------------|---------------|-------------------|
| 60  | 1000ms        | 41.67ms       | 4000ms            |
| 120 | 500ms         | 20.83ms       | 2000ms            |
| 180 | 333ms         | 13.89ms       | 1333ms            |

### 4.3 Test Tap Tempo

```bash
cargo test test_tap_tempo
```

**What it tests:**
- Averaging of tap intervals
- Ignoring outliers
- Reset after timeout

---

## 5. Generator Testing

### 5.1 Test All Generators

```bash
cargo test generators::
```

**Expected:** All generator tests pass

### 5.2 Drone Generator Tests

```bash
cargo test generators::drone::tests
```

**Tests include:**
- `test_drone_creation` - Default parameters
- `test_drone_generate` - Note generation
- `test_drone_voice_count` - Multiple voices
- `test_drone_octave_spread` - Octave range
- `test_drone_velocity` - Velocity settings
- `test_drone_set_param` - Parameter changes

**What to verify:**
- Notes stay within scale
- Voice count matches parameter
- Octave spread is respected

### 5.3 Arpeggiator Tests

```bash
cargo test generators::arpeggio::tests
```

**Tests include:**
- `test_arpeggio_creation` - Default pattern
- `test_arpeggio_up_pattern` - Ascending notes
- `test_arpeggio_down_pattern` - Descending notes
- `test_arpeggio_updown_pattern` - Bounce pattern
- `test_arpeggio_octave_range` - Multi-octave
- `test_arpeggio_gate` - Note length
- `test_euclidean_pattern` - Euclidean rhythms

**Pattern verification:**

| Pattern | Chord [C,E,G] Output |
|---------|---------------------|
| Up      | C, E, G, C, E, G... |
| Down    | G, E, C, G, E, C... |
| UpDown  | C, E, G, E, C, E... |
| Random  | Random order        |

### 5.4 Chord Generator Tests

```bash
cargo test generators::chord::tests
```

**Tests include:**
- `test_chord_creation` - Default voicing
- `test_chord_close_voicing` - Close position
- `test_chord_open_voicing` - Open position
- `test_chord_drop2_voicing` - Drop-2
- `test_chord_functional_progression` - I-IV-V-I
- `test_chord_extensions` - 7ths, 9ths
- `test_chord_inversions` - Root, 1st, 2nd

**Voicing verification:**

| Voicing | C Major Notes |
|---------|---------------|
| Close   | C4, E4, G4    |
| Open    | C3, G3, E4    |
| Drop2   | G3, C4, E4    |

### 5.5 Melody Generator Tests

```bash
cargo test generators::melody::tests
```

**Tests include:**
- `test_melody_creation` - Initial state
- `test_melody_generate` - Note generation
- `test_melody_range` - Note range limits
- `test_melody_intervals` - Interval probabilities
- `test_melody_transforms` - Transpose, invert, retrograde
- `test_melody_phrase_repeat` - Phrase structure
- `test_melody_reset` - State reset

### 5.6 Drum Generator Tests

```bash
cargo test generators::drums::tests
```

**Tests include:**
- `test_drum_creation` - Default style
- `test_drum_four_on_floor` - Kick pattern
- `test_drum_breakbeat` - Syncopated pattern
- `test_drum_euclidean` - Euclidean distribution
- `test_drum_humanize` - Timing/velocity variation
- `test_drum_fill` - Fill generation
- `test_drum_voices` - Multi-voice patterns
- `test_drum_style_change` - Style switching

**GM Drum Map verification:**

| Note | Drum       |
|------|------------|
| 36   | Kick       |
| 38   | Snare      |
| 42   | Closed HH  |
| 46   | Open HH    |
| 49   | Crash      |
| 51   | Ride       |

---

## 6. Sequencer Testing

### 6.1 Test All Sequencer Components

```bash
cargo test sequencer::
```

### 6.2 Scheduler Tests

```bash
cargo test sequencer::scheduler::tests
```

**Tests include:**
- `test_scheduler_creation` - Initial state
- `test_schedule_events` - Event scheduling
- `test_event_ordering` - Priority queue
- `test_tempo_change` - Tempo updates
- `test_start_stop` - Transport control
- `test_seek` - Position seeking
- `test_time_signature` - Time sig changes
- `test_clear_queue` - Queue clearing
- `test_midi_bytes` - MIDI byte generation

**What to verify:**
- Events fire in correct order
- Tempo changes don't cause timing glitches
- Seek works correctly

### 6.3 Track Tests

```bash
cargo test sequencer::track::tests
```

**Tests include:**
- `test_track_creation` - Default state
- `test_track_config` - Channel, transpose, swing
- `test_mute_solo` - Mute/solo behavior
- `test_track_manager_mute` - Multi-track muting
- `test_track_manager_solo` - Solo behavior
- `test_transpose` - Note transposition
- `test_transpose_out_of_range` - Range clamping
- `test_velocity_scaling` - Velocity adjustment
- `test_swing_application` - Swing timing

**Solo behavior verification:**
- When any track is soloed, only soloed tracks output
- Multiple tracks can be soloed
- Unmuting a track doesn't affect solo state

### 6.4 Clip Tests

```bash
cargo test sequencer::clip::tests
```

**Tests include:**
- `test_clip_creation` - Default clip
- `test_clip_builder` - Builder pattern
- `test_clip_add_notes` - Note management
- `test_clip_loop_mode` - Loop settings
- `test_clip_oneshot` - One-shot playback
- `test_clip_loop_count` - Counted loops
- `test_clip_pingpong` - Ping-pong mode
- `test_clip_loop_points` - Start/end points
- `test_clip_generated` - Generator-based
- `test_clip_hybrid` - Mixed mode
- `test_clip_state_transitions` - State machine
- `test_clip_stop_at_end` - End behavior

**Loop mode verification:**

| Mode | Behavior |
|------|----------|
| OneShot | Play once, stop |
| Loop | Repeat forever |
| LoopCount(n) | Repeat n times |
| PingPong | Forward, backward, repeat |

### 6.5 Trigger Tests

```bash
cargo test sequencer::trigger::tests
```

**Tests include:**
- `test_trigger_queue` - Queue operations
- `test_trigger_queue_poll` - Polling behavior
- `test_trigger_queue_immediate` - Immediate triggers
- `test_quantize_immediate` - No quantization
- `test_quantize_beat` - Beat quantization
- `test_quantize_bar` - Bar quantization
- `test_quantize_phrase` - Phrase quantization
- `test_quantize_at_boundary` - Boundary handling
- `test_follow_action_next` - Next clip
- `test_follow_action_previous` - Previous clip
- `test_follow_action_specific` - Named clip
- `test_cancel_for_track` - Cancel pending
- `test_scene` - Scene triggers
- `test_scene_manager` - Scene management

**Quantization verification:**

| Mode | Current Tick 50 | Trigger Tick |
|------|-----------------|--------------|
| Immediate | - | 50 |
| Beat (24 PPQN) | - | 72 |
| Bar (96 ticks) | - | 96 |
| Beats(2) | - | 72 |
| Bars(2) | - | 192 |

---

## 7. Arrangement Testing

### 7.1 Test All Arrangement Components

```bash
cargo test arrangement::
```

### 7.2 Parts System Tests

```bash
cargo test arrangement::part::tests
```

**Tests include:**
- `test_part_creation` - Create empty part
- `test_part_track_states` - Set track states
- `test_part_transition` - Transition types
- `test_part_macros` - Macro actions
- `test_part_manager_create` - Manager creation
- `test_part_manager_add` - Add parts
- `test_part_manager_trigger` - Part triggering
- `test_part_manager_quantized` - Quantized transitions
- `test_part_manager_pending` - Pending transitions
- `test_part_manager_update` - Update processing
- `test_part_nav` - Next/previous navigation

**Track clip states:**

| State | Behavior |
|-------|----------|
| Empty | No clip |
| Clip(n) | Play clip n |
| Generator(name) | Run generator |
| Stop | Stop playback |
| Hold | Keep current |

### 7.3 Scene System Tests

```bash
cargo test arrangement::scene::tests
```

**Tests include:**
- `test_scene_creation` - Create scene
- `test_scene_slots` - Set slots
- `test_scene_launch_mode` - Launch modes
- `test_scene_follow_action` - Follow actions
- `test_scene_manager_create` - Manager creation
- `test_scene_manager_add` - Add scenes
- `test_scene_manager_launch` - Launch scene
- `test_scene_manager_nav` - Navigation
- `test_scene_manager_pending` - Pending launches
- `test_scene_matrix` - Track x scene matrix

### 7.4 Song Mode Tests

```bash
cargo test arrangement::song::tests
```

**Tests include:**
- `test_song_creation` - Create song
- `test_song_sections` - Add sections
- `test_song_position` - Position tracking
- `test_song_player_create` - Player creation
- `test_song_player_load` - Load song
- `test_song_player_play` - Playback
- `test_song_player_stop` - Stop behavior
- `test_song_player_goto` - Jump to section
- `test_song_player_loop` - Loop regions
- `test_song_player_update` - Update processing

---

## 8. Recording Testing

### 8.1 Test All Recording Components

```bash
cargo test recording::
```

### 8.2 MIDI Capture Tests

```bash
cargo test recording::capture::tests
```

**Tests include:**
- `test_recorder_creation` - Initial state
- `test_recorder_start_stop` - Basic recording
- `test_recorder_arm` - Armed state
- `test_record_notes` - Note capture
- `test_note_on_off_pairing` - Note matching
- `test_active_notes_on_stop` - Close active notes
- `test_overdub_mode` - Overdub recording
- `test_punch_region` - Punch in/out
- `test_count_in` - Count-in beats
- `test_loop_recording` - Loop wrapping
- `test_quantize_grid` - Grid quantization
- `test_quantize_strength` - Partial quantization
- `test_recorded_note` - Note structure

**Record modes:**

| Mode | Behavior |
|------|----------|
| Replace | Clear existing, record new |
| Overdub | Add to existing |
| Punch | Record only in region |

### 8.3 Clip Freeze Tests

```bash
cargo test recording::freeze::tests
```

**Tests include:**
- `test_freezer_creation` - Initial state
- `test_freezer_start_stop` - Capture control
- `test_freeze_events` - Event capture
- `test_freeze_auto_complete` - Auto-finish
- `test_freeze_options_bars` - Bar-based length
- `test_freeze_quantization` - Grid quantization
- `test_frozen_note` - Note structure
- `test_merge_overlapping` - Note merging
- `test_cancel_freeze` - Cancel operation
- `test_min_note_length` - Filter short notes

### 8.4 MIDI Export Tests

```bash
cargo test recording::export::tests
```

**Tests include:**
- `test_exporter_creation` - Initial state
- `test_exporter_format` - Type 0/1 selection
- `test_add_track` - Track management
- `test_export_type0` - Single track export
- `test_export_type1` - Multi-track export
- `test_variable_length` - VLQ encoding
- `test_meta_events` - Tempo, time sig

**MIDI file format:**

| Type | Description |
|------|-------------|
| Type 0 | Single track, all channels |
| Type 1 | Multiple tracks, one per channel |

---

## 9. Configuration Testing

### 9.1 Test Configuration System

```bash
cargo test config::
```

### 9.2 Config Structure Tests

```bash
cargo test config::tests
```

**Tests include:**
- `test_song_config` - Song configuration
- `test_track_config` - Track settings
- `test_generator_config` - Generator params
- `test_part_config` - Part definitions
- `test_yaml_round_trip` - Serialize/deserialize
- `test_yaml_load` - File loading
- `test_yaml_save` - File saving

### 9.3 Hot Reload Tests

```bash
cargo test config::watcher::tests
```

**Tests include:**
- `test_watcher_creation` - Create watcher
- `test_watcher_detects_changes` - Change detection
- `test_watcher_debounce` - Debouncing
- `test_watcher_validate` - Validation before reload
- `test_config_event_types` - Event handling

**What to verify:**
- File changes are detected
- Rapid changes are debounced (500ms default)
- Invalid configs don't break playback

### 9.4 Sample Configuration Test

Create a test config file:

```yaml
# test_config.yaml
name: "Test Song"
tempo: 120.0
time_signature: [4, 4]
key: C
scale: Major

tracks:
  - name: "Lead"
    channel: 0
    generator:
      type: arpeggio
      pattern: up
```

Test loading:

```bash
# From Rust code or test
cargo test test_yaml_load
```

---

## 10. Control System Testing

### 10.1 Test All Control Components

```bash
cargo test control::
```

### 10.2 Keyboard Controller Tests

```bash
cargo test control::keyboard::tests
```

**Tests include:**
- `test_keyboard_creation` - Initial state
- `test_shortcut_matching` - Key matching
- `test_default_bindings` - Default shortcuts
- `test_custom_binding` - Custom bindings
- `test_key_categories` - Binding categories
- `test_process_key` - Key processing
- `test_modifier_keys` - Ctrl, Alt, Shift
- `test_action_generation` - Action output
- `test_key_repeat` - Repeat handling
- `test_binding_override` - Override bindings

**Default shortcuts:**

| Key | Action |
|-----|--------|
| Space | Play/Pause |
| Escape | Stop |
| 1-9 | Trigger parts |
| Up | Tempo up |
| Down | Tempo down |
| M | Mute modifier |
| S | Solo modifier |

### 10.3 MIDI Controller Tests

```bash
cargo test control::midi_map::tests
```

**Tests include:**
- `test_midi_controller_creation` - Initial state
- `test_note_binding` - Note triggers
- `test_cc_binding` - CC mappings
- `test_channel_filter` - Channel filtering
- `test_encoder_modes` - Encoder handling
- `test_mapping_layers` - Layer switching
- `test_midi_learn` - Learn mode
- `test_action_generation` - Action output
- `test_sensitivity` - CC sensitivity

**Encoder modes:**

| Mode | Behavior |
|------|----------|
| Absolute | 0-127 direct |
| Relative64 | 64 = no change, <64 = down, >64 = up |
| RelativeBinary | 0-63 = down, 64-127 = up |
| RelativeSigned | 1-64 = up, 65-127 = down (signed) |

### 10.4 Parameter System Tests

```bash
cargo test control::params::tests
```

**Tests include:**
- `test_parameter_creation` - Create param
- `test_parameter_range` - Min/max clamping
- `test_parameter_default` - Default values
- `test_parameter_smoothing` - Value smoothing
- `test_parameter_normalized` - 0.0-1.0 access
- `test_parameter_registry` - Registry management
- `test_preset_tempo` - Tempo param
- `test_preset_volume` - Volume param
- `test_preset_pan` - Pan param
- `test_preset_filter` - Filter param
- `test_parameter_groups` - Grouped params
- `test_parameter_units` - Unit display

---

## 11. Audio Engine Testing

### 11.1 Test Audio Components

```bash
cargo test audio::
```

### 11.2 FluidSynth Tests

```bash
cargo test audio::fluidsynth::tests
```

**Tests include:**
- `test_synth_creation` - Create synth
- `test_synth_sample_rate` - Sample rate config
- `test_note_on_off` - Note handling
- `test_control_change` - CC handling
- `test_program_change` - Program selection
- `test_pitch_bend` - Pitch bend
- `test_all_notes_off` - Panic function
- `test_render_buffer` - Audio rendering

### 11.3 Audio Output Tests

```bash
cargo test audio::output::tests
```

**Tests include:**
- `test_list_devices` - Device enumeration
- `test_default_device_name` - Default device
- `test_supported_sample_rates` - Rate detection
- `test_audio_config` - Configuration
- `test_latency_calculation` - Latency calc

### 11.4 Manual Audio Test

To test audio output with a soundfont:

1. Place a .sf2 file in the project directory
2. Modify code to load soundfont and play notes
3. Verify audio plays through speakers

---

## 12. Performance Testing

### 12.1 Run All Benchmarks

```bash
cargo bench
```

**Note:** First run takes longer to compile.

### 12.2 Run Specific Benchmarks

```bash
# Timing benchmark
cargo bench tick_to_micros

# Event queue benchmark
cargo bench event_queue

# Scale quantization
cargo bench scale_quantize

# Note processing
cargo bench note_processing

# MIDI parsing
cargo bench midi_parsing
```

### 12.3 Benchmark Results Interpretation

| Benchmark | Target | Description |
|-----------|--------|-------------|
| tick_to_micros | <10ns | Core timing conversion |
| event_queue/insert | <1μs | Scheduling events |
| event_queue/drain | <10μs/1000 | Processing queue |
| vlq_encoding | <100ns | MIDI file encoding |
| scale_quantize | <1μs/60 notes | Scale quantization |
| note_processing | <100μs/1000 | Full processing |
| jitter_measurement | informational | Timing accuracy |
| memory/preallocated | faster | Buffer strategy |
| position_calc | <10μs/1000 | Position conversion |
| midi_parsing | <1μs/msg | MIDI parsing |
| quantization | <100ns | Note quantization |

### 12.4 Extended Stability Test

Run for extended period to check stability:

```bash
# Run the extended operation test
cargo test test_extended_operation_simulation -- --nocapture
```

This simulates 10 minutes of operation at 120 BPM.

---

## 13. Integration Testing

### 13.1 Run All Integration Tests

```bash
cargo test --test integration_tests
```

### 13.2 Individual Integration Tests

```bash
# Full playback pipeline
cargo test --test integration_tests test_full_playback_pipeline

# Generator to scheduler flow
cargo test --test integration_tests test_generator_to_scheduler_flow

# Timing accuracy
cargo test --test integration_tests test_timing_accuracy

# Scale quantization
cargo test --test integration_tests test_scale_quantization_integration

# Multi-track output
cargo test --test integration_tests test_multi_track_output

# Part transitions
cargo test --test integration_tests test_part_transitions

# Recording/playback cycle
cargo test --test integration_tests test_recording_playback_cycle

# Config validation
cargo test --test integration_tests test_config_validation

# MIDI export format
cargo test --test integration_tests test_midi_export_format

# Euclidean rhythms
cargo test --test integration_tests test_euclidean_rhythm_integration

# Controller mapping
cargo test --test integration_tests test_controller_mapping_integration

# Extended operation
cargo test --test integration_tests test_extended_operation_simulation

# Memory safety
cargo test --test integration_tests test_event_queue_memory

# Thread safety
cargo test --test integration_tests test_thread_safety_simulation
```

### 13.3 Integration Test Coverage

| Test | Components Covered |
|------|--------------------|
| full_playback_pipeline | Timing, sequencer |
| generator_to_scheduler_flow | Generators, scheduler |
| timing_accuracy | Clock, timing |
| scale_quantization | Music theory, generators |
| multi_track_output | Tracks, mute/solo |
| part_transitions | Parts, quantization |
| recording_playback_cycle | Recording, playback |
| config_validation | Configuration |
| midi_export_format | MIDI export |
| euclidean_rhythm | Drums, algorithms |
| controller_mapping | Control system |
| extended_operation | Stability |
| event_queue_memory | Memory management |
| thread_safety | Concurrency |

---

## Summary Checklist

Use this checklist to verify complete testing:

### Build & Tests
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (282 tests)
- [ ] `cargo test --test integration_tests` passes (14 tests)
- [ ] `cargo bench` runs without errors

### MIDI
- [ ] `--list-midi` shows devices
- [ ] `--list-sources` shows inputs
- [ ] `--test-note` plays on synth
- [ ] `--test-clock` syncs external gear
- [ ] `--monitor` captures input

### Generators
- [ ] Drone tests pass
- [ ] Arpeggio tests pass
- [ ] Chord tests pass
- [ ] Melody tests pass
- [ ] Drum tests pass

### Sequencer
- [ ] Scheduler tests pass
- [ ] Track tests pass
- [ ] Clip tests pass
- [ ] Trigger tests pass

### Arrangement
- [ ] Part tests pass
- [ ] Scene tests pass
- [ ] Song tests pass

### Recording
- [ ] Capture tests pass
- [ ] Freeze tests pass
- [ ] Export tests pass

### Control
- [ ] Keyboard tests pass
- [ ] MIDI mapping tests pass
- [ ] Parameter tests pass

### Performance
- [ ] Benchmarks run
- [ ] Timing < 10ns
- [ ] No memory leaks

---

## Troubleshooting

### Tests Fail to Compile

```bash
# Clean and rebuild
cargo clean
cargo build
cargo test
```

### MIDI Devices Not Found

1. Check Audio MIDI Setup
2. Restart Core Audio: `sudo killall coreaudiod`
3. Verify device is connected and powered

### Benchmark Errors

```bash
# Ensure release profile
cargo bench --release
```

### Audio Issues

1. Check system audio settings
2. Verify sample rate compatibility
3. Check soundfont path

---

## Contributing

When adding new features, ensure:

1. Unit tests are added
2. Integration tests updated if needed
3. All existing tests still pass
4. Benchmarks added for performance-critical code
5. Documentation updated
