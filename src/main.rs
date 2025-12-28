// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

mod midi;
mod timing;

use anyhow::Result;
use midi::{print_destinations, print_sources, CoreMidiOutput, MidiInput, MidiOutput};
use timing::MidiClock;
use std::env;
use std::thread;
use std::time::{Duration, Instant};

fn print_usage() {
    println!("SEQ - Algorithmic MIDI Sequencer");
    println!();
    println!("Usage: seq [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --list-midi             List available MIDI destinations (outputs)");
    println!("  --list-sources          List available MIDI sources (inputs)");
    println!("  --test-note <N>         Send a test note to MIDI destination N");
    println!("  --test-clock <N> [BPM]  Send MIDI clock to destination N at BPM (default 120)");
    println!("  --monitor <N>           Monitor MIDI input from source N");
    println!("  --help                  Show this help message");
}

fn send_test_note(destination: usize) -> Result<()> {
    println!("Connecting to MIDI destination {}...", destination);
    let mut output = CoreMidiOutput::new(destination)?;

    let channel = 0; // MIDI channel 1
    let note = 60;   // Middle C
    let velocity = 100;

    println!("Sending test note (Middle C, velocity {})...", velocity);

    // Note On
    output.send(&[0x90 | channel, note, velocity])?;
    println!("Note On sent");

    // Hold for 500ms
    thread::sleep(Duration::from_millis(500));

    // Note Off
    output.send(&[0x80 | channel, note, 0])?;
    println!("Note Off sent");

    println!("Test complete!");
    Ok(())
}

fn send_test_clock(destination: usize, bpm: f64) -> Result<()> {
    println!("Connecting to MIDI destination {}...", destination);
    let mut output = CoreMidiOutput::new(destination)?;
    let mut clock = MidiClock::new(bpm);

    println!("Sending MIDI clock at {} BPM for 4 beats (press Ctrl+C to stop)...", bpm);
    println!("PPQN: 24, Pulse interval: {:.2}ms", clock.pulse_interval().as_secs_f64() * 1000.0);

    // Send start message
    let start_msg = clock.start();
    output.send(&start_msg)?;
    println!("START sent");

    let start_time = Instant::now();
    let run_duration = Duration::from_secs_f64(60.0 / bpm * 4.0); // 4 beats
    let mut last_beat = 0u64;

    // Main clock loop
    while start_time.elapsed() < run_duration {
        if let Some(tick_msg) = clock.tick() {
            output.send(&tick_msg)?;

            // Print beat changes
            if clock.beat() != last_beat {
                last_beat = clock.beat();
                println!("Beat {}", last_beat);
            }
        }

        // Small sleep to prevent busy-waiting
        let sleep_time = clock.time_until_next_pulse();
        if sleep_time > Duration::from_micros(100) {
            thread::sleep(sleep_time / 2);
        }
    }

    // Send stop message
    let stop_msg = clock.stop();
    output.send(&stop_msg)?;
    println!("STOP sent");

    println!("Clock test complete! Sent {} beats at {} BPM", last_beat, bpm);
    Ok(())
}

fn monitor_input(source: usize) -> Result<()> {
    println!("Connecting to MIDI source {}...", source);
    let input = MidiInput::new(source)?;

    println!("Monitoring MIDI input (press Ctrl+C to stop)...");
    println!();

    let start_time = Instant::now();
    let run_duration = Duration::from_secs(30); // Run for 30 seconds

    while start_time.elapsed() < run_duration {
        // Check for incoming messages
        for msg in input.recv_all() {
            println!("{:?}", msg);
        }

        // Small sleep to prevent busy-waiting
        thread::sleep(Duration::from_millis(1));
    }

    println!();
    println!("Monitor complete!");
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("SEQ - Algorithmic MIDI Sequencer");
        println!("Run with --help for usage information");
        return Ok(());
    }

    match args[1].as_str() {
        "--list-midi" => {
            print_destinations();
        }
        "--list-sources" => {
            print_sources();
        }
        "--test-note" => {
            if args.len() < 3 {
                eprintln!("Error: --test-note requires a destination number");
                eprintln!("Use --list-midi to see available destinations");
                std::process::exit(1);
            }
            let destination: usize = args[2].parse().map_err(|_| {
                anyhow::anyhow!("Invalid destination number: {}", args[2])
            })?;
            send_test_note(destination)?;
        }
        "--test-clock" => {
            if args.len() < 3 {
                eprintln!("Error: --test-clock requires a destination number");
                eprintln!("Use --list-midi to see available destinations");
                std::process::exit(1);
            }
            let destination: usize = args[2].parse().map_err(|_| {
                anyhow::anyhow!("Invalid destination number: {}", args[2])
            })?;
            let bpm: f64 = if args.len() >= 4 {
                args[3].parse().unwrap_or(120.0)
            } else {
                120.0
            };
            send_test_clock(destination, bpm)?;
        }
        "--monitor" => {
            if args.len() < 3 {
                eprintln!("Error: --monitor requires a source number");
                eprintln!("Use --list-sources to see available sources");
                std::process::exit(1);
            }
            let source: usize = args[2].parse().map_err(|_| {
                anyhow::anyhow!("Invalid source number: {}", args[2])
            })?;
            monitor_input(source)?;
        }
        "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown option: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}
