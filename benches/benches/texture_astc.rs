//! Criterion benchmarks for `ryu-texture-astc`: ASTC LDR decode of a 1920x1080
//! single-level 2D texture to RGBA8, sequential and Rayon-parallel, over
//! general (non-void-extent) blocks and over void-extent (constant colour) blocks.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_texture_astc::{
    decode_astc_to_rgba8, decode_astc_to_rgba8_parallel, query_decompressed_size,
};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

fn is_void_extent(block: &[u8; 16]) -> bool {
    (u16::from_le_bytes([block[0], block[1]]) & 0x1FF) == 0x1FC
}

/// Pool of pseudo-random 16-byte blocks that take the general (endpoint + weight)
/// decode path rather than the void-extent shortcut. Blocks the decoder rejects are
/// still included: they decode to the ASTC error colour, as real malformed data would.
fn general_block_pool(bw: usize, bh: usize, pool_size: usize) -> Vec<[u8; 16]> {
    let mut rng = Lcg(0x5EED_A57C ^ ((bw as u64) << 8 | bh as u64));
    let mut pool = Vec::with_capacity(pool_size);
    while pool.len() < pool_size {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&rng.next_u64().to_le_bytes());
        b[8..].copy_from_slice(&rng.next_u64().to_le_bytes());
        if !is_void_extent(&b) {
            pool.push(b);
        }
    }
    pool
}

fn texture_from_pool(pool: &[[u8; 16]], blocks: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(blocks * 16);
    for i in 0..blocks {
        // Stride through the pool so neighbouring blocks differ.
        data.extend_from_slice(&pool[(i * 7919) % pool.len()]);
    }
    data
}

fn void_extent_texture(blocks: usize) -> Vec<u8> {
    let mut data = vec![0u8; blocks * 16];
    for (i, block) in data.chunks_exact_mut(16).enumerate() {
        let v = ((i * 37) & 0xFFFF) as u16;
        block[0] = 0xFC;
        block[1] = 0xFD;
        block[2..8].fill(0xFF);
        block[8..10].copy_from_slice(&v.to_le_bytes());
        block[10..12].copy_from_slice(&v.rotate_left(5).to_le_bytes());
        block[12..14].copy_from_slice(&v.rotate_left(11).to_le_bytes());
        block[14..16].copy_from_slice(&0xFFFFu16.to_le_bytes());
    }
    data
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_astc");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let out_size = query_decompressed_size(WIDTH, HEIGHT, 1, 1, 1);
    assert_eq!(out_size, WIDTH * HEIGHT * 4);
    group.throughput(Throughput::Bytes(out_size as u64));

    let mut output = vec![0u8; out_size];

    for &(bw, bh) in &[(4usize, 4usize), (8, 8)] {
        let blocks = WIDTH.div_ceil(bw) * HEIGHT.div_ceil(bh);
        let pool = general_block_pool(bw, bh, 1024);
        let input = texture_from_pool(&pool, blocks);
        let label = format!("{bw}x{bh}_1920x1080");

        // Sequential and parallel decoders must agree.
        let mut check = vec![0u8; out_size];
        assert!(decode_astc_to_rgba8(
            &input,
            &mut output,
            bw,
            bh,
            WIDTH,
            HEIGHT,
            1,
            1,
            1
        ));
        assert!(decode_astc_to_rgba8_parallel(
            &input, &mut check, bw, bh, WIDTH, HEIGHT, 1, 1, 1
        ));
        assert_eq!(output, check);

        group.bench_with_input(
            BenchmarkId::new("decode_seq", &label),
            &input,
            |b, input| {
                b.iter(|| {
                    let ok = decode_astc_to_rgba8(
                        black_box(input),
                        black_box(&mut output),
                        bw,
                        bh,
                        WIDTH,
                        HEIGHT,
                        1,
                        1,
                        1,
                    );
                    black_box(ok);
                    black_box(&output);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("decode_par", &label),
            &input,
            |b, input| {
                b.iter(|| {
                    let ok = decode_astc_to_rgba8_parallel(
                        black_box(input),
                        black_box(&mut output),
                        bw,
                        bh,
                        WIDTH,
                        HEIGHT,
                        1,
                        1,
                        1,
                    );
                    black_box(ok);
                    black_box(&output);
                })
            },
        );
    }

    let blocks = WIDTH.div_ceil(4) * HEIGHT.div_ceil(4);
    let input = void_extent_texture(blocks);
    assert!(decode_astc_to_rgba8(
        &input,
        &mut output,
        4,
        4,
        WIDTH,
        HEIGHT,
        1,
        1,
        1
    ));
    group.bench_with_input(
        BenchmarkId::new("decode_seq_void_extent", "4x4_1920x1080"),
        &input,
        |b, input| {
            b.iter(|| {
                let ok = decode_astc_to_rgba8(
                    black_box(input),
                    black_box(&mut output),
                    4,
                    4,
                    WIDTH,
                    HEIGHT,
                    1,
                    1,
                    1,
                );
                black_box(ok);
                black_box(&output);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
