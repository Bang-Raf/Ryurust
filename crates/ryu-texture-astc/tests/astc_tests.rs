use ryu_texture_astc::bitstream::BitStream128;
use ryu_texture_astc::decoder::*;
use ryu_texture_astc::ffi::*;

#[test]
fn test_query_decompressed_size() {
    // 16x16, 1 level, 1 layer = 256 pixels * 4 bytes = 1024 bytes
    assert_eq!(query_decompressed_size(16, 16, 1, 1, 1), 1024);

    // 16x16, 2 levels (16x16 + 8x8 = 256 + 64 = 320 pixels) * 4 = 1280 bytes
    assert_eq!(query_decompressed_size(16, 16, 1, 2, 1), 1280);

    // FFI query matches
    assert_eq!(ryu_astc_query_decompressed_size(16, 16, 1, 2, 1), 1280);
}

#[test]
fn test_bitstream_read_write() {
    let mut bs = BitStream128::empty();
    bs.write_bits(0xAB, 8);
    bs.write_bits(0x1234, 16);
    bs.write_bits(0x7, 3);

    let raw = bs.to_le_bytes();
    let mut reader = BitStream128::new(&raw);

    let val1 = reader.read_bits(8);
    let val2 = reader.read_bits(16);
    let val3 = reader.read_bits(3);

    assert_eq!(val1, 0xAB);
    assert_eq!(val2, 0x1234);
    assert_eq!(val3, 0x7);
}

#[test]
fn test_void_extent_ldr_block_decompression() {
    // Craft a valid Void Extent LDR ASTC block
    // Bits [0..10]: mode_bits = 0x1FC (void extent LDR)
    // Bit 10 = 1, Bit 11 = 1 (mode_bits |= 0x600, plus bit 11)
    let mut stream = BitStream128::empty();
    // 0x1FC | (1 << 10) = 0x5FC
    stream.write_bits(0x5FC, 11);
    stream.write_bits(1, 1); // next bit must be 1

    // 4 * 13 bits of void extent coordinates
    for _ in 0..4 {
        stream.write_bits(0, 13);
    }

    // RGBA 16-bit components (0xFF00, 0x8000, 0x4000, 0xFF00)
    // Red = 0xFF, Green = 0x80, Blue = 0x40, Alpha = 0xFF
    let r_in: u16 = 0xFF00;
    let g_in: u16 = 0x8000;
    let b_in: u16 = 0x4000;
    let a_in: u16 = 0xFF00;

    stream.write_bits(r_in as u32, 16);
    stream.write_bits(g_in as u32, 16);
    stream.write_bits(b_in as u32, 16);
    stream.write_bits(a_in as u32, 16);

    let raw_bytes = stream.to_le_bytes();

    let mut out_pixels = [0u32; 16 * 16];
    let ok = decompress_block(&raw_bytes, &mut out_pixels[..16], 4, 4);
    assert!(ok, "Void extent block decompression failed");

    // Expected RGBA8: R=0xFF, G=0x80, B=0x40, A=0xFF
    // In little-endian u32: (A << 24) | (B << 16) | (G << 8) | R
    let expected_rgba8 = (0xFF << 24) | (0x40 << 16) | (0x80 << 8) | 0xFF;

    for (i, &pixel) in out_pixels[..16].iter().enumerate() {
        assert_eq!(
            pixel, expected_rgba8,
            "Pixel {} mismatch: expected 0x{:08X}, got 0x{:08X}",
            i, expected_rgba8, pixel
        );
    }
}

#[test]
fn test_invalid_block_handling() {
    // All zeros is invalid block mode (modeBits & 0xF == 0)
    let raw_zero_block = [0u8; 16];
    let mut out_pixels = [0u32; 16];
    let ok = decompress_block(&raw_zero_block, &mut out_pixels, 4, 4);
    assert!(!ok, "Zero block should fail gracefully");
}

