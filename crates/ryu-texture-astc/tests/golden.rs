//! Golden-output tests against Ryujinx's C# `AstcDecoder` (see `tests/golden/README.md`).
//!
//! Every fixture holds ASTC input blocks, the C# per-block verdict, the C# image-level
//! status and the C# RGBA8 output. The Rust decoder must reproduce the status, every
//! per-block verdict and every output byte. The one intentional difference: the C#
//! decoder leaves a failed block's texels zeroed, while this crate writes the ASTC error
//! colour (opaque magenta) there. The test checks both halves of that exactly.

use std::fs;
use std::path::{Path, PathBuf};

use ryu_texture_astc::{decode_astc_to_rgba8, decompress_block, ASTC_ERROR_COLOR};

#[cfg(feature = "parallel")]
use ryu_texture_astc::decode_astc_to_rgba8_parallel;

struct Fixture {
    name: String,
    block_width: usize,
    block_height: usize,
    width: usize,
    height: usize,
    depth: usize,
    levels: usize,
    layers: usize,
    success: bool,
    input: Vec<u8>,
    block_ok: Vec<bool>,
    output: Vec<u8>,
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> usize {
    let v = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    v as usize
}

fn load(path: &Path) -> Fixture {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    assert_eq!(&bytes[..8], b"RYUASTC1", "{}: bad magic", path.display());
    let mut o = 8;
    let block_width = read_u32(&bytes, &mut o);
    let block_height = read_u32(&bytes, &mut o);
    let width = read_u32(&bytes, &mut o);
    let height = read_u32(&bytes, &mut o);
    let depth = read_u32(&bytes, &mut o);
    let levels = read_u32(&bytes, &mut o);
    let layers = read_u32(&bytes, &mut o);
    let blocks = read_u32(&bytes, &mut o);
    let output_len = read_u32(&bytes, &mut o);
    let success = bytes[o] != 0;
    o += 4;
    let input = bytes[o..o + blocks * 16].to_vec();
    o += blocks * 16;
    let block_ok = bytes[o..o + blocks].iter().map(|&b| b != 0).collect();
    o += blocks;
    let output = bytes[o..o + output_len].to_vec();
    o += output_len;
    assert_eq!(o, bytes.len(), "{}: trailing bytes", path.display());

    Fixture {
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        block_width,
        block_height,
        width,
        height,
        depth,
        levels,
        layers,
        success,
        input,
        block_ok,
        output,
    }
}

/// Byte ranges of the output that block `index` writes, following the C#
/// `AstcDecoder` level/layer layout and `WriteDecompressedBlock`.
fn block_output_rows(f: &Fixture, index: usize) -> Vec<std::ops::Range<usize>> {
    let (bw, bh) = (f.block_width, f.block_height);
    let mut start_block = 0;
    let mut out_offset = 0;
    for level in 0..f.levels {
        for _ in 0..f.layers {
            let sx = (f.width >> level).max(1);
            let sy = (f.height >> level).max(1);
            let sz = (f.depth >> level).max(1);
            let bx = sx.div_ceil(bw);
            let by = sy.div_ceil(bh);
            let count = bx * by * sz;
            if index < start_block + count {
                let local = index - start_block;
                let px = (local % bx) * bw;
                let py = (local / bx) * bh;
                let cols = (px + bw).min(sx) - px;
                // Can be zero for 3D levels, as in the C# decoder.
                let rows = (py + bh).min(sy * sz).saturating_sub(py);
                let stride = sx * 4;
                return (0..rows)
                    .map(|r| {
                        let s = out_offset + (py + r) * stride + px * 4;
                        s..s + cols * 4
                    })
                    .collect();
            }
            start_block += count;
            out_offset += sx * sy * sz * 4;
        }
    }
    panic!("block index {index} out of range");
}

fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b).position(|(x, y)| x != y)
}

