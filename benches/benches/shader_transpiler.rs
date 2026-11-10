//! Criterion benchmarks for `ryu-shader-transpiler`: Maxwell bytecode -> SPIR-V
//! transpilation (`transpile_maxwell_to_spirv`, reusing one arena) for a 4 KiB and a
//! 64 KiB fragment shader, plus raw instruction decoding of the 64 KiB shader.
//! Throughput is input bytecode bytes.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_shader_transpiler::{
    decode_instruction, transpile_maxwell_to_spirv, MaxwellOpcode, ShaderArena, TranspileConfig,
    SPIRV_MAGIC,
};

/// Deterministic instruction mix (ALU-heavy, a branch every 64 instructions, EXIT last).
fn generate_shader(bytes: usize) -> Vec<u8> {
    let count = bytes / 8;
    let mut code = Vec::with_capacity(bytes);
    for i in 0..count {
        let word: u64 = if i == count - 1 {
            0xE400_0000_0000_0007 // EXIT
        } else if i % 64 == 63 {
            0xE200_0010_0000_0007 // BRA
        } else {
            match i % 8 {
                0 => 0x5800_0000_0201_0007, // FADD
                1 => 0x5900_0000_0400_0307, // FMUL
                2 => 0x1C00_0000_0102_0307, // IADD
                3 => 0x5C00_0000_0100_0007, // MOV
                4 => 0x1D00_0000_0503_0207, // IMUL
                5 => 0x5A00_0000_0403_0107, // FFMA
                6 => 0xEF00_0010_0006_0507, // LDC
                _ => 0xC000_0000_0201_0007, // TEX
            }
        };
        code.extend_from_slice(&word.to_le_bytes());
    }
    code
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("shader_transpiler");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let config = TranspileConfig::default();
    let mut arena = ShaderArena::with_capacity(256 * 1024);

    for &(label, size) in &[("4KiB", 4 * 1024usize), ("64KiB", 64 * 1024)] {
        let code = generate_shader(size);
        let out = transpile_maxwell_to_spirv(&code, &config, &mut arena).unwrap();
        assert_eq!(out.instruction_count, size / 8);
        assert_eq!(out.spirv_words[0], SPIRV_MAGIC);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("transpile", label), &code, |b, code| {
            b.iter(|| {
                let out =
                    transpile_maxwell_to_spirv(black_box(code), black_box(&config), &mut arena);
                black_box(out)
            })
        });
    }

    let code = generate_shader(64 * 1024);
    let words: Vec<u64> = code
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    assert_eq!(
        decode_instruction(words[words.len() - 1], 0).opcode,
        MaxwellOpcode::Exit
    );
    group.throughput(Throughput::Bytes(code.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("decode_instructions", "64KiB"),
        &words,
        |b, words| {
            b.iter(|| {
                let mut acc = 0u32;
                for (i, &w) in black_box(words).iter().enumerate() {
                    let inst = decode_instruction(w, (i * 8) as u32);
                    acc = acc.wrapping_add(inst.opcode as u32 ^ inst.dst_reg as u32);
                }
                black_box(acc)
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
