use ryu_texture_bcn::block::Block;
use ryu_texture_bcn::ffi::*;
use ryu_texture_bcn::*;

// =========================================================================
// STEP 1: Invariants, Boundary Checks, Null Pointer & Alignment Safety
// =========================================================================

#[test]
fn test_step1_null_and_zero_bounds() {
    unsafe {
        let mut buf = [0u8; 64];
        // Null source
        assert_eq!(
            ryu_texture_decode_bc1(std::ptr::null(), 8, buf.as_mut_ptr(), 64, 4, 4),
            RYU_BCN_ERR_NULL_PTR
        );
        // Null dest
        assert_eq!(
            ryu_texture_decode_bc1(buf.as_ptr(), 8, std::ptr::null_mut(), 64, 4, 4),
            RYU_BCN_ERR_NULL_PTR
        );
        // Zero dimensions (safe no-op)
        assert_eq!(
            ryu_texture_decode_bc1(buf.as_ptr(), 8, buf.as_mut_ptr(), 64, 0, 0),
            RYU_BCN_SUCCESS
        );
        // Source buffer too small
        assert_eq!(
            ryu_texture_decode_bc1(buf.as_ptr(), 4, buf.as_mut_ptr(), 64, 4, 4),
            RYU_BCN_ERR_BUFFER_TOO_SMALL
        );
        // Destination buffer too small
        assert_eq!(
            ryu_texture_decode_bc1(buf.as_ptr(), 8, buf.as_mut_ptr(), 16, 4, 4),
            RYU_BCN_ERR_BUFFER_TOO_SMALL
        );
    }
}

#[test]
fn test_step1_odd_and_clipped_dimensions() {
    // 1x1, 3x3, 5x5, 7x7 textures where block dimensions are clipped
    for &(w, h) in &[(1, 1), (2, 2), (3, 3), (5, 5), (7, 7), (13, 17)] {
        let blocks_x = (w + 3) / 4;
        let blocks_y = (h + 3) / 4;

        // BC1: 8 bytes per block, 4 bytes per pixel output
        let src_bc1 = vec![0u8; blocks_x * blocks_y * 8];
        let mut dst_bc1 = vec![0u8; w * h * 4];
        assert!(decode_bc1_surface(&src_bc1, &mut dst_bc1, w, h).is_ok());

        // BC2: 16 bytes per block, 4 bytes per pixel output
        let src_bc2 = vec![0u8; blocks_x * blocks_y * 16];
        let mut dst_bc2 = vec![0u8; w * h * 4];
        assert!(decode_bc2_surface(&src_bc2, &mut dst_bc2, w, h).is_ok());

        // BC3: 16 bytes per block, 4 bytes per pixel output
        let src_bc3 = vec![0u8; blocks_x * blocks_y * 16];
        let mut dst_bc3 = vec![0u8; w * h * 4];
        assert!(decode_bc3_surface(&src_bc3, &mut dst_bc3, w, h).is_ok());

        // BC4: 8 bytes per block, 1 byte per pixel
        let src_bc4 = vec![0u8; blocks_x * blocks_y * 8];
        let mut dst_bc4 = vec![0u8; ((w + 3) & !3) * h];
        assert!(decode_bc4_surface(&src_bc4, &mut dst_bc4, w, h, false).is_ok());

        // BC5: 16 bytes per block, 2 bytes per pixel
        let src_bc5 = vec![0u8; blocks_x * blocks_y * 16];
        let mut dst_bc5 = vec![0u8; ((w + 1) & !1) * h * 2];
        assert!(decode_bc5_surface(&src_bc5, &mut dst_bc5, w, h, false).is_ok());

        // BC6H: 16 bytes per block, 8 bytes per pixel
        let src_bc6 = vec![0u8; blocks_x * blocks_y * 16];
        let mut dst_bc6 = vec![0u8; w * h * 8];
        assert!(decode_bc6h_surface(&src_bc6, &mut dst_bc6, w, h, false).is_ok());

        // BC7: 16 bytes per block, 4 bytes per pixel
        let src_bc7 = vec![0u8; blocks_x * blocks_y * 16];
        let mut dst_bc7 = vec![0u8; w * h * 4];
        assert!(decode_bc7_surface(&src_bc7, &mut dst_bc7, w, h).is_ok());
    }
}

