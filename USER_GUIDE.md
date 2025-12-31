# SEQ User Guide

A complete guide to using SEQ, the algorithmic MIDI sequencer for live performance.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Quick Start](#3-quick-start)
4. [Core Concepts](#4-core-concepts)
5. [Generators](#5-generators)
6. [Tracks and Clips](#6-tracks-and-clips)
7. [Parts and Scenes](#7-parts-and-scenes)
8. [Song Mode](#8-song-mode)
9. [Recording](#9-recording)
10. [MIDI Export](#10-midi-export)
11. [Configuration](#11-configuration)
12. [MIDI Controllers](#12-midi-controllers)
13. [Keyboard Shortcuts](#13-keyboard-shortcuts)
14. [Live Performance](#14-live-performance)
15. [Tips and Best Practices](#15-tips-and-best-practices)
16. [Reference](#16-reference)

---

## 1. Introduction

### What is SEQ?

SEQ is an algorithmic MIDI sequencer designed for live electronic music performance. Unlike traditional DAW sequencers that play back pre-recorded patterns, SEQ generates music in real-time using configurable algorithms.

### Key Features

- **Generative Music**: Five algorithmic engines create evolving musical content
- **Live Performance**: Parts, scenes, and quantized transitions for structured improvisation
- **Minimal UI**: Operated primarily via MIDI controllers and keyboard
- **Hot Reload**: Change configurations without stopping playback
- **Scale-Aware**: All output respects musical scales and keys
- **MIDI Clock**: Sync with external gear as master or slave

### Target Use Cases

- Ambient and drone music
- Synthwave and synthpop
- Live electronic performance
- Generative installations
- Practice and composition tool

### Philosophy

SEQ embraces constraints as creative tools. By defining rules and probabilities rather than specific notes, you create systems that surprise you while staying musically coherent.

---

## 2. Installation

### Prerequisites

- macOS (required for Core MIDI)
- Rust 1.70 or later

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build SEQ

```bash
git clone https://github.com/yourusername/seq.git
cd seq
cargo build --release
```

The release build is recommended for performance.

### Verify Installation

```bash
# Check MIDI devices
cargo run --release -- --list-midi

# Send a test note
cargo run --release -- --test-note 0
```

---

## 3. Quick Start

### Step 1: Connect MIDI

Connect your synthesizer, drum machine, or virtual instrument via MIDI.

```bash
# List available MIDI destinations
cargo run -- --list-midi
```

Note the index number of your device.

### Step 2: Test Connection

```bash
# Send a test note to device 0
cargo run -- --test-note 0
```

You should hear Middle C (C4) for half a second.

### Step 3: Test Clock

```bash
# Send MIDI clock at 120 BPM
cargo run -- --test-clock 0 120
```

External gear should sync to the clock.

### Step 4: Create Configuration

Create a file called `my_song.yaml`:

```yaml
name: "My First Song"
tempo: 120.0
key: C
scale: Major

tracks:
  - name: "Arpeggio"
    channel: 0
    generator:
      type: arpeggio
      pattern: up
      gate: 0.8
```

### Step 5: Run SEQ

```bash
cargo run -- --config my_song.yaml
```

---

## 4. Core Concepts

### 4.1 Timing

SEQ uses **24 PPQN** (Pulses Per Quarter Note), the standard MIDI clock resolution.

| Musical Value | Ticks |
|---------------|-------|
| Whole note | 96 |
| Half note | 48 |
| Quarter note | 24 |
| Eighth note | 12 |
| Sixteenth note | 6 |
| Triplet eighth | 8 |

**Bar calculation (4/4 time):**
- 1 bar = 4 beats = 96 ticks

**Tempo relationship:**
- At 120 BPM: 1 tick ≈ 20.83ms
- At 60 BPM: 1 tick ≈ 41.67ms

### 4.2 Scales and Keys

All generators are scale-aware. Notes are automatically quantized to the selected scale.

**Available Scales:**

| Scale | Intervals |
|-------|-----------|
| Major | 1 2 3 4 5 6 7 |
| Minor (Natural) | 1 2 b3 4 5 b6 b7 |
| Harmonic Minor | 1 2 b3 4 5 b6 7 |
| Melodic Minor | 1 2 b3 4 5 6 7 |
| Dorian | 1 2 b3 4 5 6 b7 |
| Phrygian | 1 b2 b3 4 5 b6 b7 |
| Lydian | 1 2 3 #4 5 6 7 |
| Mixolydian | 1 2 3 4 5 6 b7 |
| Locrian | 1 b2 b3 4 b5 b6 b7 |
| Pentatonic Major | 1 2 3 5 6 |
| Pentatonic Minor | 1 b3 4 5 b7 |
| Blues | 1 b3 4 b5 5 b7 |
| Whole Tone | 1 2 3 #4 #5 #6 |
| Chromatic | All 12 notes |

### 4.3 MIDI Channels

MIDI has 16 channels (0-15 in code, 1-16 on hardware).

**Common convention:**
- Channels 0-8: Melodic instruments
- Channel 9: Drums (GM standard)
- Channels 10-15: Additional instruments

### 4.4 Velocity

Velocity (0-127) controls note loudness/intensity:

| Velocity | Dynamic |
|----------|---------|
| 0 | Note Off |
| 1-31 | pp (pianissimo) |
| 32-63 | p-mp (piano) |
| 64-95 | mf-f (forte) |
| 96-127 | ff (fortissimo) |

---

## 5. Generators

Generators are the heart of SEQ. Each creates a different type of musical content.

### 5.1 Drone Generator

Creates sustained, slowly-evolving ambient textures.

**Use for:** Pads, ambient beds, atmospheric textures

**Parameters:**

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| voices | 1-8 | 3 | Number of simultaneous notes |
| change_rate | 1-9999 | 96 | Ticks between note changes |
| velocity | 1-127 | 80 | Note velocity |
| octave_spread | 1-4 | 2 | Range of octaves |
| base_octave | 0-8 | 3 | Starting octave (C3 = middle) |

**Configuration:**

```yaml
generator:
  type: drone
  voices: 4
  change_rate: 192  # Change every 2 bars
  velocity: 70
  octave_spread: 2
  base_octave: 3
```

**Behavior:**
- Holds notes for extended periods
- Uses voice leading for smooth transitions
- Favors consonant intervals (thirds, fifths, octaves)
- Notes always stay within scale

### 5.2 Arpeggiator

Sequences through chord tones in patterns.

**Use for:** Bass lines, leads, rhythmic patterns

**Patterns:**

| Pattern | Description |
|---------|-------------|
| Up | Low to high |
| Down | High to low |
| UpDown | Up then down (no repeat at ends) |
| DownUp | Down then up |
| Random | Random order |
| Order | Order notes were added |

**Parameters:**

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| pattern | see above | Up | Arpeggio direction |
| octave_range | 1-4 | 1 | Octaves to span |
| gate | 0.1-1.0 | 0.8 | Note length (% of step) |
| probability | 0.0-1.0 | 1.0 | Chance note plays |
| rate | ticks | 12 | Ticks per step |

**Configuration:**

```yaml
generator:
  type: arpeggio
  pattern: updown
  octave_range: 2
  gate: 0.5
  probability: 0.9
  rate: 6  # Sixteenth notes
```

**Euclidean Mode:**

Add euclidean rhythm distribution:

```yaml
generator:
  type: arpeggio
  pattern: up
  euclidean:
    steps: 16
    pulses: 5
```

This creates a 5-note pattern distributed across 16 steps using Bjorklund's algorithm.

### 5.3 Chord Generator

Creates harmonic progressions with various voicings.

**Use for:** Pads, comping, harmonic foundation

**Voicings:**

| Voicing | Description |
|---------|-------------|
| Close | Notes within one octave |
| Open | Spread across octaves |
| Drop2 | Second note from top dropped an octave |
| Spread | Wide intervals |

**Progression Modes:**

| Mode | Description |
|------|-------------|
| Functional | Traditional I-IV-V-I progressions |
| RandomInKey | Random chords from scale |
| Custom | User-defined progression |

**Parameters:**

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| voicing | see above | Close | Chord voicing style |
| progression | see above | Functional | Progression algorithm |
| change_rate | ticks | 96 | Ticks per chord |
| velocity | 1-127 | 80 | Chord velocity |
| extensions | list | [] | Add 7ths, 9ths, etc. |

**Configuration:**

```yaml
generator:
  type: chord
  voicing: open
  progression: functional
  change_rate: 192  # Change every 2 bars
  velocity: 75
  extensions:
    - seventh
    - ninth
```

**Custom Progressions:**

```yaml
generator:
  type: chord
  progression: custom
  chords:
    - { root: 0, quality: major }   # I (C)
    - { root: 5, quality: major }   # IV (F)
    - { root: 7, quality: major }   # V (G)
    - { root: 0, quality: major }   # I (C)
```

### 5.4 Melody Generator

Creates melodic lines using Markov-chain-like algorithms.

**Use for:** Lead lines, countermelodies, solo instruments

**Parameters:**

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| note_min | 0-127 | 48 | Lowest note (C3) |
| note_max | 0-127 | 84 | Highest note (C6) |
| phrase_length | 4-32 | 8 | Notes per phrase |
| rest_probability | 0.0-1.0 | 0.1 | Chance of rest |
| step_preference | 0.0-1.0 | 0.7 | Prefer steps vs leaps |

**Configuration:**

```yaml
generator:
  type: melody
  note_min: 60   # C4
  note_max: 84   # C6
  phrase_length: 8
  rest_probability: 0.15
  step_preference: 0.8
  rhythm:
    - 12  # Eighth notes
    - 6   # Sixteenths
    - 24  # Quarters
```

**Motif Transforms:**

The melody generator can develop motifs:

| Transform | Description |
|-----------|-------------|
| Original | Play as captured |
| Transpose | Shift up/down |
| Invert | Flip intervals |
| Retrograde | Play backwards |

### 5.5 Drum Generator

Creates rhythmic patterns for drum machines.

**Use for:** Beats, percussion, rhythmic foundation

**Styles:**

| Style | Description |
|-------|-------------|
| FourOnFloor | Kick on every beat (house/techno) |
| Breakbeat | Syncopated patterns |
| Sparse | Minimal hits |
| Busy | Dense patterns |
| Euclidean | Mathematically distributed |
| Random | Probability-based |

**Parameters:**

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| style | see above | FourOnFloor | Pattern style |
| humanize | 0.0-1.0 | 0.0 | Timing/velocity variation |
| fill_probability | 0.0-1.0 | 0.1 | Chance of fill |
| swing | 0.0-1.0 | 0.0 | Swing amount |

**Configuration:**

```yaml
generator:
  type: drums
  style: four_on_floor
  humanize: 0.15
  fill_probability: 0.05
  swing: 0.2
  voices:
    kick:
      note: 36
      probability: 1.0
    snare:
      note: 38
      probability: 0.8
    hihat:
      note: 42
      probability: 0.9
```

**GM Drum Notes:**

| Note | Drum |
|------|------|
| 36 | Kick |
| 38 | Snare |
| 37 | Sidestick |
| 42 | Closed Hi-Hat |
| 44 | Pedal Hi-Hat |
| 46 | Open Hi-Hat |
| 41 | Low Tom |
| 43 | Mid Tom |
| 45 | High Tom |
| 49 | Crash |
| 51 | Ride |
| 39 | Clap |

---

## 6. Tracks and Clips

### 6.1 Tracks

A track represents one MIDI channel with its own settings.

**Track Properties:**

| Property | Description |
|----------|-------------|
| name | Display name |
| channel | MIDI channel (0-15) |
| generator | Attached generator |
| transpose | Semitone offset |
| velocity_scale | Velocity multiplier |
| swing | Track-specific swing |
| mute | Silence output |
| solo | Only play this track |

**Configuration:**

```yaml
tracks:
  - name: "Bass"
    channel: 0
    transpose: -12  # One octave down
    velocity_scale: 0.9
    generator:
      type: arpeggio
      pattern: up

  - name: "Lead"
    channel: 1
    transpose: 0
    generator:
      type: melody
```

### 6.2 Clips

Clips are containers for musical content—either static sequences or generator output.

**Clip Types:**

| Type | Description |
|------|-------------|
| Sequenced | Pre-defined note pattern |
| Generated | Real-time generator output |
| Hybrid | Static base + generated variations |

**Clip Modes:**

| Mode | Behavior |
|------|----------|
| OneShot | Play once, then stop |
| Loop | Repeat indefinitely |
| LoopCount(n) | Repeat n times |
| PingPong | Forward, backward, repeat |

**Configuration:**

```yaml
clips:
  - name: "Intro Arp"
    type: generated
    generator: arpeggio
    mode: loop
    length_bars: 4

  - name: "Verse Bass"
    type: sequenced
    mode: loop
    length_bars: 2
    notes:
      - { tick: 0, note: 36, velocity: 100, duration: 12 }
      - { tick: 24, note: 36, velocity: 90, duration: 12 }
      - { tick: 48, note: 38, velocity: 100, duration: 12 }
      - { tick: 72, note: 36, velocity: 85, duration: 12 }
```

---

## 7. Parts and Scenes

Parts and scenes help organize your performance into sections.

### 7.1 Parts

A part defines the state of all tracks—which clips or generators are active.

**Part Configuration:**

```yaml
parts:
  - name: "Intro"
    tracks:
      0: { clip: "Intro Arp" }
      1: { mute: true }
      2: { mute: true }
    transition: next_bar

  - name: "Verse"
    tracks:
      0: { clip: "Verse Bass" }
      1: { generator: chord }
      2: { generator: drums }
    transition: next_bar

  - name: "Chorus"
    tracks:
      0: { clip: "Chorus Bass" }
      1: { clip: "Chorus Chords" }
      2: { generator: drums, style: busy }
    transition: next_bar
```

**Transition Types:**

| Transition | Description |
|------------|-------------|
| immediate | Switch instantly |
| next_beat | Wait for next beat |
| next_bar | Wait for next bar |
| beats(n) | Wait n beats |
| bars(n) | Wait n bars |

### 7.2 Scenes

Scenes are like horizontal slices—each track has a slot in a scene matrix.

```
         Track 0    Track 1    Track 2
Scene 1: [Clip A]   [Clip D]   [Gen X]
Scene 2: [Clip B]   [Clip E]   [Gen Y]
Scene 3: [Clip C]   [Clip F]   [Gen Z]
```

Launching a scene triggers all its slots simultaneously.

**Scene Configuration:**

```yaml
scenes:
  - name: "Scene 1"
    slots:
      0: { clip: "Arp 1" }
      1: { clip: "Pad 1" }
      2: { generator: drums }
    follow_action: next
    follow_after: 8  # bars

  - name: "Scene 2"
    slots:
      0: { clip: "Arp 2" }
      1: { clip: "Pad 2" }
      2: { generator: drums }
```

**Follow Actions:**

| Action | Description |
|--------|-------------|
| None | Stay on scene |
| Stop | Stop playback |
| Again | Replay same scene |
| Next | Go to next scene |
| Previous | Go to previous scene |
| First | Go to first scene |
| Last | Go to last scene |
| Random | Random scene |
| Specific(name) | Named scene |

---

## 8. Song Mode

Song mode arranges parts into a linear structure for full compositions.

### 8.1 Song Structure

```yaml
song:
  name: "My Song"
  tempo: 120.0
  default_time_sig: [4, 4]

  sections:
    - part: "Intro"
      length_bars: 8

    - part: "Verse"
      length_bars: 16

    - part: "Chorus"
      length_bars: 8

    - part: "Verse"
      length_bars: 16

    - part: "Chorus"
      length_bars: 16

    - part: "Outro"
      length_bars: 8
```

### 8.2 Section Properties

| Property | Description |
|----------|-------------|
| part | Part name to use |
| length_bars | Duration in bars |
| tempo | Section-specific tempo |
| time_sig | Time signature change |
| scene | Scene to trigger |
| loop_point | Mark as loop start |

### 8.3 Loop Regions

```yaml
song:
  sections:
    - part: "Intro"
      length_bars: 8

    - part: "Verse"
      length_bars: 16
      loop_point: true  # Loop starts here

    - part: "Chorus"
      length_bars: 8    # Loop ends after this
```

### 8.4 Song Controls

| Action | Description |
|--------|-------------|
| Play | Start from current position |
| Stop | Stop and reset to start |
| Pause | Pause at current position |
| Goto(section) | Jump to section |
| SetLoop(start, end) | Define loop region |
| ClearLoop | Remove loop |

---

## 9. Recording

SEQ can record MIDI input and generator output.

### 9.1 Recording Modes

| Mode | Description |
|------|-------------|
| Replace | Clear existing, record new |
| Overdub | Add to existing notes |
| Punch | Record only in defined region |

### 9.2 Recording Input

To record MIDI input from a controller:

1. Arm the track for recording
2. Start playback
3. Play notes on controller
4. Stop to finish recording

### 9.3 Clip Freeze

"Freezing" captures generator output as a static clip:

1. Select generator track
2. Set freeze length (bars)
3. Initiate freeze
4. Generator runs, output is captured
5. Result is a new sequenced clip

This is useful for:
- Capturing a good generative passage
- Reducing CPU usage
- Creating variations to edit

### 9.4 Quantization

Recorded notes can be quantized:

| Setting | Description |
|---------|-------------|
| grid | Quantization grid (ticks) |
| strength | 0.0 = no quantize, 1.0 = full |
| start | Quantize note starts |
| end | Quantize note ends |

```yaml
recording:
  quantize:
    grid: 12  # Eighth notes
    strength: 0.75
    start: true
    end: false
```

---

## 10. MIDI Export

Export your music as Standard MIDI Files.

### 10.1 Export Formats

| Format | Description |
|--------|-------------|
| Type 0 | Single track, all channels |
| Type 1 | Multiple tracks, one per channel |

Type 1 is recommended for DAW import.

### 10.2 Export Options

```yaml
export:
  format: type1
  filename: "my_song.mid"
  include_tempo: true
  include_time_sig: true
  include_program_changes: true
```

### 10.3 What Gets Exported

- All recorded notes
- Frozen clips
- Tempo information
- Time signature changes
- Program changes (instrument selection)

### 10.4 Export Process

1. Record or freeze clips as needed
2. Configure export settings
3. Execute export command
4. Open resulting .mid file in any DAW

---

## 11. Configuration

### 11.1 File Format

SEQ uses YAML for configuration. YAML is human-readable and supports comments.

### 11.2 Complete Configuration Example

```yaml
# Song metadata
name: "Ambient Set 1"
tempo: 72.0
time_signature: [4, 4]
key: D
scale: minor

# Global settings
settings:
  ppqn: 24
  master_volume: 0.8
  swing: 0.0

# Track definitions
tracks:
  - name: "Pad"
    channel: 0
    generator:
      type: drone
      voices: 4
      change_rate: 192
      velocity: 70
      octave_spread: 2
      base_octave: 3

  - name: "Arp"
    channel: 1
    transpose: 0
    generator:
      type: arpeggio
      pattern: up
      octave_range: 2
      gate: 0.7
      rate: 12

  - name: "Bass"
    channel: 2
    transpose: -12
    generator:
      type: arpeggio
      pattern: up
      octave_range: 1
      gate: 0.9
      rate: 24

  - name: "Drums"
    channel: 9
    generator:
      type: drums
      style: sparse
      humanize: 0.1

# Part definitions
parts:
  - name: "Intro"
    tracks:
      0: { state: playing }
      1: { state: muted }
      2: { state: muted }
      3: { state: muted }
    transition: next_bar

  - name: "Build"
    tracks:
      0: { state: playing }
      1: { state: playing }
      2: { state: muted }
      3: { state: muted }
    transition: next_bar

  - name: "Full"
    tracks:
      0: { state: playing }
      1: { state: playing }
      2: { state: playing }
      3: { state: playing }
    transition: next_bar

  - name: "Breakdown"
    tracks:
      0: { state: playing }
      1: { state: muted }
      2: { state: muted }
      3: { state: muted }
    transition: bars(2)

# Controller mappings
controllers:
  - device: "Launchpad"
    channel: 0
    mappings:
      - { type: note, note: 36, action: trigger_part, part: "Intro" }
      - { type: note, note: 37, action: trigger_part, part: "Build" }
      - { type: note, note: 38, action: trigger_part, part: "Full" }
      - { type: note, note: 39, action: trigger_part, part: "Breakdown" }
      - { type: cc, cc: 1, action: set_tempo, min: 60, max: 120 }
```

### 11.3 Hot Reload

SEQ watches configuration files and reloads changes automatically:

1. Edit the YAML file
2. Save the file
3. Changes apply within 500ms
4. Playback continues uninterrupted

**What can be hot-reloaded:**
- Tempo
- Generator parameters
- Part definitions
- Controller mappings

**What requires restart:**
- Track count changes
- MIDI device changes

---

## 12. MIDI Controllers

### 12.1 Supported Controller Types

- Note triggers (pads, keys)
- CC (knobs, faders, encoders)
- Program Change
- Pitch Bend

### 12.2 Controller Mapping

```yaml
controllers:
  - device: "My Controller"
    channel: 0  # Optional: filter by channel
    mappings:
      # Note trigger
      - type: note
        note: 36
        action: play

      # CC knob
      - type: cc
        cc: 1
        action: set_tempo
        min: 60
        max: 180

      # Encoder (relative mode)
      - type: cc
        cc: 16
        action: adjust_tempo
        encoder_mode: relative64
        sensitivity: 0.5
```

### 12.3 Available Actions

| Action | Description |
|--------|-------------|
| play | Start playback |
| stop | Stop playback |
| pause | Pause playback |
| set_tempo | Set absolute tempo |
| adjust_tempo | Nudge tempo |
| trigger_part | Trigger named part |
| trigger_scene | Trigger scene by index |
| mute_track | Toggle track mute |
| solo_track | Toggle track solo |
| set_parameter | Set generator parameter |

### 12.4 Encoder Modes

For endless encoders:

| Mode | Description |
|------|-------------|
| absolute | 0-127 direct value |
| relative64 | 64 = no change, <64 = down, >64 = up |
| relative_binary | 0-63 = down, 64-127 = up |
| relative_signed | 1-64 = up, 65-127 = down |

### 12.5 MIDI Learn

To learn a mapping:

1. Enter MIDI learn mode
2. Move the control you want to map
3. Select the action to assign
4. Exit MIDI learn mode

---

## 13. Keyboard Shortcuts

### 13.1 Transport

| Key | Action |
|-----|--------|
| Space | Play / Pause |
| Escape | Stop (reset to start) |
| Enter | Continue from pause |

### 13.2 Tempo

| Key | Action |
|-----|--------|
| Up Arrow | Increase tempo 1 BPM |
| Down Arrow | Decrease tempo 1 BPM |
| Shift + Up | Increase tempo 10 BPM |
| Shift + Down | Decrease tempo 10 BPM |
| T | Tap tempo |

### 13.3 Parts

| Key | Action |
|-----|--------|
| 1-9 | Trigger part 1-9 |
| 0 | Trigger part 10 |

### 13.4 Tracks

| Key | Action |
|-----|--------|
| M + 1-9 | Toggle mute on track 1-9 |
| S + 1-9 | Toggle solo on track 1-9 |

### 13.5 Navigation

| Key | Action |
|-----|--------|
| [ | Previous scene |
| ] | Next scene |
| < | Previous part |
| > | Next part |
| Home | Go to start |
| End | Go to end |

### 13.6 General

| Key | Action |
|-----|--------|
| Q | Quit |
| H | Toggle help |
| R | Toggle record |
| L | Toggle MIDI learn |

---

## 14. Live Performance

### 14.1 Preparation

**Before the gig:**

1. Test all MIDI connections
2. Verify controller mappings
3. Create and test all parts
4. Set up fallback configurations
5. Practice transitions

**Checklist:**
- [ ] MIDI cables tested
- [ ] Controller batteries charged
- [ ] Configuration files backed up
- [ ] Tempo verified with external gear
- [ ] All parts trigger correctly

### 14.2 Performance Workflow

**Typical flow:**

1. Start with "Intro" part
2. Build energy by adding tracks
3. Use scenes for instant changes
4. Use parts for structured transitions
5. Drop out elements for breakdowns
6. Build back up for climax
7. Wind down with outro

### 14.3 Transition Strategies

**Smooth transitions:**
- Use `next_bar` quantization
- Cross-fade between drone voices
- Keep drums consistent through changes

**Energy drops:**
- Mute all but one track
- Change to sparse drum style
- Reduce tempo slightly

**Energy builds:**
- Add tracks one by one
- Increase tempo gradually
- Add drum fills

### 14.4 Recovery

**If something goes wrong:**

1. Hit Stop (Escape)
2. Trigger a known-good part
3. Continue performance

**Panic button:**
- Map a controller button to "stop all"
- This sends All Notes Off to all channels

### 14.5 Recording Your Performance

To capture your live performance:

1. Enable recording before starting
2. Perform as normal
3. Export when done
4. Review and edit in DAW

---

## 15. Tips and Best Practices

### 15.1 Sound Design Tips

**For ambient/drone:**
- Use slow change rates (192+ ticks)
- Limit drone voices to 3-4
- Layer with long reverb
- Use open/spread voicings

**For rhythmic music:**
- Keep bass and drums tight (high velocity)
- Use swing sparingly (0.1-0.2)
- Let arpeggiator drive the energy
- Use probability for variation

### 15.2 Performance Tips

**Controller layout:**
- Group related functions together
- Put frequently-used controls within easy reach
- Use colors if your controller supports them

**Part organization:**
- Create more parts than you think you need
- Name parts clearly (Intro, Build, Full, etc.)
- Test transitions between all part combinations

### 15.3 Configuration Tips

**Keep it simple:**
- Start with 2-3 tracks
- Add complexity gradually
- Save working configurations often

**Organization:**
- Use comments in YAML files
- Create separate configs for different sets
- Version control your configurations

### 15.4 Common Pitfalls

**Avoid:**
- Too many simultaneous voices (CPU)
- Conflicting notes on same channel
- Tempo too fast for complex generators
- Over-complicated part structures

**Solutions:**
- Freeze CPU-heavy generators
- Use different channels for each track
- Test at target tempo
- Simplify part structure

---

## 16. Reference

### 16.1 MIDI Note Numbers

| Octave | C | C# | D | D# | E | F | F# | G | G# | A | A# | B |
|--------|---|----|----|----|----|----|----|----|----|----|----|---|
| -1 | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
| 0 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 |
| 1 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 35 |
| 2 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 43 | 44 | 45 | 46 | 47 |
| 3 | 48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 | 58 | 59 |
| 4 | 60 | 61 | 62 | 63 | 64 | 65 | 66 | 67 | 68 | 69 | 70 | 71 |
| 5 | 72 | 73 | 74 | 75 | 76 | 77 | 78 | 79 | 80 | 81 | 82 | 83 |
| 6 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 93 | 94 | 95 |
| 7 | 96 | 97 | 98 | 99 | 100 | 101 | 102 | 103 | 104 | 105 | 106 | 107 |
| 8 | 108 | 109 | 110 | 111 | 112 | 113 | 114 | 115 | 116 | 117 | 118 | 119 |

**Middle C = C4 = MIDI note 60**

### 16.2 Common CC Numbers

| CC | Name | Typical Use |
|----|------|-------------|
| 1 | Mod Wheel | Modulation |
| 7 | Volume | Channel volume |
| 10 | Pan | Stereo position |
| 11 | Expression | Dynamic control |
| 64 | Sustain | Sustain pedal |
| 74 | Filter Cutoff | Brightness |
| 71 | Resonance | Filter resonance |
| 91 | Reverb | Reverb send |
| 93 | Chorus | Chorus send |

### 16.3 Timing Reference

| BPM | Beat (ms) | Tick (ms) | Bar (ms) |
|-----|-----------|-----------|----------|
| 60 | 1000 | 41.67 | 4000 |
| 80 | 750 | 31.25 | 3000 |
| 100 | 600 | 25.00 | 2400 |
| 120 | 500 | 20.83 | 2000 |
| 140 | 428 | 17.86 | 1714 |
| 160 | 375 | 15.63 | 1500 |
| 180 | 333 | 13.89 | 1333 |

### 16.4 Scale Intervals (Semitones)

| Scale | Intervals |
|-------|-----------|
| Major | 0, 2, 4, 5, 7, 9, 11 |
| Minor | 0, 2, 3, 5, 7, 8, 10 |
| Dorian | 0, 2, 3, 5, 7, 9, 10 |
| Phrygian | 0, 1, 3, 5, 7, 8, 10 |
| Lydian | 0, 2, 4, 6, 7, 9, 11 |
| Mixolydian | 0, 2, 4, 5, 7, 9, 10 |
| Pentatonic Major | 0, 2, 4, 7, 9 |
| Pentatonic Minor | 0, 3, 5, 7, 10 |
| Blues | 0, 3, 5, 6, 7, 10 |

### 16.5 File Locations

| File | Purpose |
|------|---------|
| `~/.seq/config.yaml` | Default configuration |
| `~/.seq/controllers/` | Controller mappings |
| `~/.seq/songs/` | Song configurations |
| `~/.seq/clips/` | Saved clips |
| `~/.seq/exports/` | MIDI file exports |

---

## Appendix: Glossary

| Term | Definition |
|------|------------|
| BPM | Beats Per Minute - tempo measurement |
| CC | Control Change - MIDI continuous controller message |
| Clip | Container for musical content |
| DAW | Digital Audio Workstation |
| Generator | Algorithm that creates musical content |
| GM | General MIDI - standard sound/drum mapping |
| Part | Saved state of all tracks |
| PPQN | Pulses Per Quarter Note - timing resolution |
| Scene | Horizontal slice of clip matrix |
| Tick | Smallest timing unit (1/24 of a beat) |
| VLQ | Variable Length Quantity - MIDI file encoding |
| Voicing | How chord notes are arranged |

---

*SEQ User Guide - v0.1.0*
