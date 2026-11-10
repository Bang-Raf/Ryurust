//! Criterion benchmarks for `ryu-audio-mixer`: single-voice mix and ramped mix of one
//! 240-sample (5 ms @ 48 kHz) quantum, a 128-voice batch mix, and a 1-in/6-out
//! (5.1 downmix-style) matrix mix.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_audio_mixer::{mix_batch, mix_buffer, mix_matrix, mix_ramp};
use std::time::Duration;

const QUANTUM: usize = 240;
const VOICES: usize = 128;
const CHANNELS: usize = 6;

fn pcm(phase: f32) -> Vec<f32> {
    (0..QUANTUM)
        .map(|i| (i as f32 * 0.17 + phase).sin() * 16384.0)
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("audio_mixer");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let input = pcm(0.0);
    let mut out = vec![0.0f32; QUANTUM];

    g.throughput(Throughput::Elements(QUANTUM as u64));
    g.bench_function(BenchmarkId::new("mix_buffer", QUANTUM), |b| {
        b.iter(|| {
            out.fill(0.0);
            mix_buffer(black_box(&mut out), black_box(&input), black_box(0.75));
            black_box(&out);
        })
    });

    g.bench_function(BenchmarkId::new("mix_ramp", QUANTUM), |b| {
        b.iter(|| {
            out.fill(0.0);
            let last = mix_ramp(
                black_box(&mut out),
                black_box(&input),
                black_box(0.1),
                black_box(0.9),
            );
            black_box(last);
            black_box(&out);
        })
    });

    let voices: Vec<Vec<f32>> = (0..VOICES).map(|v| pcm(v as f32 * 0.37)).collect();
    let voice_slices: Vec<&[f32]> = voices.iter().map(|v| v.as_slice()).collect();
    let volumes: Vec<f32> = (0..VOICES).map(|v| 0.1 + (v % 8) as f32 * 0.1).collect();
    assert!(volumes.iter().all(|&v| v != 0.0)); // every voice contributes

    g.throughput(Throughput::Elements((VOICES * QUANTUM) as u64));
    g.bench_function(
        BenchmarkId::new("mix_batch", format!("{VOICES}voices_x{QUANTUM}")),
        |b| {
            b.iter(|| {
                out.fill(0.0);
                mix_batch(
                    black_box(&mut out),
                    black_box(&voice_slices),
                    black_box(&volumes),
                );
                black_box(&out);
            })
        },
    );

    let gains = [0.707f32, 0.707, 1.0, 0.5, 0.5, 0.5];
    let mut flat = vec![0.0f32; CHANNELS * QUANTUM];
    g.throughput(Throughput::Elements((CHANNELS * QUANTUM) as u64));
    g.bench_function(
        BenchmarkId::new("mix_matrix", format!("1to{CHANNELS}_x{QUANTUM}")),
        |b| {
            b.iter(|| {
                flat.fill(0.0);
                let mut chunks = flat.chunks_mut(QUANTUM);
                let mut refs: [&mut [f32]; CHANNELS] =
                    std::array::from_fn(|_| chunks.next().unwrap());
                mix_matrix(black_box(&mut refs), black_box(&input), black_box(&gains));
                black_box(&refs);
            })
        },
    );

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
