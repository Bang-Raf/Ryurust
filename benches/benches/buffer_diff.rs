//! Benchmarks `ryu_buffer_diff`: dirty-range detection between a CPU-side
//! buffer and its shadow copy (`find_dirty_range`), fast equality
//! (`is_equal_fast`), and find-and-copy of the dirty range (`diff_and_copy`).

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use ryu_buffer_diff::{diff_and_copy, find_dirty_range, is_equal_fast, DirtyRange};

/// `len`-byte buffer pair with a 64-byte (one matrix) modification at `len / 4`.
fn pair(len: usize) -> (Vec<u8>, Vec<u8>, DirtyRange) {
    let dst: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(13)).collect();
    let mut src = dst.clone();
    let off = len / 4;
    for b in &mut src[off..off + 64] {
        *b = !*b;
    }
    (src, dst, DirtyRange::new(off, 64))
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_diff");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // Forward + backward scan over a sparse-dirty buffer (4 KiB UBO, 64 KiB buffer).
    for &len in &[4 * 1024usize, 64 * 1024] {
        let (src, dst, expected) = pair(len);
        assert_eq!(find_dirty_range(&src, &dst), Some(expected));

        group.throughput(Throughput::Bytes(len as u64));
        group.bench_with_input(
            BenchmarkId::new("find_dirty_range", len),
            &(src, dst),
            |b, (src, dst)| {
                b.iter(|| black_box(find_dirty_range(black_box(src), black_box(dst))));
            },
        );
    }

    // Equality of two identical 4 KiB buffers (full scan, no early exit).
    let a: Vec<u8> = (0..4096).map(|i| (i as u8).wrapping_mul(13)).collect();
    let b2 = a.clone();
    assert!(is_equal_fast(&a, &b2));
    group.throughput(Throughput::Bytes(a.len() as u64));
    group.bench_function(BenchmarkId::new("is_equal_fast", a.len()), |b| {
        b.iter(|| black_box(is_equal_fast(black_box(&a), black_box(&b2))));
    });

    // Diff + copy into a fresh stale shadow each iteration (setup clone is not timed).
    let len = 64 * 1024;
    let (src, dst, expected) = pair(len);
    {
        let mut d = dst.clone();
        assert_eq!(diff_and_copy(&src, &mut d), Some(expected));
        assert_eq!(d, src);
    }
    group.throughput(Throughput::Bytes(len as u64));
    group.bench_function(BenchmarkId::new("diff_and_copy", len), |b| {
        b.iter_batched_ref(
            || dst.clone(),
            |d| black_box(diff_and_copy(black_box(&src), d)),
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
