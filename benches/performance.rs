// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Performance benchmarks for SEQ
//!
//! Run with: cargo bench
//!
//! These benchmarks measure:
//! - Timing accuracy and jitter
//! - Event processing throughput
//! - Generator performance
//! - Memory allocation patterns

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::time::{Duration, Instant};

/// Benchmark tick-to-microsecond conversion (core timing operation)
fn bench_timing_conversion(c: &mut Criterion) {
    let ppqn = 24u32;
    let tempo = 120.0f64;

    c.bench_function("tick_to_micros", |b| {
        b.iter(|| {
            let micros_per_beat = (60_000_000.0 / black_box(tempo)) as u64;
            let micros_per_tick = micros_per_beat / black_box(ppqn) as u64;
            black_box(micros_per_tick * 1000)
        })
    });
}

/// Benchmark event queue operations (scheduler core)
fn bench_event_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_queue");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, &size| {
            b.iter(|| {
                let mut queue: BinaryHeap<Reverse<(u64, u8)>> = BinaryHeap::new();
                for i in 0..size {
                    queue.push(Reverse((i as u64 * 10, 60)));
                }
                black_box(queue.len())
            })
        });

        group.bench_with_input(BenchmarkId::new("drain", size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut queue: BinaryHeap<Reverse<(u64, u8)>> = BinaryHeap::new();
                    for i in 0..size {
                        queue.push(Reverse((i as u64 * 10, 60)));
                    }
                    queue
                },
                |mut queue| {
                    let mut count = 0;
                    while let Some(_) = queue.pop() {
                        count += 1;
                    }
                    black_box(count)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Benchmark variable-length quantity encoding (MIDI file core)
fn bench_vlq_encoding(c: &mut Criterion) {
    fn encode_vlq(mut value: u32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);
        bytes.push((value & 0x7F) as u8);
        value >>= 7;
        while value > 0 {
            bytes.push((value & 0x7F) as u8 | 0x80);
            value >>= 7;
        }
        bytes.reverse();
        bytes
    }

    let mut group = c.benchmark_group("vlq_encoding");

    for value in [0u32, 127, 128, 16383, 2097151].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(value), value, |b, &value| {
            b.iter(|| encode_vlq(black_box(value)))
        });
    }

    group.finish();
}

/// Benchmark euclidean rhythm generation
fn bench_euclidean(c: &mut Criterion) {
    fn euclidean_rhythm(steps: usize, pulses: usize) -> Vec<bool> {
        if pulses == 0 || steps == 0 {
            return vec![false; steps];
        }
        if pulses >= steps {
            return vec![true; steps];
        }

        let mut pattern = vec![vec![true]; pulses];
        let mut remainder = vec![vec![false]; steps - pulses];

        while remainder.len() > 1 {
            let min_len = pattern.len().min(remainder.len());
            for i in 0..min_len {
                pattern[i].extend(remainder[i].clone());
            }
            let new_remainder = if pattern.len() > min_len {
                pattern.drain(min_len..).collect()
            } else {
                remainder.drain(min_len..).collect()
            };
            remainder = new_remainder;
        }

        pattern.extend(remainder);
        pattern.into_iter().flatten().collect()
    }

    let mut group = c.benchmark_group("euclidean");

    let patterns = [(8, 3), (16, 5), (32, 7), (64, 11)];
    for (steps, pulses) in patterns.iter() {
        group.bench_with_input(
            BenchmarkId::new("pattern", format!("{}/{}", pulses, steps)),
            &(*steps, *pulses),
            |b, &(steps, pulses)| {
                b.iter(|| euclidean_rhythm(black_box(steps), black_box(pulses)))
            },
        );
    }

    group.finish();
}

