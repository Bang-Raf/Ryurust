//! Criterion benchmarks for `ryu-pixel-converter`: whole-surface transcodes of a
//! 1920x1080 image (16-bit R5G6B5 / R4G4B4A4 -> RGBA8, and BGRA8 -> RGBA8 swap).
//! Throughput is pixels converted.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_pixel_converter::{
    convert_bgra8_to_rgba8, convert_r4g4b4a4_to_r8g8b8a8, convert_r5g6b5_pixel,
    convert_r5g6b5_to_r8g8b8a8,
};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const PIXELS: usize = WIDTH * HEIGHT;

fn pseudo_random_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.extend_from_slice(&state.to_le_bytes());
    }
    out.truncate(len);
    out
}

type ConvFn = fn(&[u8], &mut [u8], usize) -> Result<(), &'static str>;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixel_converter");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Elements(PIXELS as u64));

    // 16 bpp -> 32 bpp. Width 1920 keeps rows 4-byte aligned, so the contiguous path is taken.
    let src16 = pseudo_random_bytes(PIXELS * 2, 0x565);
    let mut dst32 = vec![0u8; PIXELS * 4];

    // Spot-check one pixel against the scalar helper.
    convert_r5g6b5_to_r8g8b8a8(&src16, &mut dst32, WIDTH).unwrap();
    let p0 = u16::from_le_bytes([src16[0], src16[1]]);
    assert_eq!(
        u32::from_le_bytes(dst32[0..4].try_into().unwrap()),
        convert_r5g6b5_pixel(p0)
    );

    let cases: [(&str, ConvFn); 2] = [
        ("r5g6b5_to_rgba8", convert_r5g6b5_to_r8g8b8a8),
        ("r4g4b4a4_to_rgba8", convert_r4g4b4a4_to_r8g8b8a8),
    ];
    for &(id, conv) in &cases {
        group.bench_with_input(BenchmarkId::new(id, "1920x1080"), &src16, |b, src| {
            b.iter(|| {
                let r = conv(black_box(src), black_box(&mut dst32), WIDTH);
                black_box(r).ok();
                black_box(&dst32);
            })
        });
    }

    let src_bgra = pseudo_random_bytes(PIXELS * 4, 0xB6A8);
    convert_bgra8_to_rgba8(&src_bgra, &mut dst32).unwrap();
    group.bench_with_input(
        BenchmarkId::new("bgra8_to_rgba8", "1920x1080"),
        &src_bgra,
        |b, src| {
            b.iter(|| {
                let r = convert_bgra8_to_rgba8(black_box(src), black_box(&mut dst32));
                black_box(r).ok();
                black_box(&dst32);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
