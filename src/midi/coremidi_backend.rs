// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Core MIDI backend for macOS.
//!
//! This module provides a Core MIDI implementation of the `MidiOutput` trait,
//! allowing SEQ to send MIDI messages to external devices on macOS.

use anyhow::{anyhow, Result};
use coremidi::{Client, Destination, Destinations, OutputPort, PacketBuffer};

use super::MidiOutput;

/// Core MIDI output implementation for macOS.
pub struct CoreMidiOutput {
    _client: Client,
    output_port: OutputPort,
    destination: Destination,
}

impl CoreMidiOutput {
    /// Create a new Core MIDI output connected to the specified destination.
    ///
    /// # Arguments
    /// * `destination_index` - Index of the destination in the system's MIDI device list
    ///
    /// # Returns
    /// * `Ok(CoreMidiOutput)` on success
    /// * `Err` if the client, port, or connection could not be created
    pub fn new(destination_index: usize) -> Result<Self> {
        let client = Client::new("SEQ")
            .map_err(|e| anyhow!("Failed to create MIDI client: {:?}", e))?;

        let output_port = client
            .output_port("SEQ Output")
            .map_err(|e| anyhow!("Failed to create output port: {:?}", e))?;

        let count = Destinations::count();
        if destination_index >= count {
            return Err(anyhow!(
                "MIDI destination {} not found (only {} available)",
                destination_index,
                count
            ));
        }

        let destination = Destination::from_index(destination_index)
            .ok_or_else(|| anyhow!("MIDI destination {} not found", destination_index))?;

        Ok(Self {
            _client: client,
            output_port,
            destination,
        })
    }

    /// Create a new Core MIDI output connected to a destination by name.
    ///
    /// # Arguments
    /// * `name` - Partial name to match against destination names
    ///
    /// # Returns
    /// * `Ok(CoreMidiOutput)` on success
    /// * `Err` if no matching destination is found
    pub fn new_by_name(name: &str) -> Result<Self> {
        let destinations = list_destinations();
        let index = destinations
            .iter()
            .position(|(_, n)| n.to_lowercase().contains(&name.to_lowercase()))
            .ok_or_else(|| anyhow!("No MIDI destination matching '{}' found", name))?;

        Self::new(destinations[index].0)
    }
}

impl MidiOutput for CoreMidiOutput {
    fn send(&mut self, message: &[u8]) -> Result<()> {
        // Use timestamp 0 for immediate sending
        self.send_at(message, 0)
    }

    fn send_at(&mut self, message: &[u8], timestamp: u64) -> Result<()> {
        let packet_buffer = PacketBuffer::new(timestamp, message);
        self.output_port
            .send(&self.destination, &packet_buffer)
            .map_err(|e| anyhow!("Failed to send MIDI message: {:?}", e))?;
        Ok(())
    }
}

/// List all available MIDI destinations.
///
/// # Returns
/// A vector of (index, name) tuples.
pub fn list_destinations() -> Vec<(usize, String)> {
    let mut result = Vec::new();

    for (i, dest) in Destinations.into_iter().enumerate() {
        let name = dest.display_name().unwrap_or_else(|| format!("Unknown {}", i));
        result.push((i, name));
    }

    result
}

/// Get the number of available MIDI destinations.
pub fn destination_count() -> usize {
    Destinations::count()
}

/// Print all available MIDI destinations to stdout.
pub fn print_destinations() {
    let destinations = list_destinations();
    if destinations.is_empty() {
        println!("No MIDI destinations found.");
    } else {
        println!("Available MIDI destinations:");
        for (i, name) in destinations {
            println!("  {}: {}", i, name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_destinations() {
        // This test just verifies that listing destinations doesn't panic
        let destinations = list_destinations();
        // We can't assert on the actual destinations since they vary by system
        println!("Found {} destinations", destinations.len());
    }

    #[test]
    fn test_destination_count() {
        let count = destination_count();
        let list = list_destinations();
        // Count should match list length
        assert_eq!(count, list.len());
    }
}
