//! Benchmarks `ryu_host1x_syncpoint::SyncptSequencer`: immediate syncpoint
//! increments with drain, and a frame of deferred increments that complete out
//! of order (`increment_when_done` + reverse-order `signal_done` + drain).

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_host1x_syncpoint::SyncptSequencer;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("host1x_syncpoint");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let seq = SyncptSequencer::new();
    let mut drain = vec![0u32; 256];

    // Immediate increments, drained once per batch so the flushed queue stays bounded.
    const INC_BATCH: usize = 64;
    for i in 0..INC_BATCH {
        seq.increment((i % 32) as u32);
    }
    assert_eq!(seq.drain_flushed(&mut drain), INC_BATCH);
    assert_eq!(drain[5], 5);

    group.throughput(Throughput::Elements(INC_BATCH as u64));
    group.bench_function(BenchmarkId::new("increment_drain", INC_BATCH), |b| {
        b.iter(|| {
            for i in 0..INC_BATCH as u32 {
                seq.increment(black_box(i % 32));
            }
            black_box(seq.drain_flushed(black_box(&mut drain)))
        });
    });

    // Deferred increments completed in reverse order; nothing flushes until the
    // oldest one is signalled, then the whole frame flushes in submit order.
    for &per_frame in &[32usize, 128] {
        let mut handles = Vec::with_capacity(per_frame);

        seq.reset();
        for s in 0..per_frame {
            handles.push(seq.increment_when_done((s % 5) as u32, (s % 32) as u32));
        }
        for &h in handles.iter().rev().take(per_frame - 1) {
            seq.signal_done(h);
        }
        assert_eq!(seq.flushed_count(), 0);
        seq.signal_done(handles[0]);
        assert_eq!(seq.pending_count(), 0);
        assert_eq!(seq.drain_flushed(&mut drain), per_frame);
        assert_eq!(drain[per_frame - 1], ((per_frame - 1) % 32) as u32);
        seq.reset();

        group.throughput(Throughput::Elements(per_frame as u64));
        group.bench_function(BenchmarkId::new("frame_out_of_order", per_frame), |b| {
            b.iter(|| {
                handles.clear();
                for s in 0..per_frame as u32 {
                    handles.push(seq.increment_when_done(black_box(s % 5), black_box(s % 32)));
                }
                for &h in handles.iter().rev() {
                    seq.signal_done(h);
                }
                black_box(seq.drain_flushed(black_box(&mut drain)))
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
