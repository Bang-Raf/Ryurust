//! Benchmarks for `ryu-storage-fs`: NCA AES-128-CTR section decryption (`decrypt_nca_ctr`)
//! at 16 KiB and 1 MiB, LZ4 and Zstandard block decompression of 64 KiB semi-compressible
//! blocks, and PFS0 file lookup by name (`find_pfs0_file`) in a 64-entry partition.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_storage_fs::{
    compress_lz4_block, compress_zstd_block, decompress_lz4_block, decompress_zstd_block,
    decrypt_nca_ctr, find_pfs0_file, PFS0_MAGIC,
};
use std::hint::black_box;
use std::time::Duration;

const KEY: [u8; 16] = [
    0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
];
const COUNTER: [u8; 16] = [
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const BLOCK: usize = 64 << 10;
const PFS0_FILES: usize = 64;

/// Semi-compressible data: runs of text-like bytes interleaved with noisier bytes.
fn semi_compressible(len: usize, seed: usize) -> Vec<u8> {
    (0..len)
        .map(|i| {
            if (i + seed) % 16 < 12 {
                (i % 26) as u8 + b'a'
            } else {
                ((i * 7 + seed) & 0xff) as u8
            }
        })
        .collect()
}

fn build_pfs0(names: &[String]) -> Vec<u8> {
    let mut strtab = Vec::new();
    let mut name_offsets = Vec::with_capacity(names.len());
    for n in names {
        name_offsets.push(strtab.len() as u32);
        strtab.extend_from_slice(n.as_bytes());
        strtab.push(0);
    }
    while strtab.len() % 0x20 != 0 {
        strtab.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(PFS0_MAGIC);
    out.extend_from_slice(&(names.len() as u32).to_le_bytes());
    out.extend_from_slice(&(strtab.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let mut data_off = 0u64;
    for (i, &no) in name_offsets.iter().enumerate() {
        let size = 0x1000 + (i as u64) * 0x340;
        out.extend_from_slice(&data_off.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&no.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        data_off += size;
    }
    out.extend_from_slice(&strtab);
    out
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_fs");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    for &size in &[16usize << 10, 1 << 20] {
        let plain = semi_compressible(size, 3);
        let mut cipher = vec![0u8; size];
        // Section offset 0xC000 into the NCA, a realistic non-zero counter offset.
        decrypt_nca_ctr(&plain, &mut cipher, &KEY, &COUNTER, 0xC000).unwrap();
        let mut out = vec![0u8; size];
        decrypt_nca_ctr(&cipher, &mut out, &KEY, &COUNTER, 0xC000).unwrap();
        assert_eq!(out, plain, "AES-CTR must round-trip");

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("aes_ctr_decrypt", format!("{}KiB", size >> 10)),
            &cipher,
            |b, cipher| {
                b.iter(|| {
                    decrypt_nca_ctr(
                        black_box(cipher),
                        black_box(&mut out),
                        black_box(&KEY),
                        black_box(&COUNTER),
                        black_box(0xC000),
                    )
                    .unwrap();
                    black_box(&out);
                })
            },
        );
    }

    let raw = semi_compressible(BLOCK, 5);
    let mut out = vec![0u8; BLOCK];

    let mut lz4 = vec![0u8; BLOCK * 2];
    let lz4_len = compress_lz4_block(&raw, &mut lz4).unwrap();
    lz4.truncate(lz4_len);
    assert_eq!(decompress_lz4_block(&lz4, &mut out).unwrap(), BLOCK);
    assert_eq!(out, raw, "LZ4 must round-trip");

    group.throughput(Throughput::Bytes(BLOCK as u64));
    group.bench_function(BenchmarkId::new("lz4_decompress", "64KiB"), |b| {
        b.iter(|| black_box(decompress_lz4_block(black_box(&lz4), black_box(&mut out)).unwrap()))
    });

    let mut zst = vec![0u8; BLOCK * 2];
    let zst_len = compress_zstd_block(&raw, &mut zst, 3).unwrap();
    zst.truncate(zst_len);
    out.fill(0);
    assert_eq!(decompress_zstd_block(&zst, &mut out).unwrap(), BLOCK);
    assert_eq!(out, raw, "Zstd must round-trip");

    group.bench_function(BenchmarkId::new("zstd_decompress", "64KiB_level3"), |b| {
        b.iter(|| black_box(decompress_zstd_block(black_box(&zst), black_box(&mut out)).unwrap()))
    });

    // NSP-like PFS0: one .cnmt.nca plus program/control/meta NCAs named by content id.
    let names: Vec<String> = (0..PFS0_FILES)
        .map(|i| {
            let id = 0x0100_0000_0000_0000u64.wrapping_mul(i as u64 + 1) ^ 0x5a5a_1234_abcd_0f0f;
            if i == 0 {
                format!("{id:016x}{id:016x}.cnmt.nca")
            } else {
                format!("{id:016x}{:016x}.nca", id.rotate_left(17))
            }
        })
        .collect();
    let pfs0 = build_pfs0(&names);
    for (i, n) in names.iter().enumerate() {
        assert_eq!(
            find_pfs0_file(&pfs0, n).unwrap().size,
            0x1000 + (i as u64) * 0x340
        );
    }

    group.throughput(Throughput::Elements(PFS0_FILES as u64));
    group.bench_function(BenchmarkId::new("pfs0_find_file", "all_64_names"), |b| {
        b.iter(|| {
            let mut total = 0u64;
            for n in black_box(names.as_slice()) {
                total += find_pfs0_file(black_box(&pfs0), n).unwrap().size;
            }
            black_box(total)
        })
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
