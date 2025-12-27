# SEQ - Algorithmic MIDI Sequencer

A performance-focused MIDI sequencer for live electronic music, built in Rust for macOS.

## Overview

SEQ is designed for **minimal visual UI, maximum musical control**—operated via external MIDI controllers and keyboard shortcuts, configured through human-readable YAML/TOML files.

Primary use cases: ambient, synthwave, synthpop, and generative electronic music.

## Features

- **Algorithmic Generation**: Drone, arpeggio, chord, melody, and drum generators with configurable probability distributions
- **Scale-Aware**: All generators respect musical scales and keys with real-time transposition
- **Live Performance**: Part/scene triggering, quantized transitions, MIDI controller mapping
- **Hot Reload**: Edit configuration files while playing—changes apply without stopping
- **MIDI Clock**: Master/slave sync with external gear at 24 PPQN
- **Terminal UI**: Minimal ratatui-based interface showing transport and track status

## Requirements

- macOS (Core MIDI integration)
- Rust 1.70+
- External MIDI devices (optional, for performance mode)

## Installation

```bash
git clone https://github.com/mooneyedkitty/seq.git
cd seq
cargo build --release
```

## Usage

```bash
# Run the sequencer
cargo run

# List available MIDI devices
cargo run -- --list-midi
```

## Configuration

Songs are defined in YAML files:

```yaml
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
```

## Project Status

This project is in early development. See `plan.md` for the implementation roadmap.

## License

MIT License - see [LICENSE](LICENSE) for details.
