// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Timing and clock module.
//!
//! This module provides MIDI clock generation and timing utilities
//! for the sequencer.

pub mod clock;

pub use clock::{ClockState, MidiClock, TapTempo, TempoRamp, PPQN};
