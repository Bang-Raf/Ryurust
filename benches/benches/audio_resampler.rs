//! Criterion benchmarks for `ryu-audio-resampler`: 4-tap and 8-tap polyphase sinc
//! resampling of 16-bit PCM producing one 240-sample (5 ms @ 48 kHz) output quantum from a
//! 32 kHz source, and Nintendo DSP-ADPCM decoding of 300 frames (4200 samples).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_audio_resampler::{
    decode_adpcm, get_adpcm_data_size, resample_default, resample_high, AdpcmLoopContext,
};
use std::time::Duration;

const OUT_SAMPLES: usize = 240;
const IN_RATE: f32 = 32000.0;
const OUT_RATE: f32 = 48000.0;
const ADPCM_SAMPLES: usize = 300 * 14;

/// Deterministic xorshift32.
fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("audio_resampler");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // --- Resampling ---
    let ratio = IN_RATE / OUT_RATE;
    let in_len = (OUT_SAMPLES as f32 * ratio) as usize + 8;
    let input: Vec<i16> = (0..in_len)
        .map(|i| ((i as f32 * 0.113).sin() * 12000.0) as i16)
        .collect();
    let mut out = vec![0.0f32; OUT_SAMPLES];

    g.throughput(Throughput::Elements(OUT_SAMPLES as u64));
    g.bench_function(
        BenchmarkId::new("resample_default_4tap", "32k_to_48k_240"),
        |b| {
            b.iter(|| {
                let mut frac = 0.0f32;
                resample_default(
                    black_box(&mut out),
                    black_box(&input),
                    black_box(ratio),
                    &mut frac,
                    OUT_SAMPLES,
                );
                black_box(frac);
                black_box(&out);
            })
        },
    );

    g.bench_function(
        BenchmarkId::new("resample_high_8tap", "32k_to_48k_240"),
        |b| {
            b.iter(|| {
                let mut frac = 0.0f32;
                resample_high(
                    black_box(&mut out),
                    black_box(&input),
                    black_box(ratio),
                    &mut frac,
                    OUT_SAMPLES,
                );
                black_box(frac);
                black_box(&out);
            })
        },
    );

    // --- ADPCM decode ---
    let coefficients: [i16; 16] = [
        0, 0, 2048, 0, 0, 2048, 1024, 1024, 4096, -2048, 3584, -1536, 3072, -1024, 4608, -2560,
    ];
    let data_size = get_adpcm_data_size(ADPCM_SAMPLES);
    let mut next = rng(0x1234_5678);
    let mut adpcm = vec![0u8; data_size];
    for (frame, chunk) in adpcm.chunks_mut(8).enumerate() {
        // header: coefficient pair index (0..7) << 4 | scale (0..11)
        chunk[0] = (((frame % 8) as u8) << 4) | ((next() % 12) as u8);
        for b in &mut chunk[1..] {
            *b = next() as u8;
        }
    }
    let mut pcm = vec![0i16; ADPCM_SAMPLES];
    let mut ctx = AdpcmLoopContext::default();
    let decoded = decode_adpcm(
        &mut pcm,
        &adpcm,
        0,
        ADPCM_SAMPLES as i32,
        0,
        ADPCM_SAMPLES as i32,
        &coefficients,
        &mut ctx,
    );
    assert_eq!(decoded as usize, ADPCM_SAMPLES);

    g.throughput(Throughput::Elements(ADPCM_SAMPLES as u64));
    g.bench_function(BenchmarkId::new("decode_adpcm", ADPCM_SAMPLES), |b| {
        b.iter(|| {
            let mut ctx = AdpcmLoopContext::default();
            let n = decode_adpcm(
                black_box(&mut pcm),
                black_box(&adpcm),
                0,
                ADPCM_SAMPLES as i32,
                0,
                black_box(ADPCM_SAMPLES as i32),
                black_box(&coefficients),
                &mut ctx,
            );
            black_box(n);
            black_box(&pcm);
        })
    });

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
