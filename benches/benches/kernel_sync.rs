//! Benchmarks for `ryu-kernel-sync`: the sharded `ArbiterRegistry` used for kernel address
//! arbitration. Measures uncontended single-threaded insert+wake and insert+remove cycles,
//! and `compute_signal_modify_addend` lookups against a populated registry.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_kernel_sync::ArbiterRegistry;
use std::hint::black_box;
use std::time::Duration;

const ADDRESSES: usize = 16;
const WAITERS_PER_ADDRESS: usize = 4;
const POPULATED_ADDRESSES: usize = 256;
const QUERIES: usize = 1024;

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

/// Guest mutex/condvar addresses: 4-byte aligned words scattered over a 64 MiB heap.
fn guest_addresses(count: usize, seed: u64) -> Vec<u64> {
    let mut rng = XorShift64(seed);
    let mut out: Vec<u64> = Vec::with_capacity(count);
    while out.len() < count {
        let a = 0x0000_0008_0000_0000u64 + ((rng.next() % (64 << 20)) & !3);
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

/// (address, thread_id, priority) tuples for one arbitration round.
fn waiter_plan(addresses: &[u64], per_addr: usize, seed: u64) -> Vec<(u64, u64, i32)> {
    let mut rng = XorShift64(seed);
    let mut tid = 0x100u64;
    let mut plan = Vec::with_capacity(addresses.len() * per_addr);
    for _ in 0..per_addr {
        for &a in addresses {
            plan.push((a, tid, 24 + (rng.next() % 40) as i32));
            tid += 1;
        }
    }
    plan
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("kernel_sync");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let addrs = guest_addresses(ADDRESSES, 0x1234_5678_9abc_def1);
    let plan = waiter_plan(&addrs, WAITERS_PER_ADDRESS, 0x0bad_cafe_f00d_d00d);

    // Precondition: one round leaves the registry empty, so iterations are independent.
    let reg = ArbiterRegistry::new();
    let mut woken = [0u64; WAITERS_PER_ADDRESS];
    for &(a, t, p) in &plan {
        reg.insert_waiter(a, t, p);
    }
    for &a in &addrs {
        assert_eq!(reg.wake_waiters(a, -1, &mut woken), WAITERS_PER_ADDRESS);
        assert_eq!(reg.get_waiter_count(a), 0);
    }

    group.throughput(Throughput::Elements(plan.len() as u64));
    group.bench_function(BenchmarkId::new("insert_then_wake_all", "16addr_x4"), |b| {
        b.iter(|| {
            let reg = black_box(&reg);
            for &(a, t, p) in black_box(plan.as_slice()) {
                reg.insert_waiter(a, t, p);
            }
            let mut total = 0usize;
            for &a in black_box(addrs.as_slice()) {
                total += reg.wake_waiters(a, -1, &mut woken);
            }
            black_box(woken);
            black_box(total)
        })
    });

    group.bench_function(BenchmarkId::new("insert_then_remove", "16addr_x4"), |b| {
        b.iter(|| {
            let reg = black_box(&reg);
            for &(a, t, p) in black_box(plan.as_slice()) {
                reg.insert_waiter(a, t, p);
            }
            let mut removed = 0usize;
            for &(a, t, _) in black_box(plan.as_slice()) {
                removed += reg.remove_waiter(a, t) as usize;
            }
            black_box(removed)
        })
    });

    // Populated registry: 256 addresses with 4 waiters each; query a mix of
    // waited-on and idle addresses.
    let populated = ArbiterRegistry::new();
    let pop_addrs = guest_addresses(POPULATED_ADDRESSES * 2, 0x5555_aaaa_1234_4321);
    for &(a, t, p) in &waiter_plan(&pop_addrs[..POPULATED_ADDRESSES], 4, 7) {
        populated.insert_waiter(a, t, p);
    }
    let mut rng = XorShift64(0xdead_beef_0123_4567);
    let queries: Vec<(u64, i32)> = (0..QUERIES)
        .map(|_| {
            let a = pop_addrs[(rng.next() as usize) % pop_addrs.len()];
            (a, 1 + (rng.next() % 4) as i32)
        })
        .collect();

    group.throughput(Throughput::Elements(QUERIES as u64));
    group.bench_function(
        BenchmarkId::new("compute_signal_modify_addend", "256addr_populated"),
        |b| {
            b.iter(|| {
                let reg = black_box(&populated);
                let mut acc = 0i32;
                for &(a, n) in black_box(queries.as_slice()) {
                    acc = acc.wrapping_add(reg.compute_signal_modify_addend(a, n));
                }
                black_box(acc)
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
