//! Criterion benchmarks for `ryu-shader-cache`: LZ4 block decompression of a 64 KiB
//! shader-like payload, CRC32 over 64 KiB, and TOC header validation + entry parsing
//! for a 4096-entry table of contents.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_shader_cache::{
    compute_crc32, decompress_lz4_block, parse_toc_entries, validate_header, OffsetAndSize,
    FILE_FORMAT_VERSION_PACKED, TOCS_MAGIC,
};

const PAYLOAD: usize = 64 * 1024;
const TOC_ENTRIES: usize = 4096;

/// Shader-like payload: a sequence of multi-word "instruction templates" drawn from a
/// small set (so it compresses roughly like real shader code), with a pseudo-random
/// operand word appended to some of them.
fn shader_like_payload(len: usize) -> Vec<u8> {
    const TEMPLATES: [&[u32]; 8] = [
        &[0x0005_0081, 0x0000_0007, 0x0000_0020, 0x0000_0021],
        &[0x0005_0085, 0x0000_0007, 0x0000_0022, 0x0000_0023],
        &[0x0004_003D, 0x0000_0007, 0x0000_0030],
        &[0x0003_003E, 0x0000_0031, 0x0000_0032],
        &[
            0x0006_0041,
            0x0000_0040,
            0x0000_0041,
            0x0000_0042,
            0x0000_0043,
        ],
        &[0x0002_00F8, 0x0000_0050],
        &[0x0005_008E, 0x0000_0007, 0x0000_0060, 0x0000_0061],
        &[0x0004_0017, 0x0000_0070, 0x0000_0006, 0x0000_0004],
    ];
    let mut state = 0x5_4ADE_u64;
    let mut out = Vec::with_capacity(len + 32);
    while out.len() < len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r = (state >> 33) as u32;
        for w in TEMPLATES[(r & 7) as usize] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        if (r >> 3) % 4 == 0 {
            out.extend_from_slice(&(r >> 5).to_le_bytes());
        }
    }
    out.truncate(len);
    out
}

/// Minimal greedy LZ4 block compressor (hash of 4-byte prefixes, 64 KiB window),
/// used only to produce valid input for the decompressor.
fn lz4_compress(src: &[u8]) -> Vec<u8> {
    fn put_len(out: &mut Vec<u8>, mut n: usize) {
        while n >= 255 {
            out.push(255);
            n -= 255;
        }
        out.push(n as u8);
    }
    fn hash(v: u32) -> usize {
        (v.wrapping_mul(2654435761) >> 20) as usize
    }

    let mut out = Vec::with_capacity(src.len());
    let mut table = vec![usize::MAX; 1 << 12];
    let mut anchor = 0;
    let mut i = 0;
    // LZ4 rules: last match must start >= 12 bytes before end; last 5 bytes are literals.
    let match_limit = src.len().saturating_sub(12);

    while i < match_limit {
        let seq = u32::from_le_bytes(src[i..i + 4].try_into().unwrap());
        let h = hash(seq);
        let cand = table[h];
        table[h] = i;
        if cand != usize::MAX
            && i - cand <= 0xFFFF
            && u32::from_le_bytes(src[cand..cand + 4].try_into().unwrap()) == seq
        {
            let mut mlen = 4;
            while i + mlen < src.len() - 5 && src[cand + mlen] == src[i + mlen] {
                mlen += 1;
            }
            let lit = i - anchor;
            let ml = mlen - 4;
            out.push(((lit.min(15) as u8) << 4) | ml.min(15) as u8);
            if lit >= 15 {
                put_len(&mut out, lit - 15);
            }
            out.extend_from_slice(&src[anchor..i]);
            out.extend_from_slice(&((i - cand) as u16).to_le_bytes());
            if ml >= 15 {
                put_len(&mut out, ml - 15);
            }
            i += mlen;
            anchor = i;
        } else {
            i += 1;
        }
    }

    let lit = src.len() - anchor;
    out.push((lit.min(15) as u8) << 4);
    if lit >= 15 {
        put_len(&mut out, lit - 15);
    }
    out.extend_from_slice(&src[anchor..]);
    out
}

fn build_toc(entries: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + entries * 16);
    data.extend_from_slice(&TOCS_MAGIC.to_le_bytes());
    data.extend_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes()); // codegen version
    data.extend_from_slice(&0u32.to_le_bytes()); // padding
    data.extend_from_slice(&0x0123_4567_89AB_CDEFu64.to_le_bytes()); // timestamp
    data.extend_from_slice(&0u64.to_le_bytes()); // reserved
    let mut offset = 0u64;
    for i in 0..entries {
        let uncompressed = 2048 + (i as u32 * 97) % 8192;
        let compressed = uncompressed / 2 + (i as u32 % 64);
        data.extend_from_slice(&offset.to_le_bytes());
        data.extend_from_slice(&uncompressed.to_le_bytes());
        data.extend_from_slice(&compressed.to_le_bytes());
        offset += compressed as u64;
    }
    data
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("shader_cache");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let payload = shader_like_payload(PAYLOAD);
    let compressed = lz4_compress(&payload);
    let mut output = vec![0u8; PAYLOAD];
    assert_eq!(decompress_lz4_block(&compressed, &mut output), Ok(PAYLOAD));
    assert_eq!(output, payload);

    group.throughput(Throughput::Bytes(PAYLOAD as u64));
    group.bench_with_input(
        BenchmarkId::new(
            "lz4_decompress",
            format!("64KiB_from_{}B", compressed.len()),
        ),
        &compressed,
        |b, compressed| {
            b.iter(|| {
                let r = decompress_lz4_block(black_box(compressed), black_box(&mut output));
                black_box(r).ok();
                black_box(&output);
            })
        },
    );

    group.bench_with_input(BenchmarkId::new("crc32", "64KiB"), &payload, |b, data| {
        b.iter(|| black_box(compute_crc32(black_box(data))))
    });

    let toc = build_toc(TOC_ENTRIES);
    let mut entries = vec![OffsetAndSize::default(); TOC_ENTRIES];
    assert_eq!(parse_toc_entries(&toc, &mut entries), Ok(TOC_ENTRIES));
    assert!(validate_header(&toc, TOCS_MAGIC).is_ok());

    group.throughput(Throughput::Elements(TOC_ENTRIES as u64));
    group.bench_with_input(
        BenchmarkId::new("toc_parse", TOC_ENTRIES),
        &toc,
        |b, toc| {
            b.iter(|| {
                let hdr = validate_header(black_box(toc), TOCS_MAGIC);
                let n = parse_toc_entries(black_box(toc), black_box(&mut entries));
                black_box(hdr).ok();
                black_box(n).ok();
                black_box(&entries);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
