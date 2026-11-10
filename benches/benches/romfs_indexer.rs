//! Benchmarks for `ryu-romfs-indexer` on a synthetic RomFS metadata image (100 directories up
//! to 3 levels deep, 10,000 files) built with `MockRomFsBuilder`: path lookups through
//! `RyuRomFs::find_file` (hits and misses), the borrowed `RomFsReader::find_file`, and
//! `RyuRomFs::from_metadata_bytes` (header parse + metadata copy + entry count).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_romfs_indexer::{MockRomFsBuilder, RomFsReader, RyuRomFs};
use std::hint::black_box;
use std::time::Duration;

const DIRS: usize = 100;
const FILES: usize = 10_000;
const LOOKUPS: usize = 1024;

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

/// Returns (metadata image, file paths with their data sizes).
fn build_image() -> (Vec<u8>, Vec<(String, u64)>) {
    let mut b = MockRomFsBuilder::new();
    let tops = ["Data", "Sound", "Shader", "Model", "Movie"];
    let mut dir_ids = vec![0u32];
    let mut dir_paths = vec![String::new()];
    for t in tops {
        dir_ids.push(b.add_dir(0, t));
        dir_paths.push(format!("/{t}"));
    }
    let mut rng = XorShift64(0x1357_9bdf_2468_ace0);
    while dir_ids.len() < DIRS {
        // Nest under a top-level or second-level dir (max depth 3).
        let parent = 1 + (rng.next() as usize) % (dir_ids.len() - 1);
        if dir_paths[parent].matches('/').count() >= 3 {
            continue;
        }
        let name = format!("pack_{:03}", dir_ids.len());
        let id = b.add_dir(dir_ids[parent], &name);
        let path = format!("{}/{}", dir_paths[parent], name);
        dir_ids.push(id);
        dir_paths.push(path);
    }
    let exts = ["bfres", "bntx", "bars", "szs", "msbt", "byml"];
    let mut files = Vec::with_capacity(FILES);
    for f in 0..FILES {
        let d = (rng.next() as usize) % dir_ids.len();
        let name = format!("asset_{f:05}.{}", exts[f % exts.len()]);
        let size = 4096 + (f as u64) * 128;
        b.add_file(dir_ids[d], &name, (f as u64) * 0x1000, size);
        files.push((format!("{}/{}", dir_paths[d], name), size));
    }
    (b.build_metadata(), files)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("romfs_indexer");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let (image, files) = build_image();
    let romfs = RyuRomFs::from_metadata_bytes(&image).expect("valid RomFS image");
    assert_eq!(romfs.file_count(), FILES);
    assert_eq!(romfs.dir_count(), DIRS);

    let mut rng = XorShift64(0x9e37_79b9_7f4a_7c15);
    let hits: Vec<(String, u64)> = (0..LOOKUPS)
        .map(|_| files[(rng.next() as usize) % files.len()].clone())
        .collect();
    for (p, sz) in &hits {
        assert_eq!(romfs.find_file(p).map(|i| i.size), Some(*sz), "lookup {p}");
    }
    let misses: Vec<String> = hits
        .iter()
        .map(|(p, _)| p.replacen("asset_", "missing_", 1))
        .collect();
    for p in &misses {
        assert!(romfs.find_file(p).is_none());
    }

    group.throughput(Throughput::Elements(LOOKUPS as u64));
    group.bench_function(BenchmarkId::new("find_file", "hit_random_10k_files"), |b| {
        b.iter(|| {
            let r = black_box(&romfs);
            let mut total = 0u64;
            for (p, _) in black_box(hits.as_slice()) {
                total = total.wrapping_add(r.find_file(p).map_or(0, |i| i.offset));
            }
            black_box(total)
        })
    });

    group.bench_function(
        BenchmarkId::new("find_file", "miss_random_10k_files"),
        |b| {
            b.iter(|| {
                let r = black_box(&romfs);
                let mut found = 0usize;
                for p in black_box(misses.as_slice()) {
                    found += r.find_file(p).is_some() as usize;
                }
                black_box(found)
            })
        },
    );

    let reader = RomFsReader::from_bytes(&image).expect("valid RomFS image");
    group.bench_function(
        BenchmarkId::new("reader_find_file", "hit_random_10k_files"),
        |b| {
            b.iter(|| {
                let r = black_box(&reader);
                let mut total = 0u64;
                for (p, _) in black_box(hits.as_slice()) {
                    total = total.wrapping_add(r.find_file(p).map_or(0, |i| i.offset));
                }
                black_box(total)
            })
        },
    );

    group.throughput(Throughput::Bytes(image.len() as u64));
    group.bench_function(BenchmarkId::new("from_metadata_bytes", "10k_files"), |b| {
        b.iter(|| black_box(RyuRomFs::from_metadata_bytes(black_box(&image))))
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
