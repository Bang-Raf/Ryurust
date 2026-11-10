//! Criterion benchmarks for `ryu-motion-fusion`: Mahony IMU filter updates over a
//! 1000-sample deterministic accelerometer/gyro trace, quaternion -> 3x3 rotation matrix,
//! and update + matrix per sample (what a controller poll does).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_motion_fusion::{MahonySensorFilter, Quaternion, Vector3};
use std::time::Duration;

const SAMPLES: usize = 1000;
const SAMPLE_PERIOD: f32 = 0.005; // 200 Hz IMU

/// Slow wobble around gravity with small noise-like terms; gyro in rad/s.
fn imu_trace() -> Vec<(Vector3, Vector3)> {
    (0..SAMPLES)
        .map(|i| {
            let t = i as f32 * SAMPLE_PERIOD;
            let accel = Vector3::new(
                0.05 + 0.2 * (t * 3.1).sin(),
                0.1 + 0.15 * (t * 2.3).cos(),
                0.98 + 0.02 * ((i * 7 % 13) as f32 / 13.0 - 0.5),
            );
            let gyro = Vector3::new(
                0.4 * (t * 3.1).cos(),
                -0.3 * (t * 2.3).sin(),
                0.05 * (t * 0.7).sin(),
            );
            (accel, gyro)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("motion_fusion");
    g.sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let trace = imu_trace();
    {
        let mut f = MahonySensorFilter::new(SAMPLE_PERIOD, 1.0, 0.05);
        for &(a, w) in &trace {
            f.update(a, w);
        }
        let len = f.quaternion.length();
        assert!(
            (len - 1.0).abs() < 1e-3,
            "filter quaternion not normalised: {len}"
        );
    }

    g.throughput(Throughput::Elements(SAMPLES as u64));
    g.bench_function(
        BenchmarkId::new("update", format!("{SAMPLES}samples")),
        |b| {
            let mut f = MahonySensorFilter::new(SAMPLE_PERIOD, 1.0, 0.05);
            b.iter(|| {
                f.reset();
                for &(a, w) in black_box(&trace) {
                    f.update(black_box(a), black_box(w));
                }
                black_box(f.quaternion);
            })
        },
    );

    g.bench_function(
        BenchmarkId::new("update_and_matrix", format!("{SAMPLES}samples")),
        |b| {
            let mut f = MahonySensorFilter::new(SAMPLE_PERIOD, 1.0, 0.05);
            b.iter(|| {
                f.reset();
                for &(a, w) in black_box(&trace) {
                    f.update(black_box(a), black_box(w));
                    black_box(f.quaternion.to_rotation_matrix_3x3());
                }
            })
        },
    );

    let q = {
        let (x, y, z, w) = (0.1f32, -0.2f32, 0.3f32, 0.927f32);
        let n = (x * x + y * y + z * z + w * w).sqrt();
        Quaternion::new(x / n, y / n, z / n, w / n)
    };
    g.throughput(Throughput::Elements(1));
    g.bench_function("rotation_matrix_3x3", |b| {
        b.iter(|| black_box(black_box(q).to_rotation_matrix_3x3()))
    });

    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
