# SEQ - Algorithmic MIDI Sequencer

An algorithmic MIDI sequencer for live performance, built in Rust targeting macOS with Core MIDI integration.

## Features

- **Generative Engines**: Drone, arpeggio, chord progressions, melodies, and drum patterns
- **Live Performance**: Parts, scenes, and song mode for structured improvisation
- **MIDI I/O**: Full Core MIDI integration with clock sync
- **Recording**: MIDI capture, clip freeze, and Standard MIDI File export
- **Terminal UI**: Real-time display with ratatui
- **Hot Reload**: Live configuration changes without stopping playback
- **Development Sound Engine**: Built-in FluidSynth for testing without external gear

## Requirements

- macOS (Core MIDI support)
- Rust 1.70+ (install via [rustup](https://rustup.rs))
- Optional: SoundFont file (.sf2) for built-in audio

## Quick Start

### Build

```bash
git clone https://github.com/yourusername/seq.git
cd seq
cargo build --release
```

### List MIDI Devices

```bash
# List available MIDI outputs
cargo run -- --list-midi

# List available MIDI inputs
cargo run -- --list-sources
```

### Test MIDI Output

```bash
# Send a test note to MIDI destination 0
cargo run -- --test-note 0

# Send MIDI clock at 120 BPM to destination 0
cargo run -- --test-clock 0 120
```

### Monitor MIDI Input

```bash
# Monitor input from MIDI source 0
cargo run -- --monitor 0
```

## Architecture

```
seq/
├── src/
│   ├── main.rs           # CLI and entry point
│   ├── midi/             # MIDI I/O and clock
│   ├── timing/           # Clock and tempo management
│   ├── generators/       # Generative engines
│   ├── sequencer/        # Clips, tracks, and scheduling
│   ├── arrangement/      # Parts, scenes, and songs
│   ├── recording/        # MIDI capture and export
│   ├── control/          # Keyboard and MIDI control
│   ├── config/           # YAML configuration and hot reload
│   ├── music/            # Scales and music theory
│   ├── audio/            # FluidSynth integration
│   └── ui/               # Terminal UI widgets
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
└── config/               # Sample configurations
```

## Generators

### Drone Generator
Sustained ambient notes with slow evolution.

Parameters:
- `voices`: Number of simultaneous voices (1-8)
- `change_rate`: How often notes change (in ticks)
- `octave_spread`: Range of octaves
- `base_octave`: Starting octave

### Arpeggiator
Pattern-based note sequencing.

Patterns:
- `Up`, `Down`, `UpDown`, `DownUp`, `Random`, `Order`

Parameters:
- `pattern`: Arpeggio pattern type
- `octave_range`: Number of octaves to span
- `gate`: Note length as percentage (0.0-1.0)
- `probability`: Chance each note plays (0.0-1.0)

### Chord Generator
Harmonic progressions with voicings.

Voicings:
- `Close`: Standard close voicing
- `Open`: Spread across octaves
- `Drop2`: Jazz drop-2 voicing
- `Spread`: Wide interval voicing

Progression modes:
- `Functional`: I-IV-V-I style progressions
- `RandomInKey`: Random chords within scale
- `Custom`: User-defined progressions

### Melody Generator
Markov-based melodic generation.

Parameters:
- `note_range`: MIDI note range
- `interval_probabilities`: Weights for interval selection
- `phrase_length`: Notes per phrase

Transforms:
- `Original`, `Transpose`, `Invert`, `Retrograde`

### Drum Generator
Rhythmic pattern generation.

Styles:
- `FourOnFloor`: Classic house/techno kick
- `Breakbeat`: Syncopated patterns
- `Sparse`: Minimal hits
- `Busy`: Dense patterns
- `Euclidean`: Mathematically distributed hits
- `Random`: Probability-based

## Configuration

### Song Configuration (YAML)

```yaml
name: "My Song"
tempo: 120.0
time_signature: [4, 4]
key: C
scale: Major

tracks:
  - name: "Bass"
    channel: 0
    generator:
      type: "arpeggio"
      pattern: "up"
      octave_range: 1
      gate: 0.8

  - name: "Chords"
    channel: 1
    generator:
      type: "chord"
      voicing: "open"
      progression: "functional"

  - name: "Drums"
    channel: 9
    generator:
      type: "drums"
      style: "four_on_floor"
      humanize: 0.1

parts:
  - name: "Intro"
    tracks:
      0: { state: "playing" }
      1: { state: "muted" }
      2: { state: "muted" }

  - name: "Main"
    tracks:
      0: { state: "playing" }
      1: { state: "playing" }
      2: { state: "playing" }
```

### Controller Mapping

```yaml
device: "Launchpad"
channel: 0

mappings:
  # Transport
  - type: note
    note: 36
    action: play

  - type: note
    note: 37
    action: stop

  # Tempo
  - type: cc
    cc: 1
    action: set_tempo
    min: 60
    max: 180

  # Part triggers
  - type: note
    note: 48
    action: trigger_part
    part: "Intro"

  - type: note
    note: 49
    action: trigger_part
    part: "Main"

  # Track mutes
  - type: note
    note: 60
    action: mute_track
    track: 0
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | Play/Pause |
| Escape | Stop |
| 1-9 | Trigger parts |
| Up/Down | Nudge tempo |
| M + 1-9 | Mute track |
| S + 1-9 | Solo track |
| Q | Quit |

## Timing

SEQ uses 24 PPQN (Pulses Per Quarter Note), the standard MIDI clock resolution:

- 1 quarter note = 24 ticks
- 1 eighth note = 12 ticks
- 1 sixteenth note = 6 ticks
- 1 bar (4/4) = 96 ticks

At 120 BPM:
- 1 tick = 20.83ms
- 1 beat = 500ms
- 1 bar = 2 seconds

## MIDI File Export

Export recordings as Standard MIDI Files:

```rust
let exporter = MidiExporter::new()
    .format(MidiFileFormat::Type1)
    .tempo(120.0)
    .time_signature(4, 4);

exporter.add_track("Lead", 0, notes);
exporter.export("output.mid")?;
```

Supports:
- Type 0 (single track) and Type 1 (multi-track) formats
- Tempo and time signature meta events
- Program changes per track

## Performance

Run benchmarks:

```bash
cargo bench
```

Key metrics:
- Timing conversion: ~1.2ns
- Event scheduling: <1us per event
- Scale quantization: <100ns per note

## Testing

```bash
# Run all tests (282 tests)
cargo test

# Run integration tests
cargo test --test integration_tests

# Run with output
cargo test -- --nocapture
```

## Development

### Code Structure

- **Traits**: `Generator`, `MidiOutput` define interfaces
- **Registry pattern**: Generators registered by name
- **Builder pattern**: Configuration structs use builders
- **Event-driven**: Scheduler processes time-ordered events

### Adding a Generator

1. Create `src/generators/my_generator.rs`
2. Implement the `Generator` trait
3. Register in `GeneratorRegistry`
4. Add to `src/generators/mod.rs`

```rust
pub struct MyGenerator {
    // parameters
}

impl Generator for MyGenerator {
    fn generate(&mut self, context: &GeneratorContext) -> Vec<MidiEvent> {
        // Generate MIDI events
    }

    fn set_param(&mut self, name: &str, value: f64) {
        // Handle parameter changes
    }

    fn reset(&mut self) {
        // Reset state
    }
}
```

## Project Status

v0.1.0 - Initial release with all core features implemented:
- 11 phases of development completed
- 282 tests passing
- Performance benchmarks included

## License

MIT License - see LICENSE file for details.

## Author

Robert L. Snyder, Sierra Vista, AZ
