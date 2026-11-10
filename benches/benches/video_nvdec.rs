//! Criterion benchmarks for `ryu-video-nvdec`.
//!
//! The crate does not decode real VP9 bitstreams. What exists is: the VP9 inverse
//! transforms (IDCT/IADST + add-to-prediction), the VP9 8-tap deblocking loop filter, and a
//! synthetic `Vp9DecoderPipeline::decode_frame` that reads one bool-coded skip/has-coeff
//! flag and an 8-bit DC value per 16x16 block, applies DC-only 8x8 IDCTs, then runs the
//! loop filter. Benchmarks are named after those operations:
//! full 8x8 and 32x32 IDCT-add, loop filter over a 1080p I420 frame, and the synthetic
//! DC-only reconstruction of a 1080p frame.

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use ryu_video_nvdec::{
    idct32x32_1024_add, idct8x8_64_add, loop_filter_frame, Vp9DecoderPipeline, Vp9FrameConfig,
};
use std::time::Duration;

const W: usize = 1920;
const H: usize = 1080;
const UV_W: usize = W / 2;
const UV_H: usize = H / 2;

fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

/// Dequantized-looking coefficient block: large low-frequency terms decaying with frequency.
fn coeff_block(n: usize, seed: u32) -> Vec<i32> {
    let mut next = rng(seed);
    (0..n * n)
        .map(|k| {
            let (r, c) = (k / n, k % n);
            let mag = 600 / (1 + r + c) as i32;
            (next() % (2 * mag as u32 + 1)) as i32 - mag
        })
        .collect()
}

/// Plane made of 8x8 blocks with a per-block base level plus mild noise, so block edges
/// are low-contrast steps the deblocking filter actually modifies.
fn blocky_plane(w: usize, h: usize, seed: u32) -> Vec<u8> {
    let mut next = rng(seed);
    let (bw, bh) = (w.div_ceil(8), h.div_ceil(8));
    let bases: Vec<u8> = (0..bw * bh).map(|_| 100 + (next() % 24) as u8).collect();
    let mut p = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let base = bases[(y / 8) * bw + x / 8];
            p[y * w + x] = base + (next() % 3) as u8;
        }
    }
    p
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("video_nvdec");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // --- Inverse transforms: add residual into a mid-grey prediction block ---
    // (the block is reset to 128 each iteration; a 64 B / 1 KiB fill is included in timing)
    let c8 = coeff_block(8, 0xC0FFEE);
    let c32 = coeff_block(32, 0xBEEF);
    let mut blk8 = [128u8; 64];
    let mut blk32 = [128u8; 1024];

    g.throughput(Throughput::Elements(64));
    g.bench_function(BenchmarkId::new("idct8x8_full_add", "64px"), |b| {
        b.iter(|| {
            blk8.fill(128);
            idct8x8_64_add(black_box(&c8), black_box(&mut blk8), 8);
            black_box(&blk8);
        })
    });

    g.throughput(Throughput::Elements(1024));
    g.bench_function(BenchmarkId::new("idct32x32_full_add", "1024px"), |b| {
        b.iter(|| {
            blk32.fill(128);
            idct32x32_1024_add(black_box(&c32), black_box(&mut blk32), 32);
            black_box(&blk32);
        })
    });

    // --- Deblocking loop filter over a full 1080p I420 frame ---
    let y0 = blocky_plane(W, H, 1);
    let u0 = blocky_plane(UV_W, UV_H, 2);
    let v0 = blocky_plane(UV_W, UV_H, 3);
    let (level, sharpness) = (32u8, 0u8);
    {
        let (mut y, mut u, mut v) = (y0.clone(), u0.clone(), v0.clone());
        loop_filter_frame(&mut y, &mut u, &mut v, W, H, W, UV_W, level, sharpness);
        assert!(
            y != y0 && u != u0,
            "loop filter did not modify the test frame"
        );
    }
    let frame_bytes = (W * H + 2 * UV_W * UV_H) as u64;

    g.throughput(Throughput::Bytes(frame_bytes));
    g.bench_function(
        BenchmarkId::new("loop_filter_frame", "1920x1080_i420"),
        |b| {
            b.iter_batched_ref(
                || (y0.clone(), u0.clone(), v0.clone()),
                |(y, u, v)| {
                    loop_filter_frame(
                        black_box(y),
                        black_box(u),
                        black_box(v),
                        W,
                        H,
                        W,
                        UV_W,
                        black_box(level),
                        sharpness,
                    );
                    black_box((&y, &u, &v));
                },
                BatchSize::LargeInput,
            )
        },
    );

    // --- Synthetic DC-only reconstruction + loop filter (not a VP9 bitstream decode) ---
    let mut next = rng(0x5EED);
    let bitstream: Vec<u8> = (0..64 * 1024).map(|_| next() as u8).collect();
    let config = Vp9FrameConfig {
        width: W as u32,
        height: H as u32,
        is_key_frame: 1,
        intra_only: 1,
        filter_level: 12,
        sharpness: 1,
        base_qindex: 80,
        transform_mode: 1,
        lossless: 0,
    };
    let mut y = vec![0u8; W * H];
    let mut u = vec![0u8; UV_W * UV_H];
    let mut v = vec![0u8; UV_W * UV_H];
    Vp9DecoderPipeline::decode_frame(&bitstream, &config, &mut y, &mut u, &mut v, W, UV_W)
        .expect("decode_frame rejected buffers");
    assert!(
        y.iter().any(|&p| p != 128),
        "synthetic reconstruction produced a flat frame"
    );

    g.throughput(Throughput::Bytes(frame_bytes));
    g.bench_function(
        BenchmarkId::new("synthetic_dc_reconstruct_frame", "1920x1080_i420"),
        |b| {
            b.iter(|| {
                let r = Vp9DecoderPipeline::decode_frame(
                    black_box(&bitstream),
                    black_box(&config),
                    black_box(&mut y),
                    black_box(&mut u),
                    black_box(&mut v),
                    W,
                    UV_W,
                );
                black_box(r).ok();
                black_box((&y, &u, &v));
            })
        },
    );

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
