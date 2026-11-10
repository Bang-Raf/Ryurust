//! Benchmarks `ryu_maxwell_state`: full 14 KiB Maxwell 3D register-block change
//! scan (runtime-dispatched SIMD path and scalar path), batched per-register
//! dirty marking, and the 64-bit pipeline-state key hasher.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_maxwell_state::*;

fn mapped_tracker() -> MaxwellStateTracker {
    let mut t = MaxwellStateTracker::new();
    t.map_register_range(0x000, 0x100, GROUP_VERTEX_BUFFER);
    t.map_register_range(0x100, 0x080, GROUP_VERTEX_ATTRIB);
    t.map_register_range(0x180, 0x080, GROUP_BLEND_STATE);
    t.map_register_range(0x800, 0x040, GROUP_RASTERIZER);
    t.map_register_range(0x900, 0x040, GROUP_SCISSOR);
    t.map_register_range(0xA00, 0x100, GROUP_SHADER_STATE);
    t.map_register_range(0xC00, 0x100, GROUP_RENDER_TARGET);
    t
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("maxwell_state");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // Full state block with two dirty registers (vertex buffer + scissor).
    let shadow = vec![0u8; MAXWELL_STATE_BYTES];
    let mut current = vec![0u8; MAXWELL_STATE_BYTES];
    current[0x020] = 1;
    current[0x904] = 2;
    let expected = (1u64 << GROUP_VERTEX_BUFFER) | (1u64 << GROUP_SCISSOR);

    let mut tracker = mapped_tracker();
    assert_eq!(tracker.scan_changes(&current, &shadow), expected);
    tracker.clear_dirty(!0);
    assert_eq!(tracker.scan_changes_scalar(&current, &shadow), expected);

    group.throughput(Throughput::Bytes(MAXWELL_STATE_BYTES as u64));
    group.bench_function("scan_changes/14KiB", |b| {
        b.iter(|| {
            tracker.clear_dirty(!0);
            black_box(tracker.scan_changes(black_box(&current), black_box(&shadow)))
        });
    });
    group.bench_function("scan_changes_scalar/14KiB", |b| {
        b.iter(|| {
            tracker.clear_dirty(!0);
            black_box(tracker.scan_changes_scalar(black_box(&current), black_box(&shadow)))
        });
    });

    // 1024 register writes spread over the first 0x1000 bytes (mapped and unmapped).
    let offsets: Vec<u32> = (0..1024u32)
        .map(|i| (i.wrapping_mul(37) % 0x400) * 4)
        .collect();
    group.throughput(Throughput::Elements(offsets.len() as u64));
    group.bench_function("set_dirty_batch/1024", |b| {
        b.iter(|| {
            tracker.clear_dirty(!0);
            tracker.set_dirty_batch(black_box(&offsets));
            black_box(tracker.get_dirty_mask())
        });
    });

    // Pipeline key hashing at a PipelineUid-like 72 bytes and a larger 1 KiB key.
    for &len in &[72usize, 1024] {
        let data: Vec<u8> = (0..len)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(7))
            .collect();
        group.throughput(Throughput::Bytes(len as u64));
        group.bench_with_input(
            BenchmarkId::new("pipeline_state_hash64", len),
            &data,
            |b, data| {
                b.iter(|| black_box(pipeline_state_hash64(black_box(data))));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
