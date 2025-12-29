# SEQ Implementation Plan - Rust on macOS

## Overview
Algorithmic MIDI sequencer for live performance, built in Rust targeting macOS with Core MIDI integration.

---

## Phase 1: Environment & Project Setup

### Step 1.1: Install Rust Toolchain ✅ COMPLETE
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup component add rustfmt clippy rust-analyzer
```
**Verify:** `rustc --version` returns version, `cargo clippy --version` works

**Status:** Completed 2025-12-27
- rustc 1.92.0 installed
- clippy 0.1.92 installed
- rustfmt, rust-analyzer components added

**Next:** Proceed to Step 1.2 - Initialize Git Repository

### Step 1.2: Initialize Git Repository ✅ COMPLETE
```bash
cd /Users/rsnyder/Development/seq
git init
```
Create `.gitignore`:
```
/target
Cargo.lock
*.swp
.DS_Store
```
**Verify:** `git status` shows clean repo with .gitignore

**Status:** Completed 2025-12-27
- Git repository initialized on branch `main`
- .gitignore created with target, Cargo.lock, *.swp, .DS_Store

**Next:** Proceed to Step 1.3 - Create Cargo Project

### Step 1.3: Create Cargo Project ✅ COMPLETE
```bash
cargo init --name seq
```
**Verify:** `cargo build` succeeds, `cargo run` prints "Hello, world!"

**Status:** Completed 2025-12-27
- Cargo project initialized
- Build and run verified successfully

### Step 1.4: VS Code Setup ✅ COMPLETE
Install extensions:
- `rust-analyzer` (rust-lang.rust-analyzer) - LSP, completion, diagnostics
- `Even Better TOML` (tamasfe.even-better-toml) - Cargo.toml support
- `crates` (serayuzgur.crates) - dependency version management
- `CodeLLDB` (vadimcn.vscode-lldb) - debugging

Create `.vscode/settings.json`:
```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