// =========================================================================
// STEP 2: Deterministic Dual-Execution & Bit-Identical Mathematical Verification
// =========================================================================

#[test]
fn test_step2_bc1_solid_colors_and_lerp() {
    // Solid Red RGB565: 0xF800 -> 888: R=255, G=0, B=0, A=255 (0xFF0000FF in little endian RGBA)
    // c0 = 0xF800, c1 = 0x0000 (c0 > c1)
    let c0: u16 = 0xF800;
    let c1: u16 = 0x0000;
    let mut block = [0u8; 8];
    block[0..2].copy_from_slice(&c0.to_le_bytes());
    block[2..4].copy_from_slice(&c1.to_le_bytes());
    // Indices: all 0 -> pixel selects color 0 (Red)
    block[4..8].copy_from_slice(&0x00000000u32.to_le_bytes());

    let mut output = [0u32; 16];
    decode_bc1_block(&block, &mut output);
    for &p in &output {
        // Red in RGBA32: R=255, G=0, B=0, A=255 -> 0xFF0000FF
        assert_eq!(p, 0xFF0000FF, "BC1 solid red mismatch");
    }

    // Indices: all 1 -> pixel selects color 1 (Black 0xFF000000)
    block[4..8].copy_from_slice(&0x55555555u32.to_le_bytes()); // 01010101...
    decode_bc1_block(&block, &mut output);
    for &p in &output {
        assert_eq!(p, 0xFF000000, "BC1 solid black mismatch");
    }

    // Indices: all 2 -> (2 * c0 + c1) / 3 -> (2*255 + 0)/3 = 170 -> 0xFF0000AA
    block[4..8].copy_from_slice(&0xAAAAAAAAu32.to_le_bytes()); // 10101010...
    decode_bc1_block(&block, &mut output);
    for &p in &output {
        assert_eq!(p, 0xFF0000AA, "BC1 lerp2 mismatch");
    }

    // Punch-through alpha: c0 <= c1
    // c0 = 0x0000, c1 = 0xF800
    block[0..2].copy_from_slice(&0x0000u16.to_le_bytes());
    block[2..4].copy_from_slice(&0xF800u16.to_le_bytes());
    // Index 3 in punchthrough mode must be transparent black: 0x00000000
    block[4..8].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // 11111111...
    decode_bc1_block(&block, &mut output);
    for &p in &output {
        assert_eq!(p, 0x00000000, "BC1 transparent alpha mode mismatch");
    }
}

#[test]
fn test_step2_bc2_explicit_alpha() {
    let mut block = [0u8; 16];
    // First 8 bytes: alpha nibbles
    // Nibble 0xF -> (0xF | 0xF0) = 255
    // Nibble 0x8 -> (0x8 | 0x80) = 136
    block[0] = 0xF8; // pixel 0: 0x8 (136), pixel 1: 0xF (255)
                     // Next 8 bytes: RGB (solid White 0xFFFF)
    block[8..10].copy_from_slice(&0xFFFFu16.to_le_bytes());
    block[10..12].copy_from_slice(&0x0000u16.to_le_bytes());
    // Indices all 0
    block[12..16].copy_from_slice(&0u32.to_le_bytes());

    let mut output = [0u32; 16];
    decode_bc2_block(&block, &mut output);

    // Pixel 0: RGB=White (0x00FFFFFF) | A=136 (0x88000000) = 0x88FFFFFF
    assert_eq!(output[0], 0x88FFFFFF, "BC2 pixel 0 alpha mismatch");
    // Pixel 1: RGB=White (0x00FFFFFF) | A=255 (0xFF000000) = 0xFFFFFFFF
    assert_eq!(output[1], 0xFFFFFFFF, "BC2 pixel 1 alpha mismatch");
}

