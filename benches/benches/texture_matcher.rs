//! Criterion benchmarks for `ryu-texture-matcher`: texture-cache compatibility
//! queries over a batch of 4096 deterministic (lhs, rhs) packed texture-key pairs
//! mixing colour, depth and compressed formats. Throughput is key pairs evaluated.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_texture_matcher::{
    format_matches, get_format_class, view_format_compatible, view_target_compatible,
    PackedTextureKey,
};

const PAIRS: usize = 4096;

// (format, bpp, is_compressed, is_depth_or_stencil). Format ids follow the crate's
// numbering (36 = R8G8B8A8Unorm, 52/9 depth/colour alias, 65..78 BCn, 109+ ASTC, ...).
const FORMATS: &[(u16, u8, bool, bool)] = &[
    (0, 1, false, false),
    (5, 2, false, false),
    (9, 4, false, false),
    (10, 4, false, false),
    (21, 8, false, false),
    (36, 4, false, false),
    (45, 16, false, false),
    (49, 1, false, true),
    (50, 2, false, true),
    (52, 4, false, true),
    (53, 4, false, true),
    (54, 8, false, true),
    (65, 8, true, false),
    (67, 16, true, false),
    (75, 16, true, false),
    (76, 16, true, false),
    (109, 16, true, false),
    (113, 16, true, false),
    (140, 4, false, false),
];

const TARGETS: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];

fn make_key(rng: &mut u64) -> PackedTextureKey {
    *rng = rng
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let r = *rng >> 16;
    let (format, bpp, compressed, depth) = FORMATS[(r as usize) % FORMATS.len()];
    let target = TARGETS[((r >> 8) as usize) % TARGETS.len()];
    PackedTextureKey::new(
        format,
        target,
        0,
        1,
        2,
        3,
        ((r >> 16) & 1) as u8,
        1,
        1,
        false,
        bpp,
        get_format_class(format as u32),
        compressed,
        depth,
    )
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_matcher");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Elements(PAIRS as u64));

    let mut rng = 0x7E47_u64;
    let pairs: Vec<(PackedTextureKey, PackedTextureKey)> = (0..PAIRS)
        .map(|_| (make_key(&mut rng), make_key(&mut rng)))
        .collect();

    group.bench_with_input(
        BenchmarkId::new("view_format_compatible", PAIRS),
        &pairs,
        |b, pairs| {
            b.iter(|| {
                let mut acc = 0u32;
                for &(l, r) in black_box(pairs).iter() {
                    acc = acc.wrapping_add(
                        view_format_compatible(l, r, false, false, true, false) as u32
                    );
                }
                black_box(acc)
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("view_target_compatible", PAIRS),
        &pairs,
        |b, pairs| {
            b.iter(|| {
                let mut acc = 0u32;
                for &(l, r) in black_box(pairs).iter() {
                    acc = acc
                        .wrapping_add(view_target_compatible(l.target(), r.target(), true) as u32);
                }
                black_box(acc)
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("full_match_query", PAIRS),
        &pairs,
        |b, pairs| {
            b.iter(|| {
                // Typical cache-lookup sequence: cheap mask checks, then format quality.
                let mut acc = 0u32;
                for &(l, r) in black_box(pairs).iter() {
                    if l.sampler_params_matches(&r) && l.target_and_samples_compatible(&r) {
                        acc = acc.wrapping_add(format_matches(
                            l.format() as u32,
                            r.format() as u32,
                            true,
                            false,
                        ) as u32);
                    }
                }
                black_box(acc)
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
