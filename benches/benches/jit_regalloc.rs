//! Criterion benchmarks for `ryu-jit-regalloc`: a full linear-scan allocation of one
//! function (reset + add intervals + `run_pass`) for 512 and 2048 live intervals already
//! sorted by start, 512 intervals in shuffled order (exercises the sort), and the AVX2
//! liveness-bitset OR over a 4096-bit set.

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use ryu_jit_regalloc::{bitset_or_avx2, Interval, LinearScanEngine, RegisterMasksNative};
use std::time::Duration;

fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

/// x86-64-like masks: 16 GPRs minus RSP/RBP, 16 XMM.
fn masks() -> RegisterMasksNative {
    RegisterMasksNative {
        int_available: 0xFFFF & !((1 << 4) | (1 << 5)),
        vec_available: 0xFFFF,
        registers_count: 16,
    }
}

/// Mostly short-lived SSA temporaries, some medium, a few long-lived values; ~30% vector;
/// ~3% fixed to RAX/RCX/RDX (call/shift/div constraints). A fixed interval that would
/// overlap an earlier fixed interval on the same register is made non-fixed: two such
/// intervals are unsatisfiable and `run_pass` rejects the whole input.
fn intervals(n: usize, seed: u32, shuffled: bool) -> Vec<Interval> {
    let mut next = rng(seed);
    let mut fixed_until = [i32::MIN; 3];
    let mut v: Vec<Interval> = (0..n)
        .map(|i| {
            let start = (i * 2) as i32;
            let roll = next() % 100;
            let len = if roll < 80 {
                1 + next() % 16
            } else if roll < 95 {
                16 + next() % 112
            } else {
                128 + next() % 896
            } as i32;
            let is_vec = next() % 10 < 3;
            let mut is_fixed = !is_vec && next() % 100 < 3;
            let fixed_reg = if is_fixed { (next() % 3) as usize } else { 0 };
            if is_fixed {
                if fixed_until[fixed_reg] > start {
                    is_fixed = false;
                } else {
                    fixed_until[fixed_reg] = start + len;
                }
            }
            Interval {
                id: i as u32,
                start,
                end: start + len,
                reg_type: is_vec as u8,
                is_fixed,
                fixed_reg: if is_fixed { fixed_reg as i16 } else { -1 },
                first_use: start,
                ..Default::default()
            }
        })
        .collect();
    if shuffled {
        for i in (1..v.len()).rev() {
            let j = next() as usize % (i + 1);
            v.swap(i, j);
        }
    }
    v
}

fn run(engine: &mut LinearScanEngine, ivs: &[Interval]) -> usize {
    engine.reset();
    for &iv in ivs {
        engine.add_interval(iv);
    }
    engine.run_pass(masks())
}

/// Precondition: every interval is either assigned a register or given a spill slot.
fn check(ivs: &[Interval]) {
    let mut e = LinearScanEngine::new();
    let allocated = run(&mut e, ivs);
    assert_eq!(e.fixed_conflict(), None, "input must be satisfiable");
    let spilled = e
        .intervals
        .iter()
        .filter(|iv| iv.assigned_reg < 0 && iv.spill_offset >= 0)
        .count();
    assert_eq!(
        allocated + spilled,
        ivs.len(),
        "intervals neither allocated nor spilled"
    );
    assert!(allocated > 0);
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("jit_regalloc");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    for &(n, shuffled) in &[(512usize, false), (2048, false), (512, true)] {
        let ivs = intervals(n, 0x1234 + n as u32, shuffled);
        check(&ivs);
        let mut engine = LinearScanEngine::new();
        let id = format!("{n}_{}", if shuffled { "unsorted" } else { "sorted" });
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("linear_scan", id), &ivs, |b, ivs| {
            b.iter(|| {
                let allocated = run(&mut engine, black_box(ivs));
                black_box(allocated);
                black_box(&engine.intervals);
            })
        });
    }

    const WORDS: usize = 64;
    let mut next = rng(0xB175);
    let src: Vec<u64> = (0..WORDS)
        .map(|_| (next() as u64) << 32 | next() as u64)
        .collect();
    let dst0: Vec<u64> = (0..WORDS)
        .map(|_| (next() as u64) << 32 | next() as u64)
        .collect();
    g.throughput(Throughput::Bytes((WORDS * 8) as u64));
    g.bench_function(
        BenchmarkId::new("bitset_or", format!("{}bits", WORDS * 64)),
        |b| {
            b.iter_batched_ref(
                || dst0.clone(),
                |dst| black_box(bitset_or_avx2(black_box(dst), black_box(&src))),
                BatchSize::SmallInput,
            )
        },
    );

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
