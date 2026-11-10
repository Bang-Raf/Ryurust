//! Benchmarks `ryu_vulkan_descriptors::RyuDescriptorState`: descriptor binding
//! updates with change detection, per-draw redundancy filtering of dirty
//! uniform buffers and textures, and template slice packing.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_vulkan_descriptors::{
    RyuDescriptorBufferInfo, RyuDescriptorImageInfo, RyuDescriptorState,
    MAX_UNIFORM_BUFFERS_PER_STAGE, MAX_UNIFORM_BUFFER_BINDINGS, SET_TEXTURE, SET_UNIFORM,
};

const DRAWS: usize = 100;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("vulkan_descriptors");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let mut state = RyuDescriptorState::new();

    // Rewrite all 90 uniform bindings with new values (every update is a real change).
    let mut gen = 0u64;
    group.throughput(Throughput::Elements(MAX_UNIFORM_BUFFER_BINDINGS as u64));
    group.bench_function(
        BenchmarkId::new("update_buffer", MAX_UNIFORM_BUFFER_BINDINGS),
        |b| {
            b.iter(|| {
                gen = gen.wrapping_add(1);
                let mut changed = 0u32;
                for i in 0..MAX_UNIFORM_BUFFER_BINDINGS {
                    changed +=
                        state.update_buffer(SET_UNIFORM, i, black_box(gen), (i as u64) * 256, 256)
                            as u32;
                }
                black_box(changed)
            });
        },
    );

    // Per draw: change 2 of the 18 stage-0 UBOs, re-set one unchanged UBO, then filter.
    let mut out_idx = [0u32; 64];
    let mut out_buf = [RyuDescriptorBufferInfo::default(); 64];
    state.reset();
    state.update_buffer(SET_UNIFORM, 0, 7, 0, 256);
    state.update_buffer(SET_UNIFORM, 1, 8, 0, 256);
    assert_eq!(
        state.filter_redundant_buffers(SET_UNIFORM, &mut out_idx, &mut out_buf),
        2
    );
    assert_eq!(
        out_buf[1],
        RyuDescriptorBufferInfo {
            buffer_handle: 8,
            offset: 0,
            range: 256
        }
    );
    assert!(!state.update_buffer(SET_UNIFORM, 0, 7, 0, 256));
    assert_eq!(
        state.filter_redundant_buffers(SET_UNIFORM, &mut out_idx, &mut out_buf),
        0
    );

    let mut draw = 0u64;
    group.throughput(Throughput::Elements(DRAWS as u64));
    group.bench_function(
        BenchmarkId::new("update_filter_buffers_per_draw", DRAWS),
        |b| {
            b.iter(|| {
                let mut emitted = 0usize;
                for _ in 0..DRAWS {
                    draw = draw.wrapping_add(1);
                    let i = (draw as usize * 2) % MAX_UNIFORM_BUFFERS_PER_STAGE;
                    state.update_buffer(SET_UNIFORM, i, black_box(draw + 1), 0, 256);
                    state.update_buffer(SET_UNIFORM, i + 1, black_box(draw + 2), 0, 256);
                    state.update_buffer(SET_UNIFORM, 17, 0x1234, 0, 256);
                    emitted +=
                        state.filter_redundant_buffers(SET_UNIFORM, &mut out_idx, &mut out_buf);
                }
                black_box(&out_buf);
                black_box(emitted)
            });
        },
    );

    // Per draw: rebind 32 textures across stages, half of them unchanged, then filter.
    let mut out_img_idx = [0u32; 320];
    let mut out_img = [RyuDescriptorImageInfo::default(); 320];
    group.bench_function(
        BenchmarkId::new("update_filter_images_per_draw", DRAWS),
        |b| {
            b.iter(|| {
                let mut emitted = 0usize;
                for _ in 0..DRAWS {
                    draw = draw.wrapping_add(1);
                    for t in 0..32usize {
                        let view = if t % 2 == 0 { draw } else { t as u64 };
                        state.update_image(SET_TEXTURE, t * 10, 0x55, black_box(view), 5);
                    }
                    emitted +=
                        state.filter_redundant_images(SET_TEXTURE, &mut out_img_idx, &mut out_img);
                }
                black_box(&out_img);
                black_box(emitted)
            });
        },
    );

    // Copy the 18 stage-0 UBO descriptors into template memory.
    const SLICE: usize = MAX_UNIFORM_BUFFERS_PER_STAGE;
    const SLICE_BYTES: usize = SLICE * std::mem::size_of::<RyuDescriptorBufferInfo>();
    let mut tmpl = [0u8; SLICE_BYTES];
    assert_eq!(
        state.pack_template_buffer_slice(SET_UNIFORM, 0, SLICE, &mut tmpl),
        SLICE_BYTES
    );
    group.throughput(Throughput::Bytes(SLICE_BYTES as u64));
    group.bench_function(BenchmarkId::new("pack_template_buffer_slice", SLICE), |b| {
        b.iter(|| {
            let n = state.pack_template_buffer_slice(
                SET_UNIFORM,
                black_box(0),
                SLICE,
                black_box(&mut tmpl),
            );
            black_box(&tmpl);
            black_box(n)
        });
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
