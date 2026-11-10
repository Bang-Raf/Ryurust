//! Benchmarks `ryu_gpu_gpfifo::parse_pushbuffer_chunk`, the GPFIFO pushbuffer
//! command-stream parser, on a mixed stream of incrementing, non-incrementing,
//! increment-once and immediate-data methods. Throughput is in parsed methods.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_gpu_gpfifo::{parse_pushbuffer_chunk, GpfifoParsedMethod, GpfifoState};

/// Encodes a pushbuffer method header word.
fn header(sec_op: u32, method: u32, subchannel: u32, count_or_imm: u32) -> i32 {
    (method | (subchannel << 13) | (count_or_imm << 16) | (sec_op << 29)) as i32
}

/// One chunk: 16 inc + 1 immediate + 32 non-inc + 4 one-inc = 53 methods (58 words).
fn chunk() -> (Vec<i32>, usize) {
    let mut s = Vec::new();
    s.push(header(1, 0x100, 0, 16));
    s.extend((0..16).map(|i| i * 4));
    s.push(header(4, 0x585, 0, 0x42));
    s.push(header(3, 0x890, 1, 32));
    s.extend((0..32).map(|i| i * 100));
    s.push(header(5, 0x200, 2, 4));
    s.extend((0..4).map(|i| i * 17));
    (s, 16 + 1 + 32 + 4)
}

fn bench(c: &mut Criterion) {
    let (one, per_chunk) = chunk();

    let mut group = c.benchmark_group("gpu_gpfifo");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // 1 chunk (~53 methods) and a frame-sized stream (~50k methods).
    for &chunks in &[1usize, 943] {
        let stream: Vec<i32> = one
            .iter()
            .copied()
            .cycle()
            .take(one.len() * chunks)
            .collect();
        let expected = per_chunk * chunks;
        let mut out = vec![
            GpfifoParsedMethod {
                method_address: 0,
                argument: 0,
                subchannel: 0,
                is_last_call: 0
            };
            expected + 16
        ];

        let mut st = GpfifoState::default();
        assert_eq!(parse_pushbuffer_chunk(&stream, &mut st, &mut out), expected);
        assert_eq!(st.method_count, 0);

        group.throughput(Throughput::Elements(expected as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_pushbuffer_chunk", format!("{expected}_methods")),
            &stream,
            |b, stream| {
                b.iter(|| {
                    let mut state = GpfifoState::default();
                    let n =
                        parse_pushbuffer_chunk(black_box(stream), &mut state, black_box(&mut out));
                    black_box((n, state));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