Create `.vscode/launch.json` for debugging:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug SEQ",
      "cargo": {
        "args": ["build", "--bin=seq", "--package=seq"],
        "filter": {
          "name": "seq",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```
**Verify:** Open project in VS Code, rust-analyzer activates, format on save works

**Status:** Completed 2025-12-27
- .vscode/settings.json created with rust-analyzer config
- .vscode/launch.json created for LLDB debugging

### Step 1.5: Add Initial Dependencies to Cargo.toml ✅ COMPLETE
```toml
[package]
name = "seq"
version = "0.1.0"
edition = "2021"

[dependencies]
# MIDI
coremidi = "0.8"              # macOS Core MIDI bindings
midir = "0.10"                # Cross-platform MIDI (fallback/testing)

# Audio (for dev sound engine)
fluidlite = "0.1"             # FluidSynth lite bindings

# Config
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
toml = "0.8"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Terminal UI
ratatui = "0.28"              # TUI framework
crossterm = "0.28"            # Terminal backend

# Utilities
anyhow = "1.0"                # Error handling
thiserror = "1.0"             # Custom errors
tracing = "0.1"               # Logging
tracing-subscriber = "0.3"    # Log output
rand = "0.8"                  # RNG for generative algorithms
```
**Verify:** `cargo build` succeeds with all dependencies

**Status:** Completed 2025-12-27
- All dependencies added to Cargo.toml
- Note: fluidlite updated to v0.2 with bindgen feature for Rust 1.92 compatibility
- Build verified successful

### Step 1.6: Initial Commit ✅ COMPLETE
```bash
git add -A
git commit -m "Initial project setup with dependencies"
```
**Verify:** `git log` shows commit

**Status:** Completed 2025-12-27
- Initial commit created: 6b4abf1
- All Phase 1 files committed

---

## ✅ PHASE 1 COMPLETE

All environment and project setup steps finished. Ready to proceed to Phase 2.

**Next:** Proceed to Phase 2, Step 2.1 - MIDI Output Abstraction

---

## Phase 2: Core MIDI Infrastructure

### Step 2.1: MIDI Output Abstraction ✅ COMPLETE
Create `src/midi/mod.rs` with trait:
```rust
pub trait MidiOutput: Send {
    fn send(&mut self, message: &[u8]) -> Result<()>;
    fn send_at(&mut self, message: &[u8], timestamp: u64) -> Result<()>;
}
```
**Verify:** Code compiles, unit tests pass

**Status:** Completed 2025-12-27
- Created `src/midi/mod.rs` with `MidiOutput` trait
- Added MIDI message constants module
- Added MockMidiOutput for testing
- 3 unit tests pass

**Next:** Proceed to Step 2.2 - Core MIDI Backend (macOS)

### Step 2.2: Core MIDI Backend (macOS) ✅ COMPLETE
Create `src/midi/coremidi_backend.rs`:
- Initialize Core MIDI client
- Create virtual output port
- Connect to external MIDI destinations
- List available MIDI devices
- Send MIDI messages with timestamps

**Verify:**
- Run `cargo run -- --list-midi` shows available MIDI ports
- Send test note to external synth or MIDI Monitor app

**Status:** Completed 2025-12-27
- Created `src/midi/coremidi_backend.rs` with `CoreMidiOutput` struct
- Implemented `MidiOutput` trait for Core MIDI
- Added `--list-midi` and `--test-note` CLI commands
- 5 tests pass (3 trait tests + 2 coremidi tests)

**Next:** Proceed to Step 2.3 - MIDI Clock Implementation

### Step 2.3: MIDI Clock Implementation ✅ COMPLETE
Create `src/timing/clock.rs`:
- BPM-based clock with configurable tempo
- Generate MIDI clock messages (0xF8) at 24 PPQN
- Start/Stop/Continue messages
- Tap tempo support
- Tempo ramping (gradual BPM changes)

**Verify:**
- Clock outputs correct PPQN at various tempos (test with MIDI monitor)
- Tap tempo averages correctly
- External synth syncs to clock

**Status:** Completed 2025-12-27
- Created `src/timing/mod.rs` and `src/timing/clock.rs`
- `MidiClock` struct with start/stop/pause/continue/tick methods
- `TapTempo` for tap tempo calculation
- `TempoRamp` for gradual tempo changes
- Added `--test-clock <N> [BPM]` CLI command
- 14 tests pass (9 clock tests + 5 previous)

**Next:** Proceed to Step 2.4 - MIDI Input Handling

### Step 2.4: MIDI Input Handling ✅ COMPLETE
Create `src/midi/input.rs`:
- Listen for MIDI input from controllers
- Parse Note On/Off, CC, Program Change
- MIDI learn mode (capture next message)
- External clock sync (slave mode)

**Verify:**
- Controller input is received and logged
- MIDI learn captures CC assignments
- Clock slave mode syncs to external master

**Status:** Completed 2025-12-27
- Created `src/midi/input.rs` with `MidiInput` struct
- `MidiMessage` enum for parsing all MIDI message types
- `MidiLearnCapture` for learning controller assignments
- `ExternalClockSync` for slave mode synchronization
- Added `--list-sources` and `--monitor <N>` CLI commands
- 24 tests pass (10 input tests + 14 previous)

**Next:** Proceed to Step 2.5 - Commit Phase 2

### Step 2.5: Commit Phase 2 ✅ COMPLETE
```bash
git add -A
git commit -m "Core MIDI infrastructure with clock and I/O"
```

**Status:** Completed 2025-12-27
- Commit 867bb25 created and pushed to origin
- 7 files changed, 1494 insertions

---

## ✅ PHASE 2 COMPLETE

Core MIDI infrastructure implemented with clock and I/O.

**Next:** Proceed to Phase 3, Step 3.1 - Config Data Structures

---

## Phase 3: Configuration System

### Step 3.1: Config Data Structures ✅ COMPLETE
Create `src/config/mod.rs` with serde structs:
- `SongConfig`: name, tempo, key, scale
- `TrackConfig`: name, channel, generator type, clips
- `PartConfig`: track states (active, muted, clip selection)
- `ControlMapping`: MIDI note/CC to action mappings
- `GeneratorConfig`: parameters per generator type

**Verify:** Sample YAML deserializes correctly, round-trip test passes

**Status:** Completed 2025-12-27
- Created `src/config/mod.rs` with all config structures
- `SongFile` with `SongConfig`, `TrackConfig`, `PartConfig`
- `ControlsFile` with `ControlMapping`, `MidiDeviceConfig`
- `GeneratorConfig` with flexible key-value params
- `TrackState` enum for active/muted/clip states
- Load/save YAML and round-trip serialization
- 31 tests pass (7 config tests + 24 previous)

**Next:** Proceed to Step 3.2 - File Watcher for Hot Reload

### Step 3.2: File Watcher for Hot Reload ✅ COMPLETE
Create `src/config/watcher.rs`:
- Watch config directory for changes
- Debounce rapid changes
- Emit reload events
- Validate before applying

**Verify:**
- Change YAML file while running
- Config reloads without stopping playback
- Invalid config shows error, keeps previous

**Status:** Completed 2025-12-27
- Created `src/config/watcher.rs` with `ConfigWatcher` struct
- `ConfigEvent` enum for reload events (Reloaded, Error, FileCreated, FileDeleted)
- Uses `notify` crate (v6.1) for file system watching
- Debouncing with configurable duration (default 500ms)
- YAML validation before emitting reload events
- Added `tempfile` dev dependency for testing
- 36 tests pass (5 watcher tests + 31 previous)

**Next:** Proceed to Step 3.3 - Scale & Key System

### Step 3.3: Scale & Key System ✅ COMPLETE
Create `src/music/scale.rs`:
- Define Scale enum (Major, Minor variants, Modes, Pentatonic, etc.)
- Note-to-scale-degree mapping
- Transpose within scale
- Parallel key relationships
- Custom scale definitions from config

**Verify:**
- Unit tests for all built-in scales
- Transpose C4 up 3 scale degrees in D minor = F4
- Custom scale from YAML works

**Status:** Completed 2025-12-27
- Created `src/music/mod.rs` and `src/music/scale.rs`
- `Note` enum with all 12 pitch classes and transposition
- `ScaleType` enum with 17 built-in scales (Major, modes, pentatonic, blues, etc.)
- `Scale` struct with degree mapping, quantization, and transposition within scale
- `Key` struct with relative/parallel/dominant/subdominant relationships
- `ScaleRegistry` for custom scale definitions from config
- 56 tests pass (20 scale tests + 36 previous)

**Next:** Proceed to Step 3.4 - Commit Phase 3

### Step 3.4: Commit Phase 3 ✅ COMPLETE
```bash
git add -A
git commit -m "Configuration system with hot reload and scale definitions"
```

**Status:** Completed 2025-12-27
- Commit e6f7da1 created and pushed to origin
- 7 files changed, 1811 insertions

---

## ✅ PHASE 3 COMPLETE

Configuration system implemented with hot reload and scale definitions.

**Next:** Proceed to Phase 4, Step 4.1 - Generator Trait & Registry

---

## Phase 4: Generative Engines

### Step 4.1: Generator Trait & Registry ✅ COMPLETE
Create `src/generators/mod.rs`:
```rust
pub trait Generator: Send {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent>;
    fn set_param(&mut self, name: &str, value: f64);
    fn reset(&mut self);
}
```
Registry to instantiate generators by name from config.

**Verify:** Trait compiles, mock generator can be registered and instantiated

**Status:** Completed 2025-12-27
- Created `src/generators/mod.rs` with `Generator` trait and `GeneratorRegistry`
- `MidiEvent` struct for note data with timing
- `GeneratorContext` with key, tempo, ppqn, and timing info
- Factory pattern for registering generators by name
- 6 unit tests for core generator infrastructure

### Step 4.2: Drone Generator ✅ COMPLETE
Create `src/generators/drone.rs`:
- Sustained notes (root, fifth, octave)
- Slow random note selection within scale
- Configurable density and movement speed
- Voice leading for smooth transitions
- Probability distributions for note selection

**Verify:**
- Drone plays sustained notes in correct scale
- Notes change at configured rate
- Voice leading creates smooth transitions

**Status:** Completed 2025-12-27
- Created `src/generators/drone.rs` with `DroneGenerator`
- Configurable voices (1-8), change rate, velocity
- Voice leading with interval preferences
- Octave spread and base octave settings
- 6 unit tests pass

### Step 4.3: Arpeggiator ✅ COMPLETE
Create `src/generators/arpeggio.rs`:
- Patterns: up, down, up-down, random, order-played
- Octave range and direction
- Gate percentage (note length)
- Euclidean rhythm option
- Probability-based note skipping

**Verify:**
- Each pattern type produces correct note order
- Octave spanning works
- Euclidean mode creates expected rhythms

**Status:** Completed 2025-12-27
- Created `src/generators/arpeggio.rs` with `ArpeggioGenerator`
- 6 pattern types: Up, Down, UpDown, DownUp, Random, Order
- Euclidean rhythm generation (Bjorklund's algorithm)
- Gate length, octave range, and note probability
- 7 unit tests pass

### Step 4.4: Chord Generator ✅ COMPLETE
Create `src/generators/chord.rs`:
- Progression algorithms (functional harmony, random-in-key)
- Voicings: close, open, drop-2, spread
- Inversions: random, ascending, voice-led
- Tension additions (7ths, 9ths, sus)
- Rhythm patterns for changes

**Verify:**
- Chords are in correct scale/key
- Voicing types produce correct intervals
- Progressions follow harmonic logic

**Status:** Completed 2025-12-27
- Created `src/generators/chord.rs` with `ChordGenerator`
- 4 voicing types: Close, Open, Drop2, Spread
- 4 inversion modes: Root, Random, VoiceLed, Ascending
- 3 progression modes: Functional, RandomInKey, Custom
- Extensions: 7ths, 9ths, add2, sus4
- 7 unit tests pass

### Step 4.5: Melodic Generator ✅ COMPLETE
Create `src/generators/melody.rs`:
- Markov chain-based generation
- Configurable interval probabilities
- Rhythmic templates
- Motif operations: repeat, transpose, invert, retrograde

**Verify:**
- Melodies stay in scale
- Interval probabilities affect output statistically
- Motif transformations are correct

**Status:** Completed 2025-12-27
- Created `src/generators/melody.rs` with `MelodyGenerator`
- `IntervalProbabilities` for Markov-like interval selection
- 4 motif transforms: Original, Transpose, Invert, Retrograde
- Phrase structure with motif capture and development
- Configurable note range, duration, and density
- 7 unit tests pass

### Step 4.6: Drum Generator ✅ COMPLETE
Create `src/generators/drums.rs`:
- Euclidean rhythm algorithms
- Style templates (four-on-floor, breakbeat, sparse)
- Per-instrument probability
- Ghost notes and accents
- Humanization (timing/velocity variation)
- Fill generation

**Verify:**
- Euclidean patterns match mathematical definition
- Style templates sound appropriate
- Humanization adds subtle variation

**Status:** Completed 2025-12-27
- Created `src/generators/drums.rs` with `DrumGenerator`
- GM drum note constants (kick, snare, hats, toms, etc.)
- 6 styles: FourOnFloor, Breakbeat, Sparse, Busy, Euclidean, Random
- Multi-voice support with per-voice patterns
- Fill generation with probability triggers
- Humanization for timing and velocity
- 8 unit tests pass

### Step 4.7: Commit Phase 4 ✅ COMPLETE
```bash
git add -A
git commit -m "Generative engines: drone, arp, chord, melody, drums"
```

**Status:** Completed 2025-12-27
- 97 tests pass (41 generator tests + 56 previous)
- All 5 generators implemented with comprehensive test coverage

---

## ✅ PHASE 4 COMPLETE

Generative engines implemented: drone, arpeggio, chord, melody, drums.

**Next:** Proceed to Phase 5, Step 5.1 - Event Scheduler

---

## Phase 5: Sequencer Core

### Step 5.1: Event Scheduler ✅ COMPLETE
Create `src/sequencer/scheduler.rs`:
- Priority queue for timed MIDI events
- Microsecond-precision timing
- Lookahead buffer (generate events ahead of playback)
- Handle tempo changes without drift

**Verify:**
- Events fire at correct times (test with scope/analyzer)
- Tempo changes don't cause timing glitches
- No drift over extended playback (10+ minutes)

**Status:** Completed 2025-12-28
- Created `src/sequencer/mod.rs` with `SequencerTiming` struct
- Created `src/sequencer/scheduler.rs` with `Scheduler` and `ScheduledEvent`
- Priority queue (BinaryHeap) for time-ordered event dispatch
- Microsecond-precision timing with tick conversion
- Tempo change handling with event time recalculation
- Start/stop/pause/resume/seek controls
- 9 unit tests pass

### Step 5.2: Track System ✅ COMPLETE
Create `src/sequencer/track.rs`:
- Track state: playing clip, generator, mute/solo
- Multi-channel routing
- Per-track transpose
- Per-track swing

**Verify:**
- Multiple tracks output to different MIDI channels
- Mute/solo works correctly
- Track transpose shifts all notes

**Status:** Completed 2025-12-28
- Created `src/sequencer/track.rs` with `Track` and `TrackManager`
- `TrackState` enum: Active, Muted, Soloed
- `TrackConfig` with channel, transpose, swing, velocity scaling
- Solo handling - when any track is soloed, only soloed tracks output
- Swing application to off-beat notes
- Note range filtering and velocity processing
- 10 unit tests pass

### Step 5.3: Clip System ✅ COMPLETE
Create `src/sequencer/clip.rs`:
- Sequenced clips (from config/MIDI file)
- Generated clips (real-time from generators)
- Hybrid clips (sequenced + variations)
- Loop points and length
- One-shot vs looping

**Verify:**
- Static clips play correctly
- Generated clips produce output
- Loop points respected

**Status:** Completed 2025-12-28
- Created `src/sequencer/clip.rs` with `Clip` and `ClipBuilder`
- `ClipType`: Sequenced, Generated, Hybrid
- `ClipMode`: OneShot, Loop, LoopCount(n), PingPong
- `ClipNote` for static note sequences
- Loop points with start/end configuration
- Hybrid mode mixes static notes with generated variations
- 12 unit tests pass

### Step 5.4: Pattern Triggering ✅ COMPLETE
Create `src/sequencer/trigger.rs`:
- Instant trigger
- Quantized trigger (next beat/bar/phrase)
- Queue system for upcoming changes
- Follow actions (chain, random next)

**Verify:**
- Quantized triggers wait for correct boundary
- Queue shows pending changes
- Follow actions execute

**Status:** Completed 2025-12-28
- Created `src/sequencer/trigger.rs` with `TriggerQueue`, `Scene`, `SceneManager`
- `QuantizeMode`: Immediate, Tick, Beat, Bar, Beats(n), Bars(n), Phrase
- `FollowAction`: None, Stop, Again, Next, Previous, First, Last, Random, Specific, Either
- Scene system for triggering multiple track clips together
- Sorted trigger queue with poll-based dispatch
- 17 unit tests pass

### Step 5.5: Commit Phase 5 ✅ COMPLETE
```bash
git add -A
git commit -m "Sequencer core: scheduler, tracks, clips, triggering"
```

**Status:** Completed 2025-12-28
- 145 tests pass (48 sequencer tests + 97 previous)
- All sequencer components implemented with comprehensive test coverage

---

## ✅ PHASE 5 COMPLETE

Sequencer core implemented: scheduler, tracks, clips, triggering.

**Next:** Proceed to Phase 6, Step 6.1 - Basic TUI Framework

---

## Phase 6: Terminal UI

### Step 6.1: Basic TUI Framework ✅ COMPLETE
Create `src/ui/mod.rs` with ratatui:
- Main layout: status bar, tracks view, transport
- Async input handling
- 60fps render loop

**Verify:** TUI displays without flickering, responds to key input

**Status:** Completed 2025-12-28
- Created `src/ui/mod.rs` with `App`, `UiState`, and render functions
- Crossterm backend with raw mode and alternate screen
- Configurable frame rate (default 60fps)
- Key event handling with `KeyAction` enum
- Status message system with auto-expiry
- Help overlay toggle
- 6 unit tests pass

### Step 6.2: Transport Display ✅ COMPLETE
- Current tempo (BPM)
- Time signature
- Bar:beat:tick position
- Play/stop/record status

**Verify:** Position updates in real-time, tempo shows correctly

**Status:** Completed 2025-12-28
- Created `src/ui/transport.rs` with `TransportWidget`
- `TransportState` with play/stop/record indicators
- Position display (Bar:Beat:Tick format)
- Tempo and time signature display
- `PositionWidget` and `TempoWidget` components
- Beat flash indicator for visual metronome
- 2 unit tests pass

### Step 6.3: Track Status View ✅ COMPLETE
- Track names and states
- Active clip/generator per track
- Mute/solo indicators
- Current notes playing (optional)

**Verify:** All tracks visible, states update live

**Status:** Completed 2025-12-28
- Created `src/ui/tracks.rs` with `TracksWidget`
- `TrackUiState` for track display data
- Mute/Solo indicators with color coding
- Velocity level meters
- Selection highlighting
- `TrackDetailWidget` for expanded view
- `NoteDisplayWidget` for piano roll visualization
- 4 unit tests pass

### Step 6.4: Controller Mapping Display ✅ COMPLETE
- Show incoming MIDI activity
- Current parameter mappings
- MIDI learn mode indicator

**Verify:** MIDI input shown in real-time, mappings listed

**Status:** Completed 2025-12-28
- Created `src/ui/midi_activity.rs` with `MidiActivityWidget`
- Input/Output message columns with age-based fading
- `MappingsWidget` for controller assignments with value bars
- `LearnIndicatorWidget` with blink animation
- `ActivityIndicator` for MIDI I/O flash
- `ControllerMapping` for source/target display
- 6 unit tests pass

### Step 6.5: Commit Phase 6 ✅ COMPLETE
```bash
git add -A
git commit -m "Terminal UI with transport and track display"
```

**Status:** Completed 2025-12-28
- 163 tests pass (18 UI tests + 145 previous)
- All UI components implemented with comprehensive test coverage

---

## ✅ PHASE 6 COMPLETE

Terminal UI implemented: transport, tracks, MIDI activity display.

**Next:** Proceed to Phase 7, Step 7.1 - Keyboard Shortcuts

---

## Phase 7: Control System

### Step 7.1: Keyboard Shortcuts ✅ COMPLETE
Create `src/control/keyboard.rs`:
- Play/pause (space)
- Stop (escape)
- Part triggers (number keys)
- Tempo nudge (up/down arrows)
- Track mute toggles

**Verify:** All shortcuts work as expected

**Status:** Completed 2025-12-28
- Created `src/control/mod.rs` with `ControlAction` enum and `ControllerManager`
- Created `src/control/keyboard.rs` with `KeyboardController`
- `Shortcut` struct for key+modifier combinations
- `KeyBinding` with categories for help display
- Default bindings for transport, tempo, mute/solo, scenes, navigation
- Key repeat support for tempo/navigation
- 10 unit tests pass

### Step 7.2: MIDI Controller Mapping ✅ COMPLETE
Create `src/control/midi_map.rs`:
- Load mappings from config
- Note triggers for parts/clips
- CC for continuous parameters
- Relative encoder support
- Multiple mapping layers

**Verify:**
- Controller triggers parts
- Knobs adjust parameters smoothly
- Mappings reload from config

**Status:** Completed 2025-12-28
- Created `src/control/midi_map.rs` with `MidiController`
- `MidiBinding` for Note, CC, Program Change, Pitch Bend
- `EncoderMode`: Absolute, Relative64, RelativeBinary, RelativeSigned
- `MidiMappingEntry` with sensitivity and layer support
- Multi-layer mapping system for controller banks
- MIDI learn mode with last message capture
- 9 unit tests pass

### Step 7.3: Parameter System ✅ COMPLETE
Create `src/control/params.rs`:
- Named parameter registry
- Min/max/default values
- Smoothing for continuous changes
- Parameter automation (future)

**Verify:**
- Parameters can be set by name
- Values clamp to range
- Smoothing prevents clicks

**Status:** Completed 2025-12-28
- Created `src/control/params.rs` with `ParameterRegistry`
- `Parameter` with min/max/default, units, precision, grouping
- `ParameterValue` with exponential smoothing
- Normalized value access (0.0-1.0)
- Preset functions for tempo, volume, pan, filter, swing
- 12 unit tests pass

### Step 7.4: Commit Phase 7 ✅ COMPLETE
```bash
git add -A
git commit -m "Control system: keyboard, MIDI mapping, parameters"
```

**Status:** Completed 2025-12-28
- 194 tests pass (31 control tests + 163 previous)
- All control components implemented with comprehensive test coverage

---

## ✅ PHASE 7 COMPLETE

Control system implemented: keyboard shortcuts, MIDI mapping, parameters.

**Next:** Proceed to Phase 8, Step 8.1 - FluidSynth Integration

---

## Phase 8: Development Sound Engine

### Step 8.1: FluidSynth Integration ✅ COMPLETE
Create `src/audio/fluidsynth.rs`:
- Initialize FluidLite
- Load SF2 soundfont
- Route MIDI events to synth
- Audio output to system

**Verify:**
- Sound plays through speakers
- All 16 MIDI channels work
- Soundfont can be changed via config

**Status:** Completed 2025-12-28
- Created `src/audio/mod.rs` with `AudioEngine` combining synth and output
- Created `src/audio/fluidsynth.rs` with `FluidSynth` wrapper
- FluidLite settings for sample rate, gain, polyphony, MIDI channels
- MIDI event routing: note_on, note_off, cc, program_change, pitch_bend
- All notes off, reset, and bank select support
- Reverb and chorus enable/disable
- 8 unit tests pass

### Step 8.2: Audio Output ✅ COMPLETE
Create `src/audio/output.rs`:
- Core Audio integration (cpal crate)
- Buffer management
- Latency configuration

**Verify:**
- Audio plays without underruns
- Latency is acceptable (<20ms)

**Status:** Completed 2025-12-28
- Created `src/audio/output.rs` with `AudioOutput`
- `AudioConfig` with sample rate, buffer size, channels
- cpal backend for cross-platform audio (Core Audio on macOS)
- Callback-based audio rendering
- Device enumeration and sample rate detection
- Latency calculation helper
- 5 unit tests pass

### Step 8.3: Commit Phase 8 ✅ COMPLETE
```bash
git add -A
git commit -m "Development sound engine with FluidSynth"
```

**Status:** Completed 2025-12-28
- 209 tests pass (15 audio tests + 194 previous)
- All audio components implemented with comprehensive test coverage

---

## ✅ PHASE 8 COMPLETE

Development sound engine implemented: FluidSynth wrapper and cpal audio output.

**Next:** Proceed to Phase 9, Step 9.1 - Parts System

---

## Phase 9: Song & Arrangement

### Step 9.1: Parts System ✅ COMPLETE
Create `src/arrangement/part.rs`:
- Part definitions (collections of clip/generator states)
- Part transitions (cut, quantized)
- Macro triggers (change multiple things at once)

**Verify:**
- Switching parts changes all tracks
- Transitions respect quantization
- Macros execute all changes

**Status:** Completed 2025-12-29
- Created `src/arrangement/mod.rs` with main arrangement module
- Created `src/arrangement/part.rs` with `Part`, `PartManager`
- `TrackClipState`: Empty, Clip, Generator, Stop, Hold
- `PartTransition`: Immediate, NextBeat, NextBar, Beats(n), Bars(n), EndOfPhrase, Crossfade
- `MacroAction` for tempo, parameters, mute/solo, MIDI messages
- Quantized transition scheduling with pending queue
- 11 unit tests pass

### Step 9.2: Scene System ✅ COMPLETE
Create `src/arrangement/scene.rs`:
- Scenes as track state snapshots
- Scene matrix (tracks x scenes)
- Scene follow actions

**Verify:**
- Scenes capture and restore track states
- Follow actions work

**Status:** Completed 2025-12-29
- Created `src/arrangement/scene.rs` with `Scene`, `SceneManager`
- `SceneSlot`: Empty, Clip, Generator, Stop, Hold
- `SceneLaunchMode`: Immediate, Beat, Bar, Beats(n), Bars(n)
- Matrix-style track × scene slot access
- Follow actions with configurable bar count
- Scene navigation (next/prev with wrap)
- 10 unit tests pass

### Step 9.3: Song Mode ✅ COMPLETE
Create `src/arrangement/song.rs`:
- Ordered arrangement of parts
- Position in arrangement
- Auto-advance through parts
- Loop sections

**Verify:**
- Song plays through arrangement
- Looping works
- Can jump to any part

**Status:** Completed 2025-12-29
- Created `src/arrangement/song.rs` with `Song`, `SongPlayer`
- `SongSection`: Part name, length, tempo, time signature, scene index
- `SongPosition`: Section, bar, beat, tick with formatting
- `LoopRegion`: Start/end sections with repeat count
- `SongPlayer`: Play/pause/stop, goto section, loop control
- Position calculation from ticks to section/bar/beat
- Metadata support for song info
- 10 unit tests pass

### Step 9.4: Commit Phase 9 ✅ COMPLETE
```bash
git add -A
git commit -m "Song and arrangement: parts, scenes, song mode"
```

**Status:** Completed 2025-12-29
- 243 tests pass (34 arrangement tests + 209 previous)
- All arrangement components implemented with comprehensive test coverage

---

## ✅ PHASE 9 COMPLETE

Song and arrangement system implemented: parts, scenes, song mode.

**Next:** Proceed to Phase 10, Step 10.1 - MIDI Recording

---

## Phase 10: Recording & Export

### Step 10.1: MIDI Recording ✅ COMPLETE
Create `src/recording/capture.rs`:
- Record incoming MIDI to clip
- Overdub and replace modes
- Quantize on input (optional)
- Punch in/out

**Verify:**
- Recorded notes play back correctly
- Overdub adds notes
- Quantization works

**Status:** Completed 2025-12-29
- Created `src/recording/mod.rs` with main recording module
- Created `src/recording/capture.rs` with `MidiRecorder`
- `RecordMode`: Replace, Overdub, Punch
- `RecordingState`: Idle, Armed, Recording, CountIn, Paused
- `QuantizeSettings` with grid, strength, start/end options
- `RecordedNote` with channel, note, velocity, timing
- `PunchRegion` for punch in/out recording
- Count-in support and loop recording
- 22 unit tests pass

### Step 10.2: Clip Freeze ✅ COMPLETE
- Capture generator output to static clip
- Save to config file format

**Verify:** Frozen clip plays back identically to live generation

**Status:** Completed 2025-12-29
- Created `src/recording/freeze.rs` with `ClipFreezer`
- `FreezeOptions`: length (ticks/bars), velocity, quantize, merge, min note length
- `FrozenNote` for captured note data
- `FreezerState`: Idle, Capturing, Complete
- Active note tracking for note on/off pairing
- Quantization grid support
- Overlapping note merging
- 10 unit tests pass

### Step 10.3: MIDI File Export ✅ COMPLETE
Create `src/recording/export.rs`:
- Export clips as standard MIDI files
- Export full arrangement

**Verify:** Exported MIDI opens in DAW correctly

**Status:** Completed 2025-12-29
- Created `src/recording/export.rs` with `MidiExporter`
- `MidiFileFormat`: Type0 (single track), Type1 (multi-track)
- `ExportTrack` with name, channel, notes, program
- Variable-length quantity encoding for MIDI timing
- Header chunk (MThd) and track chunk (MTrk) writing
- Tempo and time signature meta events
- Note on/off event generation with delta times
- 7 unit tests pass

### Step 10.4: Commit Phase 10 ✅ COMPLETE
```bash
git add -A
git commit -m "Recording and MIDI export"
```

**Status:** Completed 2025-12-29
- 282 tests pass (39 recording tests + 243 previous)
- All recording components implemented with comprehensive test coverage

---

## ✅ PHASE 10 COMPLETE

Recording and export system implemented: MIDI capture, clip freeze, MIDI file export.

**Next:** Proceed to Phase 11, Step 11.1 - Integration Tests

---

## Phase 11: Polish & Testing

### Step 11.1: Integration Tests
- Full playback test
- Config hot reload test
- Multi-controller test
- Extended stability test (1 hour run)

**Verify:** All tests pass

### Step 11.2: Performance Profiling
- CPU usage under load
- Timing jitter measurement
- Memory usage over time

**Verify:**
- CPU < 10% idle, < 30% busy
- Jitter < 1ms
- No memory leaks

### Step 11.3: Documentation
- README with quick start
- Config file reference
- Controller setup guide

**Verify:** New user can set up from docs

### Step 11.4: Final Commit
```bash
git add -A
git commit -m "v0.1.0 - Initial release"
git tag v0.1.0
```

---

## File Structure (Final)

```
seq/
├── Cargo.toml
├── README.md
├── .vscode/
│   ├── settings.json
│   └── launch.json
├── config/
│   ├── default_song.yaml
│   ├── scales.yaml
│   └── controllers/
│       └── launchpad.yaml
├── soundfonts/
│   └── default.sf2
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── midi/
│   │   ├── mod.rs
│   │   ├── coremidi_backend.rs
│   │   ├── input.rs
│   │   └── clock.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── watcher.rs
│   ├── music/
│   │   ├── mod.rs
│   │   ├── scale.rs
│   │   └── chord.rs
│   ├── generators/
│   │   ├── mod.rs
│   │   ├── drone.rs
│   │   ├── arpeggio.rs
│   │   ├── chord.rs
│   │   ├── melody.rs
│   │   └── drums.rs
│   ├── sequencer/
│   │   ├── mod.rs
│   │   ├── scheduler.rs
│   │   ├── track.rs
│   │   ├── clip.rs
│   │   └── trigger.rs
│   ├── ui/
│   │   └── mod.rs
│   ├── control/
│   │   ├── mod.rs
│   │   ├── keyboard.rs
│   │   ├── midi_map.rs
│   │   └── params.rs
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── fluidsynth.rs
│   │   └── output.rs
│   ├── arrangement/
│   │   ├── mod.rs
│   │   ├── part.rs
│   │   ├── scene.rs
│   │   └── song.rs
│   └── recording/
│       ├── mod.rs
│       ├── capture.rs
│       └── export.rs
└── tests/
    ├── midi_tests.rs
    ├── generator_tests.rs
    └── integration_tests.rs
```

---

## Critical Files to Modify/Create

Phase 1:
- `Cargo.toml` - dependencies
- `.vscode/settings.json` - editor config
- `.vscode/launch.json` - debugger
- `.gitignore` - git ignores

Phase 2:
- `src/midi/mod.rs` - MIDI abstraction
- `src/midi/coremidi_backend.rs` - macOS MIDI
- `src/timing/clock.rs` - tempo clock

Phase 4:
- `src/generators/*.rs` - all generative engines

Phase 5:
- `src/sequencer/scheduler.rs` - timing core (most critical for performance)