/// Benchmark scale quantization
fn bench_scale_quantization(c: &mut Criterion) {
    let c_major = [0, 2, 4, 5, 7, 9, 11];

    fn quantize_to_scale(note: u8, scale: &[i32]) -> u8 {
        let pc = (note % 12) as i32;
        let octave = note / 12;

        let quantized_pc = scale
            .iter()
            .min_by_key(|&&interval| (interval - pc).abs())
            .copied()
            .unwrap_or(pc) as u8;

        octave * 12 + quantized_pc
    }

    c.bench_function("scale_quantize", |b| {
        b.iter(|| {
            let mut total = 0u32;
            for note in 36..=96 {
                total += quantize_to_scale(black_box(note), &c_major) as u32;
            }
            black_box(total)
        })
    });
}

/// Benchmark note event processing
fn bench_note_processing(c: &mut Criterion) {
    #[derive(Clone)]
    struct NoteEvent {
        tick: u64,
        channel: u8,
        note: u8,
        velocity: u8,
        duration: u64,
    }

    fn process_notes(events: &[NoteEvent], transpose: i8, velocity_scale: f32) -> Vec<(u64, [u8; 3])> {
        let mut output = Vec::with_capacity(events.len() * 2);

        for event in events {
            let note = (event.note as i16 + transpose as i16).clamp(0, 127) as u8;
            let vel = ((event.velocity as f32 * velocity_scale) as u8).min(127);

            // Note on
            output.push((event.tick, [0x90 | event.channel, note, vel]));
            // Note off
            output.push((event.tick + event.duration, [0x80 | event.channel, note, 0]));
        }

        output.sort_by_key(|(tick, _)| *tick);
        output
    }

    let mut group = c.benchmark_group("note_processing");

    for count in [10, 100, 1000].iter() {
        let events: Vec<NoteEvent> = (0..*count)
            .map(|i| NoteEvent {
                tick: i as u64 * 24,
                channel: (i % 16) as u8,
                note: 60 + (i % 24) as u8,
                velocity: 80 + (i % 48) as u8,
                duration: 12,
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("process", count), &events, |b, events| {
            b.iter(|| process_notes(black_box(events), 0, 1.0))
        });
    }

    group.finish();
}

/// Benchmark timing jitter measurement simulation
fn bench_timing_jitter(c: &mut Criterion) {
    // Simulate measuring timing accuracy
    fn measure_jitter(iterations: u32) -> (f64, f64) {
        let mut deltas = Vec::with_capacity(iterations as usize);
        let target_interval = Duration::from_micros(20833); // ~24 PPQN at 120 BPM

        let mut last = Instant::now();
        for _ in 0..iterations {
            // Simulate some work
            std::hint::black_box(0..100).for_each(|_| {});

            let now = Instant::now();
            let actual = now.duration_since(last);
            let delta = if actual > target_interval {
                (actual - target_interval).as_nanos() as f64
            } else {
                -((target_interval - actual).as_nanos() as f64)
            };
            deltas.push(delta);
            last = now;
        }

        // Calculate mean and std dev
        let mean = deltas.iter().sum::<f64>() / deltas.len() as f64;
        let variance = deltas.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / deltas.len() as f64;
        let std_dev = variance.sqrt();

        (mean, std_dev)
    }

    c.bench_function("jitter_measurement", |b| {
        b.iter(|| measure_jitter(black_box(100)))
    });
}

/// Benchmark memory allocation patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");

    // Preallocated vs dynamic
    group.bench_function("preallocated_vec", |b| {
        b.iter(|| {
            let mut v: Vec<u64> = Vec::with_capacity(1000);
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v.len())
        })
    });

    group.bench_function("dynamic_vec", |b| {
        b.iter(|| {
            let mut v: Vec<u64> = Vec::new();
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v.len())
        })
    });

    // Reuse vs recreate
    group.bench_function("buffer_reuse", |b| {
        let mut buffer: Vec<u64> = Vec::with_capacity(1000);
        b.iter(|| {
            buffer.clear();
            for i in 0..1000 {
                buffer.push(black_box(i));
            }
            black_box(buffer.len())
        })
    });

    group.finish();
}

