//! Benchmarks for `ryu-memory-pagetable`: `Radix48PageTable` lookups (`read`, `read_mut`)
//! over a 256 MiB mapped guest range with seeded random and sequential access patterns,
//! and bulk population via `map_range`.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_memory_pagetable::{Radix48PageTable, PAGE_SIZE};
use std::hint::black_box;
use std::time::Duration;

const GUEST_BASE: u64 = 0x0000_0080_0000_0000;
const HOST_BASE: u64 = 0x0000_7f00_0000_0000;
const MAPPED_BYTES: u64 = 256 << 20; // 256 MiB = 65536 pages
const LOOKUPS: usize = 4096;
const MAP_RANGE_BYTES: u64 = 16 << 20; // 16 MiB = 4096 pages

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

fn build_table() -> Radix48PageTable {
    let mut pt = Radix48PageTable::new();
    pt.map_range(GUEST_BASE, MAPPED_BYTES, HOST_BASE);
    pt
}

fn random_vas(count: usize, seed: u64) -> Vec<u64> {
    let pages = MAPPED_BYTES / PAGE_SIZE as u64;
    let mut rng = XorShift64(seed);
    (0..count)
        .map(|_| {
            let r = rng.next();
            GUEST_BASE + (r % pages) * PAGE_SIZE as u64 + ((r >> 40) & 0xfff)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pagetable");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let mut pt = build_table();
    let vas = random_vas(LOOKUPS, 0x9e37_79b9_7f4a_7c15);
    // Precondition: every looked-up address is mapped to its expected host page.
    for &va in &vas {
        let page_off = (va - GUEST_BASE) & !(PAGE_SIZE as u64 - 1);
        assert_eq!(pt.read(va), HOST_BASE + page_off);
    }
    let seq_vas: Vec<u64> = (0..LOOKUPS as u64)
        .map(|i| GUEST_BASE + i * PAGE_SIZE as u64)
        .collect();

    group.throughput(Throughput::Elements(LOOKUPS as u64));
    group.bench_function(BenchmarkId::new("read", "random_256MiB"), |b| {
        b.iter(|| {
            let pt = black_box(&pt);
            let mut acc = 0u64;
            for &va in black_box(vas.as_slice()) {
                acc = acc.wrapping_add(pt.read(va));
            }
            black_box(acc)
        })
    });

    group.bench_function(BenchmarkId::new("read_mut", "random_256MiB"), |b| {
        b.iter(|| {
            let pt = black_box(&mut pt);
            let mut acc = 0u64;
            for &va in black_box(vas.as_slice()) {
                acc = acc.wrapping_add(pt.read_mut(va));
            }
            black_box(acc)
        })
    });

    group.bench_function(BenchmarkId::new("read", "sequential_16MiB"), |b| {
        b.iter(|| {
            let pt = black_box(&pt);
            let mut acc = 0u64;
            for &va in black_box(seq_vas.as_slice()) {
                acc = acc.wrapping_add(pt.read(va));
            }
            black_box(acc)
        })
    });

    let map_pages = MAP_RANGE_BYTES / PAGE_SIZE as u64;
    group.throughput(Throughput::Elements(map_pages));
    group.bench_function(BenchmarkId::new("map_range", "16MiB_fresh_table"), |b| {
        b.iter_batched(
            Radix48PageTable::new,
            |mut t| {
                t.map_range(
                    black_box(GUEST_BASE),
                    black_box(MAP_RANGE_BYTES),
                    black_box(HOST_BASE),
                );
                // Returned so the table is dropped outside the timed section.
                black_box(t)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