#[test]
fn test_ffi_null_safety() {
    unsafe {
        assert_eq!(
            ryu_astc_decompress_block(std::ptr::null(), std::ptr::null_mut(), 4, 4),
            -1
        );
        assert_eq!(
            ryu_astc_decode_to_rgba8(
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                4,
                4,
                16,
                16,
                1,
                1,
                1
            ),
            -1
        );
        assert_eq!(
            ryu_astc_decode_to_rgba8_parallel(
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                4,
                4,
                16,
                16,
                1,
                1,
                1
            ),
            -1
        );
        assert_eq!(ryu_astc_query_decompressed_size(-1, 10, 1, 1, 1), 0);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn test_dual_sequential_and_parallel_consistency() {
    // Create an image with 4 void extent blocks (8x8 pixels with 4x4 blocks)
    let mut stream = BitStream128::empty();
    stream.write_bits(0x5FC, 11);
    stream.write_bits(1, 1);
    for _ in 0..4 {
        stream.write_bits(0, 13);
    }
    stream.write_bits(0xAA00, 16);
    stream.write_bits(0xBB00, 16);
    stream.write_bits(0xCC00, 16);
    stream.write_bits(0xFF00, 16);

    let raw_block = stream.to_le_bytes();

    // 4 blocks = 64 bytes total input for an 8x8 texture (2x2 blocks of 4x4)
    let mut input_data = Vec::new();
    for _ in 0..4 {
        input_data.extend_from_slice(&raw_block);
    }

    let out_size = query_decompressed_size(8, 8, 1, 1, 1);
    assert_eq!(out_size, 8 * 8 * 4); // 256 bytes

    let mut output_seq = vec![0u8; out_size];
    let mut output_par = vec![0u8; out_size];

    let ok_seq = decode_astc_to_rgba8(&input_data, &mut output_seq, 4, 4, 8, 8, 1, 1, 1);
    let ok_par = decode_astc_to_rgba8_parallel(&input_data, &mut output_par, 4, 4, 8, 8, 1, 1, 1);

    assert!(ok_seq);
    assert!(ok_par);
    assert_eq!(
        output_seq, output_par,
        "Sequential and parallel output must be bit-identical"
    );
}

#[test]
fn test_invalid_block_yields_zero_texels() {
    // Start from non-zero texels so the zeros must have been written by the decoder.
    let mut out_pixels = [0xDEAD_BEEFu32; 16];
    assert!(!decompress_block(&[0u8; 16], &mut out_pixels, 4, 4));
    assert!(out_pixels.iter().all(|&p| p == ASTC_ERROR_COLOR));
    // Zero, as Ryujinx's C# decoder leaves a failed block.
    assert_eq!(ASTC_ERROR_COLOR, 0);
}

/// Regression: this block's colour integer sequence decodes to more values than the
/// 32-entry colour buffer holds, which used to panic with an out-of-bounds index.
#[test]
fn test_oversized_color_sequence_does_not_panic() {
    let block: [u8; 16] = [
        0x0f, 0xbb, 0x16, 0x3c, 0x20, 0x3a, 0x7a, 0x94, 0x71, 0x75, 0xfd, 0x56, 0x96, 0x97, 0xbe,
        0x4f,
    ];
    let mut out_pixels = [0xDEAD_BEEFu32; 16];
    let ok = decompress_block(&block, &mut out_pixels, 4, 4);
    assert!(!ok, "oversized colour encoding must be rejected");
    assert!(out_pixels.iter().all(|&p| p == ASTC_ERROR_COLOR));
}

struct XorShift64(u64);
impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn test_random_blocks_never_panic() {
    for &(bw, bh) in &[(4usize, 4usize), (8, 8)] {
        let mut rng = XorShift64(0x9E37_79B9_7F4A_7C15 ^ ((bw as u64) << 8 | bh as u64));
        let mut out_pixels = [0u32; 144];
        for _ in 0..100_000 {
            let mut block = [0u8; 16];
            block[..8].copy_from_slice(&rng.next_u64().to_le_bytes());
            block[8..].copy_from_slice(&rng.next_u64().to_le_bytes());
            if !decompress_block(&block, &mut out_pixels, bw, bh) {
                assert!(out_pixels[..bw * bh].iter().all(|&p| p == ASTC_ERROR_COLOR));
            }
        }
    }
}

/// A void-extent LDR block of a single colour (16-bit UNORM channels).
fn void_extent_block(r: u16, g: u16, b: u16, a: u16) -> [u8; 16] {
    let mut stream = BitStream128::empty();
    stream.write_bits(0x5FC, 11);
    stream.write_bits(1, 1);
    for _ in 0..4 {
        stream.write_bits(0x1FFF, 13);
    }
    for c in [r, g, b, a] {
        stream.write_bits(c as u32, 16);
    }
    stream.to_le_bytes()
}

/// 8x8 texture of 4x4 blocks (2x2 grid); block `i` has colour `0x10 * (i + 1)` in R.
fn four_block_texture() -> Vec<u8> {
    (0..4u16)
        .flat_map(|i| void_extent_block(0x1000 * (i + 1), 0x2000, 0x3000, 0xFF00))
        .collect()
}

fn texel(out: &[u8], x: usize, y: usize) -> [u8; 4] {
    out[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4]
        .try_into()
        .unwrap()
}

type ImageDecoder = fn(&[u8], &mut [u8], usize, usize, usize, usize, usize, usize, usize) -> bool;
type FfiImageDecoder = unsafe extern "C" fn(
    *const u8,
    usize,
    *mut u8,
    usize,
    i32,
    i32,
    i32,
    i32,
    i32,
    i32,
    i32,
) -> i32;

/// Sequential and parallel image decoders, Rust API and FFI. Without the `parallel`
/// feature the parallel FFI entry point falls back to the sequential decoder.
fn image_decoders() -> Vec<(ImageDecoder, FfiImageDecoder)> {
    vec![
        (decode_astc_to_rgba8, ryu_astc_decode_to_rgba8),
        #[cfg(feature = "parallel")]
        (
            decode_astc_to_rgba8_parallel,
            ryu_astc_decode_to_rgba8_parallel,
        ),
        #[cfg(not(feature = "parallel"))]
        (decode_astc_to_rgba8, ryu_astc_decode_to_rgba8_parallel),
    ]
}

#[test]
fn test_all_valid_blocks_report_success() {
    let input = four_block_texture();
    for (decode, ffi_decode) in image_decoders() {
        let mut out = vec![0u8; 8 * 8 * 4];
        assert!(decode(&input, &mut out, 4, 4, 8, 8, 1, 1, 1));
        for (i, (bx, by)) in [(0, 0), (4, 0), (0, 4), (4, 4)].into_iter().enumerate() {
            assert_eq!(
                texel(&out, bx, by),
                [0x10 * (i as u8 + 1), 0x20, 0x30, 0xFF]
            );
        }

        let mut ffi_out = vec![0u8; out.len()];
        let status = unsafe {
            ffi_decode(
                input.as_ptr(),
                input.len(),
                ffi_out.as_mut_ptr(),
                ffi_out.len(),
                4,
                4,
                8,
                8,
                1,
                1,
                1,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(ffi_out, out);
    }
}

#[test]
fn test_invalid_block_reports_failure_and_is_zero() {
    let mut input = four_block_texture();
    // Block 2 (bottom-left): all zeros is a reserved block mode.
    input[2 * 16..3 * 16].fill(0);
    let zero = ASTC_ERROR_COLOR.to_le_bytes();

    for (decode, ffi_decode) in image_decoders() {
        // Non-zero start, so the failed block's zeros must come from the decoder.
        let mut out = vec![0xABu8; 8 * 8 * 4];
        assert!(!decode(&input, &mut out, 4, 4, 8, 8, 1, 1, 1));

        for y in 0..8 {
            for x in 0..8 {
                let block = (y / 4) * 2 + x / 4;
                let expected = if block == 2 {
                    zero
                } else {
                    [0x10 * (block as u8 + 1), 0x20, 0x30, 0xFF]
                };
                assert_eq!(texel(&out, x, y), expected, "texel ({x}, {y})");
            }
        }

        let mut ffi_out = vec![0xABu8; out.len()];
        let status = unsafe {
            ffi_decode(
                input.as_ptr(),
                input.len(),
                ffi_out.as_mut_ptr(),
                ffi_out.len(),
                4,
                4,
                8,
                8,
                1,
                1,
                1,
            )
        };
        assert_eq!(status, 1);
        assert_eq!(ffi_out, out);
    }
}

#[test]
fn test_short_buffers_report_failure_without_panicking() {
    let input = four_block_texture();
    for (decode, _) in image_decoders() {
        let mut out = vec![0u8; 8 * 8 * 4 - 1];
        assert!(!decode(&input, &mut out, 4, 4, 8, 8, 1, 1, 1));
        let mut out = vec![0u8; 8 * 8 * 4];
        assert!(!decode(&input[..48], &mut out, 4, 4, 8, 8, 1, 1, 1));
    }
}
