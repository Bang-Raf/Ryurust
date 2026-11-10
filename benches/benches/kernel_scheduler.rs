//! Benchmarks for `ryu-kernel-scheduler`: `RyuScheduler` multi-level priority queues
//! (4 cores x 64 priorities). Measures schedule+unschedule churn, round-robin `reschedule`
//! (yield), scheduled-queue walks via `scheduled_element_at`, and `transfer_to_core` migration.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_kernel_scheduler::{RyuScheduler, CPU_CORES_COUNT, PRIORITIES_COUNT};
use std::hint::black_box;
use std::time::Duration;

const THREADS: usize = 96;
const YIELD_THREADS: usize = 6;
const YIELDS: usize = 1024;

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

/// (core, priority, thread_id) for a typical game process: most threads cluster around a
/// handful of priorities (28, 44, 59), some spread across the full range.
fn thread_set(count: usize, seed: u64) -> Vec<(usize, usize, u64)> {
    let mut rng = XorShift64(seed);
    let common = [28usize, 44, 44, 59];
    (0..count)
        .map(|i| {
            let r = rng.next();
            let prio = if r & 3 == 0 {
                (r >> 8) as usize % PRIORITIES_COUNT
            } else {
                common[(r >> 4) as usize % common.len()]
            };
            (
                (r >> 16) as usize % CPU_CORES_COUNT,
                prio,
                0x1000 + i as u64,
            )
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("kernel_scheduler");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let threads = thread_set(THREADS, 0x0123_4567_89ab_cdef);

    // schedule + unschedule: must return to an empty scheduler each iteration.
    let mut churn = Box::new(RyuScheduler::new());
    for &(c, p, t) in &threads {
        churn.schedule(c, p, t);
    }
    for &(c, p, t) in &threads {
        churn.unschedule(c, p, t);
    }
    assert!((0..CPU_CORES_COUNT).all(|c| !churn.has_scheduled(c)));

    group.throughput(Throughput::Elements(THREADS as u64));
    group.bench_function(BenchmarkId::new("schedule_unschedule", "96_threads"), |b| {
        b.iter(|| {
            let s = black_box(&mut *churn);
            for &(c, p, t) in black_box(threads.as_slice()) {
                s.schedule(c, p, t);
            }
            let first = s.scheduled_first(0);
            for &(c, p, t) in black_box(threads.as_slice()) {
                s.unschedule(c, p, t);
            }
            black_box(first)
        })
    });

    // Round-robin yield among 6 threads sharing core 0, priority 44.
    let mut yielder = Box::new(RyuScheduler::new());
    for i in 0..YIELD_THREADS as u64 {
        yielder.schedule(0, 44, 0x2000 + i);
    }
    assert_eq!(yielder.reschedule(0, 44, 0x2000), 0x2001);
    group.throughput(Throughput::Elements(YIELDS as u64));
    group.bench_function(BenchmarkId::new("reschedule", "6_threads_same_prio"), |b| {
        b.iter(|| {
            let s = black_box(&mut *yielder);
            let mut cur = s.scheduled_first(0);
            for _ in 0..YIELDS {
                cur = s.reschedule(0, 44, cur);
            }
            black_box(cur)
        })
    });

    // Walk every scheduled thread on each core by index (the selection loop pattern).
    let mut walker = Box::new(RyuScheduler::new());
    for &(c, p, t) in &threads {
        walker.schedule(c, p, t);
    }
    let per_core: Vec<usize> = (0..CPU_CORES_COUNT)
        .map(|c| threads.iter().filter(|t| t.0 == c).count())
        .collect();
    for (c, &n) in per_core.iter().enumerate() {
        assert_ne!(walker.scheduled_element_at(c, n - 1), 0);
        assert_eq!(walker.scheduled_element_at(c, n), 0);
    }
    group.throughput(Throughput::Elements(THREADS as u64));
    group.bench_function(
        BenchmarkId::new("scheduled_element_at", "walk_96_threads"),
        |b| {
            b.iter(|| {
                let s = black_box(&*walker);
                let mut acc = 0u64;
                for (c, &n) in per_core.iter().enumerate() {
                    for i in 0..n {
                        acc ^= s.scheduled_element_at(c, black_box(i));
                    }
                }
                black_box(acc)
            })
        },
    );

    // Migrate threads 0->1 then 1->0; state after the pair equals the starting state
    // (scheduled on 0, suggested on 1).
    let mut migr = Box::new(RyuScheduler::new());
    let movers: Vec<(usize, u64)> = threads.iter().take(32).map(|&(_, p, t)| (p, t)).collect();
    for &(p, t) in &movers {
        migr.schedule(0, p, t);
        migr.suggest(1, p, t);
    }
    let before = (migr.get_scheduled_mask(0), migr.get_suggested_mask(1));
    for &(p, t) in &movers {
        migr.transfer_to_core(p, 0, 1, t);
        migr.transfer_to_core(p, 1, 0, t);
    }
    assert_eq!(
        before,
        (migr.get_scheduled_mask(0), migr.get_suggested_mask(1))
    );
    assert_eq!(migr.get_scheduled_mask(1), 0);

    group.throughput(Throughput::Elements(2 * movers.len() as u64));
    group.bench_function(
        BenchmarkId::new("transfer_to_core", "32_threads_ping_pong"),
        |b| {
            b.iter(|| {
                let s = black_box(&mut *migr);
                for &(p, t) in black_box(movers.as_slice()) {
                    s.transfer_to_core(p, 0, 1, t);
                    s.transfer_to_core(p, 1, 0, t);
                }
                black_box(s.scheduled_first(0))
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
