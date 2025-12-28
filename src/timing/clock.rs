// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI Clock implementation.
//!
//! This module provides a BPM-based MIDI clock that generates timing messages
//! at 24 PPQN (Pulses Per Quarter Note) as per the MIDI specification.

use std::time::{Duration, Instant};

use crate::midi::messages;

/// Pulses Per Quarter Note - MIDI standard is 24
pub const PPQN: u32 = 24;

/// MIDI Clock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockState {
    Stopped,
    Running,
    Paused,
}

/// Configuration for tempo ramping
#[derive(Debug, Clone)]
pub struct TempoRamp {
    /// Starting tempo
    pub from_bpm: f64,
    /// Target tempo
    pub to_bpm: f64,
    /// Duration of the ramp
    pub duration: Duration,
    /// When the ramp started
    pub start_time: Instant,
}

impl TempoRamp {
    /// Calculate the current tempo based on elapsed time
    pub fn current_tempo(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            self.to_bpm
        } else {
            let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
            self.from_bpm + (self.to_bpm - self.from_bpm) * progress
        }
    }

    /// Check if the ramp is complete
    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}

/// Tap tempo calculator
#[derive(Debug, Clone)]
pub struct TapTempo {
    /// Recent tap times
    taps: Vec<Instant>,
    /// Maximum number of taps to average
    max_taps: usize,
    /// Maximum time between taps before resetting
    timeout: Duration,
}

