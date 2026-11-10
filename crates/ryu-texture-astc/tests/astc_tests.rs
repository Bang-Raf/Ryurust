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

    for i in 0..16 {
        assert_eq!(
            out_pixels[i], expected_rgba8,
            "Pixel {} mismatch: expected 0x{:08X}, got 0x{:08X}",
            i, expected_rgba8, out_pixels[i]
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
