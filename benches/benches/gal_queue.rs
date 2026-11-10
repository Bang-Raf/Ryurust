//! Benchmarks `ryu_gal_queue`: the SPSC command ring (`GalSpscQueue`) with
//! 64-byte command packets, single-threaded fill/drain and a two-thread
//! producer/consumer transfer, plus `GalSpanPool` insert/consume round trips.

use std::hint::black_box;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_gal_queue::{GalSpanPool, GalSpscQueue};

const ELEMENT_SIZE: usize = 64;
const QUEUE_CAPACITY: usize = 1024;
const BATCH: usize = 256;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("gal_queue");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // Single-threaded: push BATCH packets, then pop them all.
    let queue = GalSpscQueue::new(QUEUE_CAPACITY, ELEMENT_SIZE);
    group.throughput(Throughput::Elements(BATCH as u64));
    group.bench_function(BenchmarkId::new("spsc_fill_drain", BATCH), |b| {
        b.iter(|| {
            for i in 0..BATCH as u64 {
                let slot = queue.try_reserve();
                debug_assert!(slot >= 0);
                let ptr = queue.get_slot_ptr(slot as usize) as *mut u64;
                // SAFETY: slot < capacity and each slot is 64 bytes; only this thread touches the queue.
                unsafe { ptr.write_unaligned(black_box(i)) };
                queue.commit();
            }
            let mut sum = 0u64;
            for _ in 0..BATCH {
                let slot = queue.try_consume();
                debug_assert!(slot >= 0);
                let ptr = queue.get_slot_ptr(slot as usize) as *const u64;
                // SAFETY: as above; the slot was committed by the loop above.
                sum = sum.wrapping_add(unsafe { ptr.read_unaligned() });
                queue.release();
            }
            black_box(sum)
        });
    });
    assert_eq!(queue.pending_count(), 0);

    // Two threads: producer pushes `iters` packets, consumer pops and sums them.
    group.throughput(Throughput::Elements(1));
    group.bench_function("spsc_two_thread_transfer", |b| {
        b.iter_custom(|iters| {
            let q = Arc::new(GalSpscQueue::new(QUEUE_CAPACITY, ELEMENT_SIZE));
            let qc = Arc::clone(&q);
            let consumer = thread::spawn(move || {
                let mut sum = 0u64;
                for _ in 0..iters {
                    let slot = loop {
                        let s = qc.try_consume();
                        if s >= 0 {
                            break s as usize;
                        }
                        std::hint::spin_loop();
                    };
                    // SAFETY: slot was committed by the producer (Acquire on tail in try_consume).
                    sum = sum.wrapping_add(unsafe {
                        (qc.get_slot_ptr(slot) as *const u64).read_unaligned()
                    });
                    qc.release();
                }
                sum
            });
            let start = Instant::now();
            for i in 0..iters {
                let slot = loop {
                    let s = q.try_reserve();
                    if s >= 0 {
                        break s as usize;
                    }
                    std::hint::spin_loop();
                };
                // SAFETY: slot is free (consumer has released it) and owned by the producer until commit.
                unsafe { (q.get_slot_ptr(slot) as *mut u64).write_unaligned(i) };
                q.commit();
            }
            let sum = consumer.join().expect("consumer thread panicked");
            let elapsed = start.elapsed();
            assert_eq!(sum, iters.wrapping_mul(iters.wrapping_sub(1)) / 2);
            elapsed
        });
    });

    // Span pool: insert a payload, fetch the consumer pointer, advance.
    const POOL_SIZE: usize = 8 * 1024 * 1024;
    let pool = GalSpanPool::new(POOL_SIZE);
    for &size in &[64usize, 4096] {
        let payload: Vec<u8> = (0..size).map(|i| i as u8).collect();

        // Precondition: inserts keep succeeding across several wraparounds.
        pool.reset();
        for _ in 0..(3 * POOL_SIZE / size) {
            assert!(pool.try_insert(&payload) >= 0);
            let p = pool.get_consumer_ptr(size);
            assert!(!p.is_null());
            // SAFETY: p points at `size` valid bytes inside the pool.
            assert_eq!(unsafe { *p.add(size - 1) }, payload[size - 1]);
            pool.advance_consumer(size);
        }
        pool.reset();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("span_pool_insert_consume", size),
            &payload,
            |b, payload| {
                b.iter(|| {
                    let off = pool.try_insert(black_box(payload));
                    let p = pool.get_consumer_ptr(size);
                    pool.advance_consumer(size);
                    black_box((off, p))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
