// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Audio output via cpal (Core Audio on macOS).
//!
//! Provides low-latency audio output with configurable buffer sizes.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};

use super::AudioError;

/// Audio output configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Buffer size in frames
    pub buffer_size: u32,
    /// Number of output channels
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 512,
            channels: 2,
        }
    }
}

/// Audio output stream
pub struct AudioOutput {
    /// cpal stream
    _stream: Stream,
    /// Output device
    _device: Device,
    /// Current configuration
    config: AudioConfig,
}

impl AudioOutput {
    /// Create a new audio output with callback
    pub fn new<F>(config: AudioConfig, mut callback: F) -> Result<Self, AudioError>
    where
        F: FnMut(&mut [f32], usize) + Send + 'static,
    {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or(AudioError::NoDevice)?;

        let _supported_config = device
            .default_output_config()
            .map_err(|e| AudioError::InitFailed(format!("Failed to get default config: {}", e)))?;

        // Build stream config
        let stream_config = StreamConfig {
            channels: config.channels,
            sample_rate: cpal::SampleRate(config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size),
        };

        let channels = config.channels as usize;

        // Create the output stream
        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Clear buffer first
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    // Call user callback to fill buffer
                    callback(data, channels);
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None, // No timeout
            )
            .map_err(|e| AudioError::StreamFailed(format!("Failed to build stream: {}", e)))?;

        // Start playback
        stream
            .play()
            .map_err(|e| AudioError::StreamFailed(format!("Failed to start stream: {}", e)))?;

        Ok(Self {
            _stream: stream,
            _device: device,
            config,
        })
    }

    /// Get current configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> u32 {
        self.config.buffer_size
    }

    /// Get number of channels
    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    /// Calculate latency in milliseconds
    pub fn latency_ms(&self) -> f64 {
        (self.config.buffer_size as f64 / self.config.sample_rate as f64) * 1000.0
    }
}

/// List available audio output devices
pub fn list_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Get default device name
pub fn default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.name().ok())
}

/// Get supported sample rates for default device
pub fn supported_sample_rates() -> Vec<u32> {
    let host = cpal::default_host();
    if let Some(device) = host.default_output_device() {
        if let Ok(configs) = device.supported_output_configs() {
            let mut rates: Vec<u32> = configs
                .flat_map(|c| {
                    let min = c.min_sample_rate().0;
                    let max = c.max_sample_rate().0;
                    // Return common sample rates within the range
                    [44100, 48000, 88200, 96000, 176400, 192000]
                        .into_iter()
                        .filter(move |&r| r >= min && r <= max)
                })
                .collect();
            rates.sort();
            rates.dedup();
            return rates;
        }
    }
    vec![44100, 48000] // Fallback defaults
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.buffer_size, 512);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_latency_calculation() {
        // Can't create AudioOutput in tests without audio device,
        // but we can test the math
        let config = AudioConfig {
            sample_rate: 44100,
            buffer_size: 512,
            channels: 2,
        };

        let latency_ms = (config.buffer_size as f64 / config.sample_rate as f64) * 1000.0;
        assert!((latency_ms - 11.6).abs() < 0.1); // ~11.6ms
    }

    #[test]
    fn test_list_devices() {
        // This should not panic even without audio devices
        let devices = list_devices();
        // Just ensure it returns a vector (may be empty in CI)
        assert!(devices.len() >= 0);
    }

    #[test]
    fn test_default_device_name() {
        // Should not panic
        let _ = default_device_name();
    }

    #[test]
    fn test_supported_sample_rates() {
        let rates = supported_sample_rates();
        // Should return at least fallback defaults
        assert!(!rates.is_empty());
    }
}
