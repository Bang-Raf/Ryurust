//! Criterion benchmarks for `ryu-common-hash`: xxHash3-128 of a single 4 KiB page
//! and a 1 MiB buffer, and the batch API over 256 x 4 KiB pages (Rayon-parallel when
//! the `parallel` feature is on). Throughput is bytes hashed.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_common_hash::{compute_hash_128, compute_hashes_batch_parallel};

const PAGE: usize = 4 * 1024;
const BATCH_PAGES: usize = 256;

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

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("common_hash");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    for &(label, size) in &[("4KiB", PAGE), ("1MiB", 1024 * 1024)] {
        let data = pseudo_random_bytes(size, 0x4A54 + size as u64);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("xxh3_128", label), &data, |b, data| {
            b.iter(|| black_box(compute_hash_128(black_box(data))))
        });
    }

    let backing = pseudo_random_bytes(PAGE * BATCH_PAGES, 0xBA7C);
    let pages: Vec<&[u8]> = backing
        .as_chunks::<PAGE>()
        .0
        .iter()
        .map(|c| c.as_slice())
        .collect();
    let mut results = vec![(0u64, 0u64); BATCH_PAGES];
    compute_hashes_batch_parallel(&pages, &mut results);
    assert_eq!(results[17], compute_hash_128(pages[17]));

    group.throughput(Throughput::Bytes((PAGE * BATCH_PAGES) as u64));
    group.bench_with_input(
        BenchmarkId::new("batch_parallel", format!("{BATCH_PAGES}x4KiB")),
        &pages,
        |b, pages| {
            b.iter(|| {
                compute_hashes_batch_parallel(black_box(pages), black_box(&mut results));
                black_box(&results);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