/// Benchmark position calculation
fn bench_position_calc(c: &mut Criterion) {
    fn tick_to_position(tick: u64, ppqn: u32, beats_per_bar: u32) -> (u64, u32, u32) {
        let ticks_per_bar = ppqn as u64 * beats_per_bar as u64;
        let bar = tick / ticks_per_bar;
        let remaining = tick % ticks_per_bar;
        let beat = (remaining / ppqn as u64) as u32;
        let tick_in_beat = (remaining % ppqn as u64) as u32;
        (bar, beat, tick_in_beat)
    }

    c.bench_function("position_calc", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for tick in (0..10000).step_by(24) {
                let (bar, beat, t) = tick_to_position(black_box(tick), 24, 4);
                sum += bar + beat as u64 + t as u64;
            }
            black_box(sum)
        })
    });
}

/// Benchmark MIDI message parsing
fn bench_midi_parsing(c: &mut Criterion) {
    #[derive(Debug)]
    enum MidiMsg {
        NoteOn(u8, u8, u8),
        NoteOff(u8, u8, u8),
        CC(u8, u8, u8),
        ProgramChange(u8, u8),
        PitchBend(u8, i16),
        Other,
    }

    fn parse_midi(data: &[u8]) -> MidiMsg {
        if data.is_empty() {
            return MidiMsg::Other;
        }

        let status = data[0] & 0xF0;
        let channel = data[0] & 0x0F;

        match status {
            0x90 if data.len() >= 3 && data[2] > 0 => MidiMsg::NoteOn(channel, data[1], data[2]),
            0x80 | 0x90 if data.len() >= 3 => MidiMsg::NoteOff(channel, data[1], data[2]),
            0xB0 if data.len() >= 3 => MidiMsg::CC(channel, data[1], data[2]),
            0xC0 if data.len() >= 2 => MidiMsg::ProgramChange(channel, data[1]),
            0xE0 if data.len() >= 3 => {
                let value = ((data[2] as i16) << 7) | (data[1] as i16) - 8192;
                MidiMsg::PitchBend(channel, value)
            }
            _ => MidiMsg::Other,
        }
    }

    let messages: Vec<Vec<u8>> = vec![
        vec![0x90, 60, 100],  // Note on
        vec![0x80, 60, 0],    // Note off
        vec![0xB0, 7, 127],   // CC
        vec![0xC0, 10],       // Program change
        vec![0xE0, 0, 64],    // Pitch bend
    ];

    c.bench_function("midi_parsing", |b| {
        b.iter(|| {
            let mut count = 0;
            for _ in 0..1000 {
                for msg in &messages {
                    match parse_midi(black_box(msg)) {
                        MidiMsg::NoteOn(_, _, _) => count += 1,
                        MidiMsg::NoteOff(_, _, _) => count += 1,
                        MidiMsg::CC(_, _, _) => count += 1,
                        MidiMsg::ProgramChange(_, _) => count += 1,
                        MidiMsg::PitchBend(_, _) => count += 1,
                        MidiMsg::Other => {}
                    }
                }
            }
            black_box(count)
        })
    });
}

/// Benchmark quantization
fn bench_quantization(c: &mut Criterion) {
    fn quantize(tick: u64, grid: u64) -> u64 {
        ((tick + grid / 2) / grid) * grid
    }

    fn quantize_with_strength(tick: u64, grid: u64, strength: f64) -> u64 {
        let quantized = quantize(tick, grid);
        let diff = quantized as f64 - tick as f64;
        (tick as f64 + diff * strength) as u64
    }

    let mut group = c.benchmark_group("quantization");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for tick in 0..1000 {
                sum += quantize(black_box(tick), 24);
            }
            black_box(sum)
        })
    });

    group.bench_function("with_strength", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for tick in 0..1000 {
                sum += quantize_with_strength(black_box(tick), 24, 0.5);
            }
            black_box(sum)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_timing_conversion,
    bench_event_queue,
    bench_vlq_encoding,
    bench_euclidean,
    bench_scale_quantization,
    bench_note_processing,
    bench_timing_jitter,
    bench_memory_patterns,
    bench_position_calc,
    bench_midi_parsing,
    bench_quantization,
);

criterion_main!(benches);
