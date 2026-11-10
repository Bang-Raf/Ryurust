//! Criterion benchmarks for `ryu-tegra-swizzle`: Tegra block-linear <-> linear pitch
//! conversion of a 1920x1080 RGBA8 surface (GOB height 16 rows, gob_blocks_in_y = 2),
//! sequential and Rayon-parallel. Throughput is linear surface bytes.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_tegra_swizzle::{
    align_up, convert_block_linear_to_linear, convert_block_linear_to_linear_parallel,
    convert_linear_to_block_linear, convert_linear_to_block_linear_parallel,
    query_block_linear_size, HOST_STRIDE_ALIGNMENT,
};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const BPP: usize = 4;
const GOB_Y: usize = 2;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("tegra_swizzle");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let stride = align_up(WIDTH * BPP, HOST_STRIDE_ALIGNMENT);
    let linear_size = stride * HEIGHT;
    let bl_size = query_block_linear_size(WIDTH, HEIGHT, 1, BPP, GOB_Y, 1).unwrap();

    let linear: Vec<u8> = (0..linear_size)
        .map(|i| (i ^ (i >> 8) ^ (i >> 16)) as u8)
        .collect();
    let mut block_linear = vec![0u8; bl_size];
    let mut linear_out = vec![0u8; linear_size];

    // Round-trip and seq/par agreement, checked once outside the timed loops.
    convert_linear_to_block_linear(
        &mut block_linear,
        &linear,
        WIDTH,
        HEIGHT,
        stride,
        BPP,
        GOB_Y,
    )
    .unwrap();
    let mut bl_par = vec![0u8; bl_size];
    convert_linear_to_block_linear_parallel(
        &mut bl_par,
        &linear,
        WIDTH,
        HEIGHT,
        stride,
        BPP,
        GOB_Y,
    )
    .unwrap();
    assert_eq!(block_linear, bl_par);
    convert_block_linear_to_linear(
        &mut linear_out,
        &block_linear,
        WIDTH,
        HEIGHT,
        stride,
        BPP,
        GOB_Y,
    )
    .unwrap();
    assert_eq!(linear, linear_out);
    linear_out.fill(0);
    convert_block_linear_to_linear_parallel(
        &mut linear_out,
        &block_linear,
        WIDTH,
        HEIGHT,
        stride,
        BPP,
        GOB_Y,
    )
    .unwrap();
    assert_eq!(linear, linear_out);
    drop(bl_par);

    group.throughput(Throughput::Bytes(linear_size as u64));
    let label = "rgba8_1920x1080";

    group.bench_with_input(
        BenchmarkId::new("deswizzle_seq", label),
        &block_linear,
        |b, src| {
            b.iter(|| {
                let r = convert_block_linear_to_linear(
                    black_box(&mut linear_out),
                    black_box(src),
                    WIDTH,
                    HEIGHT,
                    stride,
                    BPP,
                    GOB_Y,
                );
                black_box(r).ok();
                black_box(&linear_out);
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("deswizzle_par", label),
        &block_linear,
        |b, src| {
            b.iter(|| {
                let r = convert_block_linear_to_linear_parallel(
                    black_box(&mut linear_out),
                    black_box(src),
                    WIDTH,
                    HEIGHT,
                    stride,
                    BPP,
                    GOB_Y,
                );
                black_box(r).ok();
                black_box(&linear_out);
            })
        },
    );

    let mut bl_out = vec![0u8; bl_size];
    group.bench_with_input(BenchmarkId::new("swizzle_seq", label), &linear, |b, src| {
        b.iter(|| {
            let r = convert_linear_to_block_linear(
                black_box(&mut bl_out),
                black_box(src),
                WIDTH,
                HEIGHT,
                stride,
                BPP,
                GOB_Y,
            );
            black_box(r).ok();
            black_box(&bl_out);
        })
    });

    group.bench_with_input(BenchmarkId::new("swizzle_par", label), &linear, |b, src| {
        b.iter(|| {
            let r = convert_linear_to_block_linear_parallel(
                black_box(&mut bl_out),
                black_box(src),
                WIDTH,
                HEIGHT,
                stride,
                BPP,
                GOB_Y,
            );
            black_box(r).ok();
            black_box(&bl_out);
        })
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