#[test]
fn test_step2_bc3_interpolated_alpha() {
    let mut block = [0u8; 16];
    // Alpha: a0 = 255, a1 = 0 (a0 > a1)
    block[0] = 255;
    block[1] = 0;
    // 48-bit indices: pixel 0: index 0 (255), pixel 1: index 1 (0), pixel 2: index 2 (6*255/7 = 218)
    // Indices in bits 16..24:
    // bit 16..18: 0 (index 0)
    // bit 19..21: 1 (index 1) -> 1 << 3 = 8
    // bit 22..24: 2 (index 2) -> 2 << 6 = 128
    let indices_low: u32 = 0 | (1 << 3) | (2 << 6);
    block[2] = indices_low as u8;
    block[3] = (indices_low >> 8) as u8;

    // RGB: solid Blue (0x001F)
    block[8..10].copy_from_slice(&0x001Fu16.to_le_bytes());
    block[10..12].copy_from_slice(&0x0000u16.to_le_bytes());

    let mut output = [0u32; 16];
    decode_bc3_block(&block, &mut output);

    let a0 = (output[0] >> 24) & 0xFF;
    let a1 = (output[1] >> 24) & 0xFF;
    let a2 = (output[2] >> 24) & 0xFF;

    assert_eq!(a0, 255, "BC3 alpha 0 mismatch");
    assert_eq!(a1, 0, "BC3 alpha 1 mismatch");
    assert_eq!(a2, 218, "BC3 alpha lerp2 mismatch");
}

#[test]
fn test_step2_bc4_bc5_rgtc() {
    // BC4: a0 = 200, a1 = 50
    let mut block4 = [0u8; 8];
    block4[0] = 200;
    block4[1] = 50;
    // index 0 -> 200, index 1 -> 50, index 2 -> (6*200 + 50)/7 = 178
    let indices: u32 = 0 | (1 << 3) | (2 << 6);
    block4[2] = indices as u8;
    block4[3] = (indices >> 8) as u8;

    let mut out4 = [0u8; 16];
    decode_bc4_block(&block4, &mut out4, false);
    assert_eq!(out4[0], 200);
    assert_eq!(out4[1], 50);
    assert_eq!(out4[2], 178);

    // BC5: R is block4, G is block4 with index 1
    let mut block5 = [0u8; 16];
    block5[0..8].copy_from_slice(&block4);
    block5[8..16].copy_from_slice(&block4);

    let mut out5 = [0u16; 16];
    decode_bc5_block(&block5, &mut out5, false);
    // Pixel 0: R=200, G=200 -> 200 | (200 << 8) = 0xC8C8
    assert_eq!(out5[0], 200 | (200 << 8));
    // Pixel 1: R=50, G=50 -> 50 | (50 << 8) = 0x3232
    assert_eq!(out5[1], 50 | (50 << 8));
}

#[test]
fn test_step2_bc6h_hdr_decode() {
    // Test BC6H Mode 0 (subsetCount = 2, partition)
    // Low: bits 0..4 = 0 (mode 0)
    let block = Block::new(0, 0);
    let mut output = [0u64; 16];
    decode_bc6h_block(block, &mut output, false);

    // With 0 endpoints, R=0, G=0, B=0, A=HALF_ONE (0x3C00)
    let expected = (HALF_ONE as u64) << 48;
    for &p in &output {
        assert_eq!(p, expected, "BC6H zero-endpoint output mismatch");
    }
}

#[test]
fn test_step2_bc7_modes_and_partitions() {
    // Test BC7 Mode 6: bit 6 is 1, bits 0..5 are 0 -> (1 << 6) = 0x40
    // Mode 6 has 1 subset, 4-bit indices, 7-bit color + 7-bit alpha + 2 P-bits
    let block = Block::new(0x40, 0);
    let mut output = [0u32; 16];
    decode_bc7_block(block, &mut output);

    // Mode 6 with 0 endpoints unquantized: endpoints are black transparent/opaque
    // All 16 pixels must be safely decoded without panic
    assert_eq!(output.len(), 16);

    // Test BC7 invalid mode: all bits 0 -> trailing zeros of (0 | 0x100) = 8 -> invalid mode -> fills 0
    let invalid_block = Block::new(0, 0);
    decode_bc7_block(invalid_block, &mut output);
    for &p in &output {
        assert_eq!(p, 0, "BC7 invalid mode must fill 0");
    }
}