type ImageDecoder = fn(&[u8], &mut [u8], usize, usize, usize, usize, usize, usize, usize) -> bool;

/// Checks one fixture; returns a description of every mismatch found.
fn check(f: &Fixture) -> Vec<String> {
    let mut problems = Vec::new();
    let (bw, bh) = (f.block_width, f.block_height);
    let blocks = f.block_ok.len();

    // Expected Rust output: the C# output, with each failed block's texels (which the
    // C# decoder leaves zeroed) replaced by the error colour.
    let mut expected = f.output.clone();
    for (i, &ok) in f.block_ok.iter().enumerate() {
        if ok {
            continue;
        }
        for row in block_output_rows(f, i) {
            if f.output[row.clone()].iter().any(|&b| b != 0) {
                problems.push(format!(
                    "{}: block {i}: C# output of a failed block is not zero",
                    f.name
                ));
            }
            let (texels, _) = expected[row].as_chunks_mut::<4>();
            texels.fill(ASTC_ERROR_COLOR.to_le_bytes());
        }
    }

    // Per-block verdict.
    let mut scratch = [0u32; 144];
    for i in 0..blocks {
        let block: &[u8; 16] = f.input[i * 16..i * 16 + 16].try_into().unwrap();
        let ok = decompress_block(block, &mut scratch, bw, bh);
        if ok != f.block_ok[i] {
            problems.push(format!(
                "{}: block {i} ({block:02x?}): rust ok={ok}, C# ok={}",
                f.name, f.block_ok[i]
            ));
        }
    }

    // Whole image, sequential (and parallel when enabled).
    let decoders: &[(&str, ImageDecoder)] = &[
        ("sequential", decode_astc_to_rgba8),
        #[cfg(feature = "parallel")]
        ("parallel", decode_astc_to_rgba8_parallel),
    ];

    for &(label, decode) in decoders {
        let mut out = vec![0xCDu8; f.output.len()];
        let ok = decode(
            &f.input, &mut out, bw, bh, f.width, f.height, f.depth, f.levels, f.layers,
        );
        if ok != f.success {
            problems.push(format!(
                "{}: {label}: status {ok}, C# status {}",
                f.name, f.success
            ));
        }
        if let Some(pos) = first_difference(&out, &expected) {
            let differing: Vec<usize> = (0..blocks)
                .filter(|&i| {
                    block_output_rows(f, i)
                        .into_iter()
                        .any(|r| out[r.clone()] != expected[r])
                })
                .collect();
            problems.push(format!(
                "{}: {label}: output differs (first byte {pos}); blocks {differing:?}",
                f.name
            ));
        }
    }

    problems
}

fn fixtures_in(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .collect();
    paths.sort();
    paths
}

fn run_dir(dir: &Path) {
    let paths = fixtures_in(dir);
    assert!(!paths.is_empty(), "no fixtures in {}", dir.display());
    let problems: Vec<String> = paths.iter().flat_map(|p| check(&load(p))).collect();
    assert!(
        problems.is_empty(),
        "{} mismatch(es) against the C# decoder:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn golden_fixtures_match_csharp_decoder() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    run_dir(&dir);

    // The fixture set must exercise both image-level outcomes.
    let all: Vec<Fixture> = fixtures_in(&dir).iter().map(|p| load(p)).collect();
    assert!(all.iter().any(|f| f.success));
    assert!(all.iter().any(|f| !f.success));
}

/// Runs extra fixtures produced by the same generator (for example a large fuzz set kept
/// outside the repository): `ASTC_GOLDEN_DIR=/path cargo test -p ryu-texture-astc
/// --test golden -- --ignored`.
#[test]
#[ignore = "needs ASTC_GOLDEN_DIR"]
fn extra_golden_fixtures_match_csharp_decoder() {
    let dir = std::env::var_os("ASTC_GOLDEN_DIR").expect("set ASTC_GOLDEN_DIR");
    run_dir(Path::new(&dir));
}
