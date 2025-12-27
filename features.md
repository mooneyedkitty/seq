# SEQ - Algorithmic MIDI Sequencer for Live Performance

## Overview

SEQ is a performance-focused MIDI sequencer designed for live electronic music. Its core philosophy is **minimal visual UI, maximum musical control**—operated via external MIDI controllers and keyboard shortcuts, configured through human-readable text files.

Primary use cases: ambient, synthwave, synthpop, and generative electronic music.

---

## Core Architecture

### Audio/MIDI Output
- **Development mode**: Internal sound engine (FluidSynth or similar) for testing without hardware
- **Performance mode**: Standard USB MIDI output to external synthesizers and drum machines
- Multi-channel MIDI routing (16 channels per port, multiple ports supported)
- MIDI clock master/slave synchronization

### Configuration System
- **YAML or TOML-based configuration files** for:
  - Song structures and arrangements
  - Part definitions (patterns, sequences, clips)
  - Scale/key definitions
  - Controller mappings
  - Algorithmic rules and constraints
- Hot-reload configuration changes without stopping playback
- Project/song/part hierarchy for organization

### Control Interface
- **Minimal or headless UI**—terminal-based status display only
- External MIDI controller mapping (pads, knobs, buttons)
- Laptop keyboard shortcuts for all functions
- Optional OSC support for tablet/phone control

---

## Algorithmic Music Generation

*The heart of SEQ*

### Scale & Key System
- Predefined scales: major, minor (natural/harmonic/melodic), modes, pentatonic, blues, chromatic
- Custom scale definitions
- Real-time key/scale changes with intelligent note mapping
- Parallel key relationships (relative major/minor switching)

### Generative Engines

#### Drone Generator
- Sustained root/fifth/octave combinations
- Slow random note selection within scale
- Configurable note density and movement speed
- Voice leading rules for smooth transitions
- Controllable probability distributions

#### Chord Generator
- Chord progression algorithms (functional harmony, neo-Riemannian, random-in-key)
- Voicing options: close, open, drop-2, spread
- Inversion selection (random, ascending, voice-led)
- Rhythm patterns for chord changes
- Tension/release controls (add7, add9, sus variations)

#### Arpeggiator
- Classic patterns: up, down, up-down, random, order-played
- Octave range and direction
- Note length and gate percentage
- Pattern variations: skip notes, repeat notes, accent patterns
- Euclidean rhythm generation
- Probability-based note triggering

#### Melodic Generator
- Markov chain-based melody generation
- Configurable interval probabilities
- Rhythmic pattern templates
- Call-and-response patterns
- Motif development (repeat, transpose, invert, retrograde)

#### Drum/Percussion Generator
- Euclidean rhythm algorithms
- Style templates (four-on-floor, breakbeat, ambient sparse)
- Probability-based hits and ghost notes
- Humanization (timing, velocity variation)
- Fill generation on demand

### Constraint System
- Note range limits (per voice/channel)
- Velocity ranges and curves
- Rhythmic quantization options
- Density controls (notes per bar)
- "Chaos" parameter: 0% = deterministic, 100% = full random within constraints

---

## Loop & Pattern System

### Clip Types
- **Sequenced clips**: Pre-composed MIDI patterns from config files
- **Generated clips**: Real-time algorithmic content
- **Hybrid clips**: Sequenced backbone with generative variations
- **Recording clips**: Live-captured MIDI input

### Loop Triggering
- Instant trigger (start immediately)
- Quantized trigger (next beat/bar/phrase)
- Queue system for upcoming changes
- One-shot vs. looping modes
- Follow actions (chain clips, random next, etc.)

### Pattern Manipulation
- Transpose patterns in real-time
- Time-stretch/compress (half-time, double-time)
- Reverse playback
- Pattern rotation (shift start point)
- Mute/unmute individual pattern layers

---

## Song & Arrangement

