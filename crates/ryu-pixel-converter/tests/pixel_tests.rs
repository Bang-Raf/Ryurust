use ryu_pixel_converter::ffi::*;
use ryu_pixel_converter::*;

// Reference C# implementations for dual-execution verification
fn ref_convert_r5g6b5(packed: u16) -> u32 {
    let p = packed as u32;
    let mut out = 0xff000000;
    out |= (p << 3) & 0x000000f8;
    out |= (p << 8) & 0x00f80000;
    out |= (out >> 5) & 0x00070007;
    out |= ((p << 5) & 0x0000fc00) | ((p >> 1) & 0x00000300);
    out
}

fn ref_convert_r5g5b5(packed: u16, force_alpha: bool) -> u32 {
    let p = packed as u32;
    let a = if force_alpha { 1 } else { p >> 15 };
    let mut out = a * 0xff000000;
    out |= (p << 3) & 0x000000f8;
    out |= (p << 6) & 0x0000f800;
    out |= (p << 9) & 0x00f80000;
    out |= (out >> 5) & 0x00070707;
    out
}

fn ref_convert_a1b5g5r5(packed: u16) -> u32 {
    let p = packed as u32;
    let a = p >> 15;
    let mut out = a * 0xff000000;
    out |= (p >> 8) & 0x000000f8;
    out |= (p << 5) & 0x0000f800;
    out |= (p << 18) & 0x00f80000;
    out |= (out >> 5) & 0x00070707;
    out
}

fn ref_convert_r4g4b4a4(packed: u16) -> u32 {
    let p = packed as u32;
    let mut out = p & 0x0000000f;
    out |= (p << 4) & 0x00000f00;
    out |= (p << 8) & 0x000f0000;
    out |= (p << 12) & 0x0f000000;
    out.wrapping_mul(0x11)
}

// =========================================================================
// STEP 1: Invariants, Boundary Checks, Null Pointer & Alignment Safety
// =========================================================================

#[test]
fn test_step1_null_and_zero_bounds() {
    unsafe {
        let mut buf = [0u8; 64];
        // Null src
        assert_eq!(
            ryu_pixel_convert_r5g6b5_to_rgba8(std::ptr::null(), 32, buf.as_mut_ptr(), 64, 4),
            RYU_PIXEL_ERR_NULL_PTR
        );
        // Null dst
        assert_eq!(
            ryu_pixel_convert_r5g6b5_to_rgba8(buf.as_ptr(), 32, std::ptr::null_mut(), 64, 4),
            RYU_PIXEL_ERR_NULL_PTR
        );
        // Zero width (safe no-op)
        assert_eq!(
            ryu_pixel_convert_r5g6b5_to_rgba8(buf.as_ptr(), 32, buf.as_mut_ptr(), 64, 0),
            RYU_PIXEL_SUCCESS
        );
        // Destination buffer too small
        assert_eq!(
            ryu_pixel_convert_r5g6b5_to_rgba8(buf.as_ptr(), 32, buf.as_mut_ptr(), 16, 4),
            RYU_PIXEL_ERR_BUFFER_TOO_SMALL
        );
    }
}

#[test]
fn test_step1_stride_alignment_calc() {
    // Check that host alignment 4 is respected
    // width=3, bpp=1 -> stride = align_up(3, 4) = 4, remainder = 1
    let (rem, out_rem, height) = get_line_remainders(16, 3, 1, 2);
    assert_eq!(rem, 1);
    assert_eq!(out_rem, 1); // out_stride = align_up(6, 4) = 8, out_remainder = 8/2 - 3 = 1
    assert_eq!(height, 4);
}

// =========================================================================
// STEP 2: Deterministic Dual-Execution & Bit-Identical Mathematical Verification
// =========================================================================

