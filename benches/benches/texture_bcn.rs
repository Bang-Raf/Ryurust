//! Criterion benchmarks for `ryu-texture-bcn`: whole-surface decode of a 1920x1080
//! BC1, BC3, BC6H (unsigned) and BC7 texture built from deterministic pseudo-random
//! blocks. Throughput is decoded output bytes.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_texture_bcn::{
    decode_bc1_surface, decode_bc3_surface, decode_bc6h_surface, decode_bc7_surface,
};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const BLOCKS: usize = (WIDTH / 4) * (HEIGHT / 4);

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

type SurfaceFn = fn(&[u8], &mut [u8], usize, usize) -> Result<(), &'static str>;

fn decode_bc6h_unsigned(
    src: &[u8],
    dst: &mut [u8],
    w: usize,
    h: usize,
) -> Result<(), &'static str> {
    decode_bc6h_surface(src, dst, w, h, false)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_bcn");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // (id, bytes per block, output bytes per pixel, decoder)
    let cases: [(&str, usize, usize, SurfaceFn); 4] = [
        ("bc1", 8, 4, decode_bc1_surface),
        ("bc3", 16, 4, decode_bc3_surface),
        ("bc6h_ufloat", 16, 8, decode_bc6h_unsigned),
        ("bc7", 16, 4, decode_bc7_surface),
    ];

    for (i, &(id, block_bytes, out_bpp, decode)) in cases.iter().enumerate() {
        let src = pseudo_random_bytes(BLOCKS * block_bytes, 0xBC00 + i as u64);
        let out_len = WIDTH * HEIGHT * out_bpp;
        let mut dst = vec![0u8; out_len];
        decode(&src, &mut dst, WIDTH, HEIGHT).expect("decode precondition");

        group.throughput(Throughput::Bytes(out_len as u64));
        group.bench_with_input(BenchmarkId::new(id, "1920x1080"), &src, |b, src| {
            b.iter(|| {
                let r = decode(black_box(src), black_box(&mut dst), WIDTH, HEIGHT);
                black_box(r).ok();
                black_box(&dst);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