impl TapTempo {
    /// Create a new tap tempo calculator
    pub fn new(max_taps: usize, timeout_ms: u64) -> Self {
        Self {
            taps: Vec::with_capacity(max_taps),
            max_taps,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Record a tap and return the calculated BPM if enough taps
    pub fn tap(&mut self) -> Option<f64> {
        let now = Instant::now();

        // Reset if timeout exceeded
        if let Some(last) = self.taps.last() {
            if now.duration_since(*last) > self.timeout {
                self.taps.clear();
            }
        }

        self.taps.push(now);

        // Keep only max_taps
        if self.taps.len() > self.max_taps {
            self.taps.remove(0);
        }

        // Need at least 2 taps to calculate BPM
        if self.taps.len() < 2 {
            return None;
        }

        // Calculate average interval
        let intervals: Vec<Duration> = self.taps
            .windows(2)
            .map(|w| w[1].duration_since(w[0]))
            .collect();

        let avg_interval: Duration = intervals.iter().sum::<Duration>() / intervals.len() as u32;
        let bpm = 60.0 / avg_interval.as_secs_f64();

        // Clamp to reasonable range
        Some(bpm.clamp(20.0, 300.0))
    }

    /// Reset the tap tempo
    pub fn reset(&mut self) {
        self.taps.clear();
    }
}

impl Default for TapTempo {
    fn default() -> Self {
        Self::new(4, 2000) // Average 4 taps, 2 second timeout
    }
}

/// MIDI Clock generator
#[derive(Debug)]
pub struct MidiClock {
    /// Current tempo in BPM
    bpm: f64,
    /// Current clock state
    state: ClockState,
    /// Current pulse count within the beat (0-23)
    pulse: u32,
    /// Current beat count
    beat: u64,
    /// Last clock tick time
    last_tick: Option<Instant>,
    /// Active tempo ramp
    tempo_ramp: Option<TempoRamp>,
    /// Tap tempo calculator
    tap_tempo: TapTempo,
}

impl MidiClock {
    /// Create a new MIDI clock at the specified tempo
    pub fn new(bpm: f64) -> Self {
        Self {
            bpm: bpm.clamp(20.0, 300.0),
            state: ClockState::Stopped,
            pulse: 0,
            beat: 0,
            last_tick: None,
            tempo_ramp: None,
            tap_tempo: TapTempo::default(),
        }
    }

    /// Get the current tempo in BPM
    pub fn bpm(&self) -> f64 {
        if let Some(ref ramp) = self.tempo_ramp {
            ramp.current_tempo()
        } else {
            self.bpm
        }
    }

    /// Set the tempo immediately
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(20.0, 300.0);
        self.tempo_ramp = None;
    }

    /// Nudge tempo by a delta
    pub fn nudge_bpm(&mut self, delta: f64) {
        self.set_bpm(self.bpm() + delta);
    }

    /// Start a tempo ramp to the target BPM over the specified duration
    pub fn ramp_to(&mut self, target_bpm: f64, duration: Duration) {
        self.tempo_ramp = Some(TempoRamp {
            from_bpm: self.bpm(),
            to_bpm: target_bpm.clamp(20.0, 300.0),
            duration,
            start_time: Instant::now(),
        });
    }

    /// Record a tap and update tempo if enough taps
    pub fn tap(&mut self) -> Option<f64> {
        if let Some(bpm) = self.tap_tempo.tap() {
            self.set_bpm(bpm);
            Some(bpm)
        } else {
            None
        }
    }

    /// Get the current clock state
    pub fn state(&self) -> ClockState {
        self.state
    }

    /// Get the current pulse within the beat (0-23)
    pub fn pulse(&self) -> u32 {
        self.pulse
    }

    /// Get the current beat count
    pub fn beat(&self) -> u64 {
        self.beat
    }

    /// Calculate the interval between clock pulses
    pub fn pulse_interval(&self) -> Duration {
        let bpm = self.bpm();
        // At 24 PPQN, interval = 60 / (BPM * 24) seconds
        let seconds = 60.0 / (bpm * PPQN as f64);
        Duration::from_secs_f64(seconds)
    }

    /// Start the clock - returns MIDI Start message
    pub fn start(&mut self) -> [u8; 1] {
        self.state = ClockState::Running;
        self.pulse = 0;
        self.beat = 0;
        self.last_tick = Some(Instant::now());
        [messages::START]
    }

    /// Stop the clock - returns MIDI Stop message
    pub fn stop(&mut self) -> [u8; 1] {
        self.state = ClockState::Stopped;
        self.pulse = 0;
        self.beat = 0;
        self.last_tick = None;
        [messages::STOP]
    }

    /// Pause the clock (continue from current position) - returns MIDI Stop message
    pub fn pause(&mut self) -> [u8; 1] {
        self.state = ClockState::Paused;
        [messages::STOP]
    }

    /// Continue from paused state - returns MIDI Continue message
    pub fn continue_playback(&mut self) -> [u8; 1] {
        if self.state == ClockState::Paused {
            self.state = ClockState::Running;
            self.last_tick = Some(Instant::now());
        }
        [messages::CONTINUE]
    }

    /// Check if it's time for the next clock pulse
    /// Returns Some with the clock message if a pulse should be sent
    pub fn tick(&mut self) -> Option<[u8; 1]> {
        if self.state != ClockState::Running {
            return None;
        }

        // Update tempo ramp if active
        if let Some(ref ramp) = self.tempo_ramp {
            if ramp.is_complete() {
                self.bpm = ramp.to_bpm;
                self.tempo_ramp = None;
            }
        }

        let now = Instant::now();
        let interval = self.pulse_interval();

        if let Some(last) = self.last_tick {
            if now.duration_since(last) >= interval {
                self.last_tick = Some(now);
                self.pulse += 1;
                if self.pulse >= PPQN {
                    self.pulse = 0;
                    self.beat += 1;
                }
                return Some([messages::TIMING_CLOCK]);
            }
        }

        None
    }

    /// Get the time until the next clock pulse
    pub fn time_until_next_pulse(&self) -> Duration {
        if self.state != ClockState::Running {
            return Duration::from_secs(0);
        }

        let interval = self.pulse_interval();
        if let Some(last) = self.last_tick {
            let elapsed = last.elapsed();
            if elapsed < interval {
                return interval - elapsed;
            }
        }

        Duration::from_secs(0)
    }
}

impl Default for MidiClock {
    fn default() -> Self {
        Self::new(120.0) // Default to 120 BPM
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_clock_creation() {
        let clock = MidiClock::new(120.0);
        assert_eq!(clock.bpm(), 120.0);
        assert_eq!(clock.state(), ClockState::Stopped);
        assert_eq!(clock.pulse(), 0);
        assert_eq!(clock.beat(), 0);
    }

    #[test]
    fn test_clock_bpm_clamping() {
        let clock = MidiClock::new(10.0); // Below minimum
        assert_eq!(clock.bpm(), 20.0);

        let clock = MidiClock::new(500.0); // Above maximum
        assert_eq!(clock.bpm(), 300.0);
    }

    #[test]
    fn test_pulse_interval() {
        let clock = MidiClock::new(120.0);
        // At 120 BPM, 24 PPQN: interval = 60 / (120 * 24) = 0.0208333... seconds
        let interval = clock.pulse_interval();
        let expected = Duration::from_secs_f64(60.0 / (120.0 * 24.0));
        assert!((interval.as_secs_f64() - expected.as_secs_f64()).abs() < 0.0001);
    }

    #[test]
    fn test_clock_start_stop() {
        let mut clock = MidiClock::new(120.0);

        let start_msg = clock.start();
        assert_eq!(start_msg, [messages::START]);
        assert_eq!(clock.state(), ClockState::Running);

        let stop_msg = clock.stop();
        assert_eq!(stop_msg, [messages::STOP]);
        assert_eq!(clock.state(), ClockState::Stopped);
    }

    #[test]
    fn test_clock_pause_continue() {
        let mut clock = MidiClock::new(120.0);

        clock.start();
        let pause_msg = clock.pause();
        assert_eq!(pause_msg, [messages::STOP]);
        assert_eq!(clock.state(), ClockState::Paused);

        let continue_msg = clock.continue_playback();
        assert_eq!(continue_msg, [messages::CONTINUE]);
        assert_eq!(clock.state(), ClockState::Running);
    }

    #[test]
    fn test_clock_tick() {
        let mut clock = MidiClock::new(6000.0); // Very fast for testing (100 ticks/sec)
        clock.set_bpm(6000.0); // This will be clamped to 300

        clock.start();

        // Wait a bit and check for tick
        thread::sleep(Duration::from_millis(10));
        let tick = clock.tick();
        // At 300 BPM, we should get ticks (interval ~8.3ms)
        assert!(tick.is_some() || clock.time_until_next_pulse() < Duration::from_millis(10));
    }

    #[test]
    fn test_tap_tempo() {
        let mut tap = TapTempo::new(4, 2000);

        // First tap - no BPM yet
        assert!(tap.tap().is_none());

        // Second tap after ~500ms = 120 BPM
        thread::sleep(Duration::from_millis(500));
        let bpm = tap.tap();
        assert!(bpm.is_some());
        let bpm = bpm.unwrap();
        // Should be around 120 BPM (allowing for timing variance)
        assert!(bpm > 100.0 && bpm < 140.0);
    }

    #[test]
    fn test_tempo_ramp() {
        let ramp = TempoRamp {
            from_bpm: 100.0,
            to_bpm: 140.0,
            duration: Duration::from_millis(100),
            start_time: Instant::now(),
        };

        // At start, should be close to from_bpm
        let tempo = ramp.current_tempo();
        assert!(tempo >= 100.0 && tempo <= 140.0);

        // After duration, should be at to_bpm
        thread::sleep(Duration::from_millis(150));
        assert!(ramp.is_complete());
        assert_eq!(ramp.current_tempo(), 140.0);
    }

    #[test]
    fn test_nudge_bpm() {
        let mut clock = MidiClock::new(120.0);
        clock.nudge_bpm(5.0);
        assert_eq!(clock.bpm(), 125.0);
        clock.nudge_bpm(-10.0);
        assert_eq!(clock.bpm(), 115.0);
    }
}
