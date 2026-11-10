//! Benchmarks `ryu_interval_tree::FlatIntervalTree`: batches of overlap queries
//! (`find_overlaps` into a Vec, `find_overlaps_slice` into a fixed buffer)
//! against cache-sized interval sets, and building a tree by unsorted insertion.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_interval_tree::FlatIntervalTree;

const QUERIES: usize = 64;
const STRIDE: u64 = 0x1000;
const SPAN: u64 = 0x4000;

fn build(n: u64) -> FlatIntervalTree {
    let mut t = FlatIntervalTree::with_capacity(n as usize);
    for i in 0..n {
        t.insert(i * STRIDE, i * STRIDE + SPAN, i);
    }
    t
}

/// Deterministic query ranges spread over the whole address range of `n` intervals.
fn queries(n: u64) -> Vec<(u64, u64)> {
    (0..QUERIES as u64)
        .map(|q| {
            let start = (q.wrapping_mul(2_654_435_761) % n) * STRIDE + 0x200;
            (start, start + 5 * STRIDE)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("interval_tree");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    for &n in &[128u64, 1024] {
        let tree = build(n);
        let qs = queries(n);

        // Vec and slice variants must agree, and match a brute-force count.
        let mut out = Vec::with_capacity(32);
        let mut buf = [0u64; 32];
        for &(s, e) in &qs {
            out.clear();
            tree.find_overlaps(s, e, &mut out);
            let brute = (0..n)
                .filter(|&i| i * STRIDE < e && s < i * STRIDE + SPAN)
                .count();
            assert_eq!(out.len(), brute);
            assert_eq!(tree.find_overlaps_slice(s, e, &mut buf), brute);
            assert_eq!(&buf[..brute], &out[..]);
        }

        group.throughput(Throughput::Elements(QUERIES as u64));
        group.bench_with_input(BenchmarkId::new("find_overlaps", n), &qs, |b, qs| {
            b.iter(|| {
                let mut hits = 0usize;
                for &(s, e) in qs {
                    out.clear();
                    tree.find_overlaps(black_box(s), black_box(e), &mut out);
                    hits += out.len();
                }
                black_box(hits)
            });
        });
        group.bench_with_input(BenchmarkId::new("find_overlaps_slice", n), &qs, |b, qs| {
            b.iter(|| {
                let mut hits = 0usize;
                for &(s, e) in qs {
                    hits +=
                        tree.find_overlaps_slice(black_box(s), black_box(e), black_box(&mut buf));
                }
                black_box(hits)
            });
        });
    }

    // Build a 1024-entry tree from a deterministic shuffled insertion order.
    let n = 1024u64;
    let order: Vec<u64> = (0..n).map(|i| (i * 389) % n).collect(); // 389 is coprime with 1024
    group.throughput(Throughput::Elements(n));
    group.bench_with_input(
        BenchmarkId::new("insert_shuffled", n),
        &order,
        |b, order| {
            b.iter(|| {
                let mut t = FlatIntervalTree::with_capacity(n as usize);
                for &i in order {
                    t.insert(black_box(i * STRIDE), i * STRIDE + SPAN, i);
                }
                black_box(t)
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
