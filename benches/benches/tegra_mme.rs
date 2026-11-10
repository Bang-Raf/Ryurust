//! Benchmarks `ryu_tegra_mme::MmeInterpreter::execute`, the Maxwell Macro
//! Execution Engine interpreter: a short straight-line ALU macro, and an
//! indirect multi-draw style loop macro that fetches two FIFO parameters and
//! emits two method calls per draw.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_tegra_mme::{MmeInterpreter, MmeMethodCall};

/// `R0 = R0 + 0` (Move). Opcode 0 would be IgnoreAndFetch and consume a FIFO word.
const NOP: i32 = 1 | (1 << 4);
/// NOP with the exit flag set.
const EXIT: i32 = NOP | 0x80;

const EMPTY_CALL: MmeMethodCall = MmeMethodCall {
    meth_addr: 0,
    value: 0,
};

/// Straight-line ALU sequence: add-immediate, xor, and, sub, then exit + delay slot.
const ALU_SEQ: [i32; 8] = [
    1 | (1 << 4) | (2 << 8) | (15 << 14),
    1 | (1 << 4) | (3 << 8) | (2 << 11) | (25 << 14),
    (1 << 4) | (4 << 8) | (3 << 11) | (2 << 14) | (8 << 17),
    (1 << 4) | (5 << 8) | (3 << 11) | (2 << 14) | (10 << 17),
    (1 << 4) | (6 << 8) | (3 << 11) | (2 << 14) | (2 << 17),
    1 | (1 << 4) | (7 << 8) | (6 << 11) | (100 << 14),
    EXIT,
    NOP,
];

/// Loop macro: R2 = arg0; maddr = 0x585;
/// loop { R4 = fetch; send R4; R5 = fetch; send R5; R2 -= 1 } while R2 != 0.
fn indirect_draw_code() -> [i32; 11] {
    let imm_neg_1 = (-1i32) << 14;
    let imm_neg_5 = (-5i32) << 14;
    [
        1 | (1 << 4) | (2 << 8) | (1 << 11),             // R2 = R1 (arg0)
        1 | (2 << 4) | (3 << 8) | (0x585 << 14),         // maddr = 0x585
        1 | (4 << 8),                                    // R4 = fetch
        1 | (4 << 4) | (4 << 11),                        // send R4
        1 | (5 << 8),                                    // R5 = fetch
        1 | (4 << 4) | (5 << 11),                        // send R5
        1 | (1 << 4) | (2 << 8) | (2 << 11) | imm_neg_1, // R2 -= 1
        7 | (1 << 4) | (2 << 11) | imm_neg_5,            // branch to op2 if R2 != 0
        NOP,                                             // delay slot
        EXIT,
        NOP,
    ]
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("tegra_mme");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let mut interp = MmeInterpreter::new();

    // Straight-line ALU macro; throughput counted in macro instructions.
    let mut calls = [EMPTY_CALL; 16];
    group.throughput(Throughput::Elements(ALU_SEQ.len() as u64));
    group.bench_function("execute_alu_seq", |b| {
        b.iter(|| {
            let n = interp.execute(black_box(&ALU_SEQ), black_box(0), &[], &[], &mut calls);
            black_box((n, &calls));
        });
    });

    // Indirect draw loop; throughput counted in draws (2 method calls each).
    let code = indirect_draw_code();
    for &draws in &[10usize, 200, 2000] {
        let fifo: Vec<i32> = (0..draws * 2).map(|i| (i * 3 + 1) as i32).collect();
        let mut out = vec![EMPTY_CALL; draws * 2 + 16];

        let n = interp.execute(&code, draws as i32, &fifo, &[], &mut out);
        assert_eq!(
            n,
            draws * 2,
            "indirect draw macro must emit 2 calls per draw"
        );
        assert!(out[..n]
            .iter()
            .zip(&fifo)
            .all(|(c, &v)| c.meth_addr == 0x585 && c.value == v));

        group.throughput(Throughput::Elements(draws as u64));
        group.bench_with_input(
            BenchmarkId::new("execute_indirect_draw", draws),
            &fifo,
            |b, fifo| {
                b.iter(|| {
                    let n = interp.execute(
                        black_box(&code),
                        black_box(draws as i32),
                        black_box(fifo),
                        &[],
                        black_box(&mut out),
                    );
                    black_box(n);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
