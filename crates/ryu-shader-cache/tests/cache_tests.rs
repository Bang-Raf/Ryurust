use ryu_shader_cache::*;

#[test]
fn test_version() {
    assert_eq!(ryu_shader_cache_version(), 0x0001_0000);
}

#[test]
fn test_crc32_standard_vector() {
    // "123456789" has standard IEEE 802.3 CRC32 = 0xCBF43926
    let data = b"123456789";
    assert_eq!(compute_crc32(data), 0xCBF43926);
}

#[test]
fn test_toc_header_validation() {
    let mut header_bytes = [0u8; 32];
    header_bytes[0..4].copy_from_slice(&TOCS_MAGIC.to_le_bytes());
    header_bytes[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&7354u32.to_le_bytes()); // codegen
    header_bytes[16..24].copy_from_slice(&1234567890u64.to_le_bytes()); // timestamp

    let parsed = validate_header(&header_bytes, TOCS_MAGIC).expect("should be valid");
    assert_eq!(parsed.magic, TOCS_MAGIC);
    assert_eq!(parsed.format_version, FILE_FORMAT_VERSION_PACKED);
    assert_eq!(parsed.codegen_version, 7354);
    assert_eq!(parsed.timestamp, 1234567890);

    // Invalid magic
    assert_eq!(
        validate_header(&header_bytes, TOCH_MAGIC),
        Err(TocError::InvalidMagic)
    );

    // Invalid version
    header_bytes[4..8].copy_from_slice(&0x9999_9999u32.to_le_bytes());
    assert_eq!(
        validate_header(&header_bytes, TOCS_MAGIC),
        Err(TocError::UnsupportedVersion)
    );
}

#[test]
fn test_toc_entries_parse() {
    let mut data = vec![0u8; 32]; // header
    data[0..4].copy_from_slice(&TOCH_MAGIC.to_le_bytes());
    data[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());

    // Add 100 entries (each 16 bytes)
    for i in 0..100u64 {
        data.extend_from_slice(&(i * 1024).to_le_bytes()); // offset
        data.extend_from_slice(&4096u32.to_le_bytes()); // uncompressed size
        data.extend_from_slice(&1024u32.to_le_bytes()); // compressed size
    }

    let mut entries = vec![OffsetAndSize::default(); 100];
    let count = parse_toc_entries(&data, &mut entries).expect("should parse");
    assert_eq!(count, 100);

    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.offset, (i as u64) * 1024);
        assert_eq!(entry.uncompressed_size, 4096);
        assert_eq!(entry.compressed_size, 1024);
    }
}

#[test]
fn test_lz4_decompression_basic() {
    // Manually constructed simple LZ4 block:
    // 5 literals "HELLO", then match offset 5, match length 5 (repeated "HELLO")
    // Total decompressed: "HELLOHELLO" (10 bytes)
    // Token 1: lit_len = 5, match_len = 5 - 4 = 1 => token = (5 << 4) | 1 = 0x51
    // Literals: "HELLO" (5 bytes)
    // Offset: 5 (2 bytes LE = [0x05, 0x00])
    let compressed = [0x51, b'H', b'E', b'L', b'L', b'O', 0x05, 0x00];

    let mut output = [0u8; 10];
    let len = decompress_lz4_block(&compressed, &mut output).expect("lz4 decompress ok");
    assert_eq!(len, 10);
    assert_eq!(&output, b"HELLOHELLO");
}

#[test]
fn test_ffi_c_abi() {
    let mut header_bytes = [0u8; 32];
    header_bytes[0..4].copy_from_slice(&TOCS_MAGIC.to_le_bytes());
    header_bytes[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());

    let mut out_header = TocHeader {
        magic: 0,
        format_version: 0,
        codegen_version: 0,
        padding: 0,
        timestamp: 0,
        reserved: 0,
    };

    // SAFETY: `header_bytes` is a live 32-byte buffer and `out_header` a live local.
    unsafe {
        assert_eq!(
            ryu_shader_cache_validate_toc(header_bytes.as_ptr(), 32, TOCS_MAGIC, &mut out_header),
            0
        );
        assert_eq!(out_header.magic, TOCS_MAGIC);

        let crc = ryu_shader_cache_crc32(b"123456789".as_ptr(), 9);
        assert_eq!(crc, 0xCBF43926);
    }
}

// =========================================================================
// Header validation in parse_toc_entries
// =========================================================================

