//! Benchmarks for `ryu-memory-tracking`: `AtomicPageBitmap` tracking a 4 GiB guest address
//! space at 4 KiB granularity (1M bits). Measures random single-bit queries, 2 MiB range
//! writes, scanning sparse dirty bits with `find_next_set`, and `count_set_bits`.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_memory_tracking::AtomicPageBitmap;
use std::hint::black_box;
use std::time::Duration;

const BIT_COUNT: usize = 1 << 20; // 4 GiB / 4 KiB
const QUERIES: usize = 4096;
const RANGE_OPS: usize = 1024;
const RANGE_PAGES: usize = 512; // 2 MiB

struct XorShift64(u64);

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// Bitmap with roughly 1% of pages set at seeded random positions.
fn sparse_bitmap() -> AtomicPageBitmap {
    let bm = AtomicPageBitmap::new(BIT_COUNT, false);
    let mut rng = XorShift64(0x2545_f491_4f6c_dd1d);
    for _ in 0..BIT_COUNT / 100 {
        bm.set((rng.next() as usize) % BIT_COUNT, true);
    }
    bm
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_tracking");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let bm = sparse_bitmap();
    let set_bits = bm.count_set_bits();
    assert!(set_bits > 0 && set_bits <= BIT_COUNT / 100);

    let mut rng = XorShift64(0x9e37_79b9_7f4a_7c15);
    let query_bits: Vec<usize> = (0..QUERIES)
        .map(|_| (rng.next() as usize) % BIT_COUNT)
        .collect();
    let range_starts: Vec<usize> = (0..RANGE_OPS)
        .map(|_| (rng.next() as usize) % (BIT_COUNT - RANGE_PAGES))
        .collect();

    group.throughput(Throughput::Elements(QUERIES as u64));
    group.bench_function(BenchmarkId::new("is_set", "random_1M_bits"), |b| {
        b.iter(|| {
            let bm = black_box(&bm);
            let mut hits = 0usize;
            for &bit in black_box(query_bits.as_slice()) {
                hits += bm.is_set(bit) as usize;
            }
            black_box(hits)
        })
    });

    let range_bm = AtomicPageBitmap::new(BIT_COUNT, false);
    group.throughput(Throughput::Elements((2 * RANGE_OPS * RANGE_PAGES) as u64));
    group.bench_function(
        BenchmarkId::new("set_range", "2MiB_set_then_clear_x1024"),
        |b| {
            b.iter(|| {
                let bm = black_box(&range_bm);
                for &start in black_box(range_starts.as_slice()) {
                    bm.set_range(start, start + RANGE_PAGES - 1, true);
                }
                for &start in black_box(range_starts.as_slice()) {
                    bm.set_range(start, start + RANGE_PAGES - 1, false);
                }
            })
        },
    );

    group.throughput(Throughput::Elements(set_bits as u64));
    group.bench_function(BenchmarkId::new("find_next_set", "scan_1pct_dirty"), |b| {
        b.iter(|| {
            let bm = black_box(&bm);
            let mut found = 0usize;
            let mut cursor = bm.find_first_set();
            while let Some(bit) = cursor {
                found += 1;
                cursor = bm.find_next_set(bit + 1);
            }
            black_box(found)
        })
    });

    group.throughput(Throughput::Elements(BIT_COUNT as u64));
    group.bench_function(BenchmarkId::new("count_set_bits", "1M_bits"), |b| {
        b.iter(|| black_box(black_box(&bm).count_set_bits()))
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
