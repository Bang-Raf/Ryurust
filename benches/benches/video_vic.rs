//! Criterion benchmarks for `ryu-video-vic`: full-surface 1920x1080 RGBA16 blend with a
//! BT.709 YUV->RGB colour matrix and with the matrix disabled (clamp-only), plus
//! motion-adaptive and bob deinterlacing of a 1920x1080 8-bit luma plane.
//!
//! Note: with the crate's default `parallel` feature, `blend_surface` splits rows across
//! the rayon thread pool, so the blend numbers are multi-threaded.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_video_vic::{
    blend_surface, deinterlace_bob, deinterlace_motion_adaptive, VicColorMatrix, VicPixel,
    VicRectangle,
};
use std::time::Duration;

const W: usize = 1920;
const H: usize = 1080;

fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

/// Full-range BT.709 YUV->RGB on 10-bit components, in the crate's
/// `((c . yuv) >> r_shift + offset) >> 8` fixed-point form (coefficients scaled by 256).
fn bt709_matrix() -> VicColorMatrix {
    VicColorMatrix {
        matrix_enable: 1,
        r_shift: 0,
        coeff00: 256,
        coeff01: 0,
        coeff02: 403,
        coeff03: -403 * 512,
        coeff10: 256,
        coeff11: -48,
        coeff12: -120,
        coeff13: (48 + 120) * 512,
        coeff20: 256,
        coeff21: 475,
        coeff22: 0,
        coeff23: -475 * 512,
        clamp_low: 0,
        clamp_high: 1023,
    }
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("video_vic");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // --- Surface blend ---
    let mut next = rng(0xA11CE);
    let src: Vec<VicPixel> = (0..W * H)
        .map(|_| VicPixel {
            r: (64 + next() % 877) as u16, // Y
            g: (64 + next() % 897) as u16, // U
            b: (64 + next() % 897) as u16, // V
            a: 1023,
        })
        .collect();
    let mut dst = vec![VicPixel::default(); W * H];
    let rect = VicRectangle {
        x: 0,
        y: 0,
        width: W as i32,
        height: H as i32,
    };
    let m709 = bt709_matrix();
    let passthrough = VicColorMatrix {
        matrix_enable: 0,
        ..Default::default()
    };

    blend_surface(&mut dst, W, H, &src, W, H, &rect, &m709);
    let in_range = dst.iter().filter(|p| p.r > 0 && p.r < 1023).count();
    assert!(
        in_range > W * H / 2,
        "BT.709 matrix clamps most pixels; test matrix is wrong"
    );

    let surface_bytes = (W * H * std::mem::size_of::<VicPixel>()) as u64;
    g.throughput(Throughput::Bytes(surface_bytes));
    g.bench_function(
        BenchmarkId::new("blend_surface_bt709", "1920x1080_rgba16"),
        |b| {
            b.iter(|| {
                blend_surface(
                    black_box(&mut dst),
                    W,
                    H,
                    black_box(&src),
                    W,
                    H,
                    black_box(&rect),
                    black_box(&m709),
                );
                black_box(&dst);
            })
        },
    );

    g.bench_function(
        BenchmarkId::new("blend_surface_passthrough", "1920x1080_rgba16"),
        |b| {
            b.iter(|| {
                blend_surface(
                    black_box(&mut dst),
                    W,
                    H,
                    black_box(&src),
                    W,
                    H,
                    black_box(&rect),
                    black_box(&passthrough),
                );
                black_box(&dst);
            })
        },
    );

    // --- Deinterlacing: field_size = line stride (W), 1080 lines ---
    let total = W * H;
    let mut data: Vec<u8> = (0..total).map(|_| next() as u8).collect();
    let prev: Vec<u8> = (0..total).map(|i| (i % 251) as u8).collect();
    // ~1/3 of pixels differ between prev and next -> "motion" path, rest -> weave path.
    let next_field: Vec<u8> = prev
        .iter()
        .enumerate()
        .map(|(i, &p)| if i % 3 == 0 { p.wrapping_add(20) } else { p })
        .collect();

    g.throughput(Throughput::Bytes(total as u64));
    g.bench_function(
        BenchmarkId::new("deinterlace_motion_adaptive", "1920x1080_luma"),
        |b| {
            b.iter(|| {
                deinterlace_motion_adaptive(
                    black_box(&mut data),
                    black_box(&prev),
                    black_box(&next_field),
                    W,
                    W,
                    black_box(true),
                );
                black_box(&data);
            })
        },
    );

    g.bench_function(BenchmarkId::new("deinterlace_bob", "1920x1080_luma"), |b| {
        b.iter(|| {
            deinterlace_bob(black_box(&mut data), W, W, black_box(true));
            black_box(&data);
        })
    });

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
