//! Criterion benchmarks for `ryu-audio-fx` biquad kernels on one 240-sample
//! (5 ms @ 48 kHz) channel: single DF2 filter, filter+mix, cascaded double filter+mix,
//! and a 4-stage multi-biquad chain.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_audio_fx::{
    process_biquad_filter, process_biquad_filter_and_mix, process_double_biquad_filter_and_mix,
    process_multi_biquad_filter, BiquadFilterParameter, BiquadFilterState,
};
use std::time::Duration;

const QUANTUM: usize = 240;
const STAGES: usize = 4;

/// The crate's recurrence *adds* `denominator * y[n-k]`, so a1/a2 are stored negated.
fn param(b: [f32; 3], a1: f32, a2: f32) -> BiquadFilterParameter {
    BiquadFilterParameter {
        enable: 1,
        numerator: b,
        denominator: [-a1, -a2],
        ..Default::default()
    }
}

fn lowpass() -> BiquadFilterParameter {
    param([0.067455, 0.134911, 0.067455], -1.14298, 0.41280)
}

fn highpass() -> BiquadFilterParameter {
    // ~200 Hz high-pass @ 48 kHz, Q = 0.707
    param([0.98166, -1.96333, 0.98166], -1.96297, 0.96368)
}

fn assert_stable(p: &BiquadFilterParameter, input: &[f32]) {
    let mut st = BiquadFilterState::default();
    let mut out = vec![0.0f32; QUANTUM];
    for _ in 0..1000 {
        process_biquad_filter(p, &mut st, &mut out, input, QUANTUM);
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(
        peak.is_finite() && peak < 2.0 * 16384.0,
        "biquad diverged: peak {peak}"
    );
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("audio_fx");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .throughput(Throughput::Elements(QUANTUM as u64));

    let input: Vec<f32> = (0..QUANTUM)
        .map(|i| (i as f32 * 0.17).sin() * 16384.0)
        .collect();
    let lp = lowpass();
    let hp = highpass();
    assert_stable(&lp, &input);
    assert_stable(&hp, &input);
    let mut out = vec![0.0f32; QUANTUM];

    let mut st = BiquadFilterState::default();
    g.bench_function(BenchmarkId::new("biquad_filter", QUANTUM), |b| {
        b.iter(|| {
            process_biquad_filter(
                black_box(&lp),
                &mut st,
                black_box(&mut out),
                black_box(&input),
                QUANTUM,
            );
            black_box(&out);
        })
    });

    let mut st = BiquadFilterState::default();
    g.bench_function(BenchmarkId::new("biquad_filter_and_mix", QUANTUM), |b| {
        b.iter(|| {
            out.fill(0.0);
            process_biquad_filter_and_mix(
                black_box(&lp),
                &mut st,
                black_box(&mut out),
                black_box(&input),
                QUANTUM,
                black_box(0.75),
            );
            black_box(&out);
        })
    });

    let (mut st0, mut st1) = (BiquadFilterState::default(), BiquadFilterState::default());
    g.bench_function(
        BenchmarkId::new("double_biquad_filter_and_mix", QUANTUM),
        |b| {
            b.iter(|| {
                out.fill(0.0);
                process_double_biquad_filter_and_mix(
                    black_box(&lp),
                    black_box(&hp),
                    &mut st0,
                    &mut st1,
                    black_box(&mut out),
                    black_box(&input),
                    QUANTUM,
                    black_box(0.75),
                );
                black_box(&out);
            })
        },
    );

    let params = [lp, hp, lp, hp];
    let mut states = [BiquadFilterState::default(); STAGES];
    g.bench_function(BenchmarkId::new("multi_biquad_4stage", QUANTUM), |b| {
        b.iter(|| {
            process_multi_biquad_filter(
                black_box(&params),
                &mut states,
                black_box(&mut out),
                black_box(&input),
                QUANTUM,
            );
            black_box(&out);
        })
    });

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