#[test]
fn test_step2_convert_r4g4_to_r4g4b4a4() {
    let src = vec![0x12, 0x34, 0xAB, 0xCD, 0x00, 0xFF, 0x55, 0xAA];
    let mut dst = vec![0u8; src.len() * 2];

    convert_r4g4_to_r4g4b4a4(&src, &mut dst, 8).unwrap();
    let dst16: &[u16] =
        unsafe { std::slice::from_raw_parts(dst.as_ptr() as *const u16, src.len()) };

    for i in 0..src.len() {
        assert_eq!(dst16[i], src[i] as u16, "R4G4 expansion mismatch at {}", i);
    }
}

#[test]
fn test_step2_convert_r5g6b5_all_bit_identical() {
    // Test all 65536 possible 16-bit values against reference
    for val in 0..=u16::MAX {
        let rust_val = convert_r5g6b5_pixel(val);
        let csharp_ref = ref_convert_r5g6b5(val);
        assert_eq!(rust_val, csharp_ref, "R5G6B5 mismatch for 0x{:04X}", val);
    }
}

#[test]
fn test_step2_convert_r5g5b5_all_bit_identical() {
    for val in 0..=u16::MAX {
        // Test with force_alpha = false
        let rust_val = convert_r5g5b5_pixel(val, false);
        let csharp_ref = ref_convert_r5g5b5(val, false);
        assert_eq!(
            rust_val, csharp_ref,
            "R5G5B5 (force=false) mismatch for 0x{:04X}",
            val
        );

        // Test with force_alpha = true
        let rust_val_forced = convert_r5g5b5_pixel(val, true);
        let csharp_ref_forced = ref_convert_r5g5b5(val, true);
        assert_eq!(
            rust_val_forced, csharp_ref_forced,
            "R5G5B5 (force=true) mismatch for 0x{:04X}",
            val
        );
    }
}

#[test]
fn test_step2_convert_a1b5g5r5_all_bit_identical() {
    for val in 0..=u16::MAX {
        let rust_val = convert_a1b5g5r5_pixel(val);
        let csharp_ref = ref_convert_a1b5g5r5(val);
        assert_eq!(rust_val, csharp_ref, "A1B5G5R5 mismatch for 0x{:04X}", val);
    }
}

#[test]
fn test_step2_convert_r4g4b4a4_all_bit_identical() {
    for val in 0..=u16::MAX {
        let rust_val = convert_r4g4b4a4_pixel(val);
        let csharp_ref = ref_convert_r4g4b4a4(val);
        assert_eq!(rust_val, csharp_ref, "R4G4B4A4 mismatch for 0x{:04X}", val);
    }
}

#[test]
fn test_step2_convert_bgra8_to_rgba8() {
    let src32 = [0xFF0000AAu32, 0x11223344u32, 0xAABBCCDDu32, 0x00FF00FFu32];
    let src_bytes: &[u8] = unsafe { std::slice::from_raw_parts(src32.as_ptr() as *const u8, 16) };
    let mut dst_bytes = vec![0u8; 16];
    convert_bgra8_to_rgba8(src_bytes, &mut dst_bytes).unwrap();
    let dst32: &[u32] = unsafe { std::slice::from_raw_parts(dst_bytes.as_ptr() as *const u32, 4) };

    for i in 0..4 {
        let expected = convert_bgra8_pixel(src32[i]);
        assert_eq!(dst32[i], expected);
    }
}

#[test]
fn test_step2_odd_widths_and_remainders() {
    for &width in &[1, 2, 3, 5, 7, 9, 13] {
        let height = 4;
        let stride = align_up(width * 2, 4);
        let src_len = stride * height;
        let src_data: Vec<u8> = (0..src_len).map(|i| (i * 17) as u8).collect();
        let mut dst_data = vec![0u8; src_len * 2];

        assert!(convert_r5g6b5_to_r8g8b8a8(&src_data, &mut dst_data, width).is_ok());
        assert!(convert_r5g5b5_to_r8g8b8a8(&src_data, &mut dst_data, width, true).is_ok());
        assert!(convert_a1b5g5r5_to_r8g8b8a8(&src_data, &mut dst_data, width).is_ok());
        assert!(convert_r4g4b4a4_to_r8g8b8a8(&src_data, &mut dst_data, width).is_ok());
    }
}