### Structure
- **Parts**: Collections of clips that play together (verse, chorus, bridge, etc.)
- **Scenes**: Snapshots of which clips are active across all tracks
- **Songs**: Ordered or free-form collections of parts/scenes

### Live Arrangement
- Switch between parts/scenes via MIDI or keyboard
- Queue next part while current plays
- Transition modes: cut, fade, crossfade
- Arrangement timeline (optional—can ignore for fully improvised sets)

### Macros
- Single trigger activates multiple changes:
  - Switch parts on multiple tracks
  - Change tempo
  - Shift key/scale
  - Adjust algorithmic parameters

---

## Recording & Capture

### Live Recording
- Arm tracks for recording
- Punch-in/punch-out
- Loop recording with overdub or replace modes
- Quantize input or record freely

### Clip Capture
- "Freeze" generated content into static clip
- Save recordings to config file format
- Export as standard MIDI file

---

## Real-Time Control

### MIDI Learn
- Map any external MIDI CC, note, or program change to any parameter
- Multiple mapping layers (performance, editing, etc.)
- Relative and absolute encoder support

### Key Parameters (exposed for control)
- Master tempo (BPM)
- Swing amount
- Global transpose
- Key/scale selection
- Algorithmic "chaos" level per generator
- Part/scene selection
- Individual clip triggers
- Track mutes/solos
- Generator density/rate controls

### Performance Modes
- **Song mode**: Follow arrangement
- **Session mode**: Free-form clip triggering
- **Generative mode**: Algorithm-driven with minimal intervention

---

## Tempo & Timing

- Tap tempo
- Tempo ramp (gradual BPM changes)
- Time signature support (4/4, 3/4, 6/8, odd meters)
- Swing/shuffle (adjustable per track or global)
- MIDI clock output to sync external gear
- MIDI clock input to sync to external master

---

## Technical Features

### Performance
- Low-latency MIDI output
- Efficient enough for Raspberry Pi deployment (future)
- Stable timing under load

### File Formats
- Native YAML/TOML configuration
- Import standard MIDI files as clips
- Export project as MIDI
- Preset library for generators

### Development
- Python implementation (performance-critical paths optimized)
- Modular architecture for adding new generators
- Plugin system for custom algorithms (future)
- Comprehensive logging for debugging

---

## Example Configuration Structure

```yaml
# song.yaml
song:
  name: "Ambient Set 1"
  tempo: 72
  key: "D"
  scale: "minor"

tracks:
  - name: "Pad"
    channel: 1
    generator: drone
    config:
      density: 0.3
      voices: 3

  - name: "Arp"
    channel: 2
    generator: arpeggio
    config:
      pattern: "up-down"
      octaves: 2
      rate: "1/16"

  - name: "Bass"
    channel: 3
    clips:
      - file: "clips/bass_pattern_1.yaml"
      - file: "clips/bass_pattern_2.yaml"

parts:
  intro:
    tracks:
      Pad: active
      Arp: muted
      Bass: clip_1

  main:
    tracks:
      Pad: active
      Arp: active
      Bass: clip_2
```

---

## Control Mapping Example

```yaml
# controls.yaml
midi:
  device: "Launchpad Mini"

mappings:
  # Pad triggers
  - note: 36
    action: trigger_part
    target: intro

  - note: 37
    action: trigger_part
    target: main

  # Knobs
  - cc: 1
    action: set_param
    target: Arp.density
    range: [0.1, 1.0]

  - cc: 2
    action: set_param
    target: global.chaos
    range: [0.0, 1.0]

keyboard:
  space: toggle_play
  q: trigger_part:intro
  w: trigger_part:main
  up: tempo_nudge:+1
  down: tempo_nudge:-1
```

---

## Future Considerations

- Visual waveform/pattern display (optional, for studio use)
- Ableton Link support for multi-device sync
- Audio input analysis (beat detection, pitch tracking)
- Machine learning-based generation models
- Hardware build (dedicated controller with Raspberry Pi)
- Plugin hosting (VST/AU) for internal sounds
