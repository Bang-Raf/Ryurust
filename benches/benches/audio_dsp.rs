//! Criterion benchmarks for `ryu-audio-dsp`: per-quantum mix and biquad kernels on one
//! 240-sample (5 ms @ 48 kHz) channel, and a 61-command batch over 64 mix buffers.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_audio_dsp::{
    dsp, process_biquad_filter_and_mix_df1, process_biquad_filter_df2, process_command_batch,
    BiquadFilterParameter, BiquadFilterState, DspCommandEntry, DspOpcode,
};
use std::time::Duration;

const QUANTUM: usize = 240;
const BUFFER_COUNT: usize = 64;
const SOURCE_BUFFERS: usize = 10;
const MIX_BUS: usize = 63;

fn pcm_quantum(phase: f32) -> Vec<f32> {
    (0..QUANTUM)
        .map(|i| ((i as f32 * 0.17 + phase).sin()) * 16384.0)
        .collect()
}

/// Second-order low-pass (fc ~ 4.4 kHz @ 48 kHz). The crate's recurrence *adds*
/// `denominator * y[n-k]`, so the denominator holds the negated a1/a2.
fn lowpass() -> BiquadFilterParameter {
    BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.067455, 0.134911, 0.067455],
        denominator: [1.14298, -0.41280],
    }
}

fn assert_filter_stable(param: &BiquadFilterParameter, input: &[f32]) {
    let mut state = BiquadFilterState::default();
    let mut out = vec![0.0f32; QUANTUM];
    for _ in 0..1000 {
        process_biquad_filter_df2(param, &mut state, &mut out, input);
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(
        peak.is_finite() && peak < 2.0 * 16384.0,
        "biquad diverged: peak {peak}"
    );
}

/// Builds a 61-command quantum (1 bus clear + 6 commands per source). Sources (0..10) are only read, so every iteration does
/// identical work; outputs are distinct from inputs within each command.
fn build_batch(param: BiquadFilterParameter) -> Vec<DspCommandEntry> {
    let mut cmds = vec![DspCommandEntry {
        opcode: DspOpcode::ClearBuffer as u32,
        output_buffer_index: MIX_BUS as u16,
        ..Default::default()
    }];
    for i in 0..SOURCE_BUFFERS {
        let src = i as u16;
        let a = (SOURCE_BUFFERS + i * 2) as u16;
        let b = a + 1;
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::CopyBuffer as u32,
            input_buffer_index: src,
            output_buffer_index: a,
            ..Default::default()
        });
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::Volume as u32,
            input_buffer_index: src,
            output_buffer_index: a,
            volume0: 0.85,
            ..Default::default()
        });
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::VolumeRamp as u32,
            input_buffer_index: a,
            output_buffer_index: b,
            volume0: 0.2,
            volume1: 0.9,
            ..Default::default()
        });
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::BiquadFilter as u32,
            input_buffer_index: b,
            output_buffer_index: a,
            filter_param: param,
            state_index: i as u32,
            ..Default::default()
        });
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::Mix as u32,
            input_buffer_index: a,
            output_buffer_index: MIX_BUS as u16,
            volume0: 0.5,
            ..Default::default()
        });
        cmds.push(DspCommandEntry {
            opcode: DspOpcode::MixRamp as u32,
            input_buffer_index: b,
            output_buffer_index: MIX_BUS as u16,
            volume0: 0.1,
            volume1: 0.3,
            last_sample_index: i as u32,
            ..Default::default()
        });
    }
    cmds
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("audio_dsp");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let input = pcm_quantum(0.0);
    let param = lowpass();
    assert_filter_stable(&param, &input);

    g.throughput(Throughput::Elements(QUANTUM as u64));

    let mut out = vec![0.0f32; QUANTUM];
    g.bench_function(BenchmarkId::new("mix_ramp", QUANTUM), |b| {
        b.iter(|| {
            out.fill(0.0);
            let last = dsp::mix_ramp(
                black_box(&mut out),
                black_box(&input),
                black_box(0.1),
                black_box(0.9),
            );
            black_box(last);
            black_box(&out);
        })
    });

    let mut state = BiquadFilterState::default();
    g.bench_function(BenchmarkId::new("biquad_df2", QUANTUM), |b| {
        b.iter(|| {
            process_biquad_filter_df2(
                black_box(&param),
                &mut state,
                black_box(&mut out),
                black_box(&input),
            );
            black_box(&out);
        })
    });

    let mut state = BiquadFilterState::default();
    g.bench_function(BenchmarkId::new("biquad_and_mix_df1", QUANTUM), |b| {
        b.iter(|| {
            out.fill(0.0);
            process_biquad_filter_and_mix_df1(
                black_box(&param),
                &mut state,
                black_box(&mut out),
                black_box(&input),
                black_box(0.75),
            );
            black_box(&out);
        })
    });

    // Full quantum batch.
    let commands = build_batch(param);
    assert_eq!(commands.len(), 61);
    let mut buffers = vec![0.0f32; BUFFER_COUNT * QUANTUM];
    for i in 0..SOURCE_BUFFERS {
        buffers[i * QUANTUM..(i + 1) * QUANTUM].copy_from_slice(&pcm_quantum(i as f32));
    }
    let mut states = vec![BiquadFilterState::default(); SOURCE_BUFFERS];
    let mut last_samples = vec![0.0f32; SOURCE_BUFFERS];
    let executed = process_command_batch(
        &mut buffers,
        BUFFER_COUNT,
        QUANTUM,
        &commands,
        &mut states,
        &mut last_samples,
    )
    .expect("batch rejected");
    assert_eq!(executed, commands.len());

    g.throughput(Throughput::Elements((commands.len() * QUANTUM) as u64));
    g.bench_function(BenchmarkId::new("command_batch", "61cmd_64x240"), |b| {
        b.iter(|| {
            let r = process_command_batch(
                black_box(&mut buffers),
                BUFFER_COUNT,
                QUANTUM,
                black_box(&commands),
                &mut states,
                &mut last_samples,
            );
            black_box(r).ok();
            black_box(&buffers);
        })
    });

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