fn toc_with_header(magic: u32, version: u32, entries: u64) -> Vec<u8> {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&magic.to_le_bytes());
    data[4..8].copy_from_slice(&version.to_le_bytes());
    for i in 0..entries {
        data.extend_from_slice(&(i * 16).to_le_bytes());
        data.extend_from_slice(&64u32.to_le_bytes());
        data.extend_from_slice(&32u32.to_le_bytes());
    }
    data
}

#[test]
fn test_toc_entries_require_valid_header() {
    let mut entries = vec![OffsetAndSize::default(); 4];

    let tocs = toc_with_header(TOCS_MAGIC, FILE_FORMAT_VERSION_PACKED, 3);
    assert_eq!(parse_toc_entries(&tocs, &mut entries), Ok(3));
    assert_eq!(
        parse_toc_entries_with_magic(&tocs, TOCS_MAGIC, &mut entries),
        Ok(3)
    );
    assert_eq!(
        parse_toc_entries_with_magic(&tocs, TOCH_MAGIC, &mut entries),
        Err(TocError::InvalidMagic)
    );

    let bad_magic = toc_with_header(SHDI_MAGIC, FILE_FORMAT_VERSION_PACKED, 3);
    assert_eq!(
        parse_toc_entries(&bad_magic, &mut entries),
        Err(TocError::InvalidMagic)
    );

    let bad_version = toc_with_header(TOCH_MAGIC, 0x0002_0000, 3);
    assert_eq!(
        parse_toc_entries(&bad_version, &mut entries),
        Err(TocError::UnsupportedVersion)
    );

    assert_eq!(
        parse_toc_entries(&tocs[..31], &mut entries),
        Err(TocError::BufferTooSmall)
    );

    // SAFETY: the closure passes live slices with their own lengths.
    unsafe {
        let ffi = |d: &[u8], e: &mut [OffsetAndSize]| {
            ryu_shader_cache_parse_entries(d.as_ptr(), d.len(), e.as_mut_ptr(), e.len())
        };
        assert_eq!(ffi(&tocs, &mut entries), 3);
        assert_eq!(ffi(&bad_magic, &mut entries), -2);
        assert_eq!(ffi(&bad_version, &mut entries), -3);
        assert_eq!(ffi(&tocs[..20], &mut entries), -1);
    }
}

// =========================================================================
// LZ4 error kinds and equivalence with the original byte-at-a-time decoder
// =========================================================================

#[test]
fn test_lz4_truncated_literals_is_corrupt_input() {
    // Token claims 5 literals but only 3 follow.
    let compressed = [0x50, b'A', b'B', b'C'];
    let mut output = [0u8; 64];
    assert_eq!(
        decompress_lz4_block(&compressed, &mut output),
        Err(Lz4Error::CorruptedInput)
    );

    // Literals present but the destination is too small.
    let compressed = [0x50, b'A', b'B', b'C', b'D', b'E'];
    let mut small = [0u8; 4];
    assert_eq!(
        decompress_lz4_block(&compressed, &mut small),
        Err(Lz4Error::OutputBufferTooSmall)
    );
}

/// The decoder as it was before the match-copy rewrite (byte-at-a-time copy), with
/// the corrected literal error kind. Used as a reference.
fn reference_lz4(input: &[u8], output: &mut [u8]) -> Result<usize, Lz4Error> {
    let (mut in_pos, mut out_pos) = (0, 0);
    while in_pos < input.len() {
        let token = input[in_pos];
        in_pos += 1;
        let mut lit_len = (token >> 4) as usize;
        if lit_len == 15 {
            loop {
                let s = *input.get(in_pos).ok_or(Lz4Error::CorruptedInput)? as usize;
                in_pos += 1;
                lit_len += s;
                if s != 255 {
                    break;
                }
            }
        }
        if in_pos + lit_len > input.len() {
            return Err(Lz4Error::CorruptedInput);
        }
        if out_pos + lit_len > output.len() {
            return Err(Lz4Error::OutputBufferTooSmall);
        }
        output[out_pos..out_pos + lit_len].copy_from_slice(&input[in_pos..in_pos + lit_len]);
        in_pos += lit_len;
        out_pos += lit_len;
        if in_pos >= input.len() {
            break;
        }
        if in_pos + 2 > input.len() {
            return Err(Lz4Error::CorruptedInput);
        }
        let offset = input[in_pos] as usize | ((input[in_pos + 1] as usize) << 8);
        in_pos += 2;
        if offset == 0 || offset > out_pos {
            return Err(Lz4Error::InvalidOffset);
        }
        let mut match_len = (token & 0x0F) as usize;
        if match_len == 15 {
            loop {
                let s = *input.get(in_pos).ok_or(Lz4Error::CorruptedInput)? as usize;
                in_pos += 1;
                match_len += s;
                if s != 255 {
                    break;
                }
            }
        }
        match_len += 4;
        if out_pos + match_len > output.len() {
            return Err(Lz4Error::OutputBufferTooSmall);
        }
        let match_src = out_pos - offset;
        for i in 0..match_len {
            output[out_pos + i] = output[match_src + i];
        }
        out_pos += match_len;
    }
    Ok(out_pos)
}

struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn push_len_ext(out: &mut Vec<u8>, mut rest: usize) {
    while rest >= 255 {
        out.push(255);
        rest -= 255;
    }
    out.push(rest as u8);
}

/// Generates a well-formed LZ4 block with many short-offset (overlapping) matches.
fn random_lz4_block(rng: &mut XorShift) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut produced = 0usize;
    let sequences = 1 + rng.below(40);
    for seq in 0..sequences {
        let lit_len = if seq == 0 {
            1 + rng.below(20)
        } else {
            rng.below(300)
        };
        let last = seq + 1 == sequences;
        let match_span = if rng.below(4) == 0 { 400 } else { 20 };
        let match_len = 4 + rng.below(match_span);
        let token_lit = lit_len.min(15) as u8;
        let token_match = (match_len - 4).min(15) as u8;
        out.push((token_lit << 4) | if last { 0 } else { token_match });
        if lit_len >= 15 {
            push_len_ext(&mut out, lit_len - 15);
        }
        for _ in 0..lit_len {
            out.push(rng.next() as u8);
        }
        produced += lit_len;
        if last {
            break;
        }
        let offset = match rng.below(3) {
            0 => 1 + rng.below(produced.min(8)),
            1 => 1 + rng.below(produced.min(64)),
            _ => 1 + rng.below(produced.min(65535)),
        };
        out.extend_from_slice(&(offset as u16).to_le_bytes());
        if match_len - 4 >= 15 {
            push_len_ext(&mut out, match_len - 4 - 15);
        }
        produced += match_len;
    }
    (out, produced)
}

#[test]
fn test_lz4_matches_reference_on_random_blocks() {
    let mut rng = XorShift(0xC0FF_EE12_3456_789B);
    for _ in 0..3000 {
        let (block, len) = random_lz4_block(&mut rng);
        let mut want = vec![0u8; len];
        let mut got = vec![0u8; len];
        assert_eq!(reference_lz4(&block, &mut want), Ok(len));
        assert_eq!(decompress_lz4_block(&block, &mut got), Ok(len));
        assert_eq!(got, want);

        // Truncated input / short output must report the same errors as the reference.
        let cut = rng.below(block.len());
        let mut a = vec![0u8; len];
        let mut b = vec![0u8; len];
        assert_eq!(
            decompress_lz4_block(&block[..cut], &mut a),
            reference_lz4(&block[..cut], &mut b)
        );
        let short = rng.below(len);
        let mut a = vec![0u8; short];
        let mut b = vec![0u8; short];
        assert_eq!(
            decompress_lz4_block(&block, &mut a),
            reference_lz4(&block, &mut b)
        );
    }

    // Arbitrary garbage must never panic and must agree with the reference.
    for _ in 0..3000 {
        let n = rng.below(64);
        let garbage: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let mut a = vec![0u8; 256];
        let mut b = vec![0u8; 256];
        let ra = decompress_lz4_block(&garbage, &mut a);
        assert_eq!(ra, reference_lz4(&garbage, &mut b));
        if ra.is_ok() {
            assert_eq!(a, b);
        }
    }
}

// =========================================================================
// CRC32 slicing-by-8 vs bytewise reference
// =========================================================================

fn reference_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn test_crc32_matches_reference_on_random_inputs() {
    let mut rng = XorShift(0x0BAD_5EED_1234_5678);
    assert_eq!(compute_crc32(&[]), 0);
    for len in (0..64).chain([255, 256, 1000, 4096, 65537]) {
        for _ in 0..8 {
            let data: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            assert_eq!(compute_crc32(&data), reference_crc32(&data), "len {len}");
            // Unaligned sub-slices.
            if len > 3 {
                let sub = &data[3..];
                assert_eq!(compute_crc32(sub), reference_crc32(sub), "len {len} +3");
            }
        }
    }
}
