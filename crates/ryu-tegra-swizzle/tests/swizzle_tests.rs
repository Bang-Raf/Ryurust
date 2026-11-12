//! Two-Step Verification Suite for ryu-tegra-swizzle
//!
//! Step 1: Unit, Boundary, Argument & FFI Safety Verification
//! Step 2: Deterministic Differential & Bit-Identical Dual-Execution Verification

use ryu_tegra_swizzle::*;

// =========================================================================
// STEP 1: Unit, Boundary, Argument & FFI Safety Verification
// =========================================================================

#[test]
fn step1_test_query_block_linear_size() {
    let size = query_block_linear_size(1280, 720, 1, 4, 4, 1).unwrap();
    assert!(size >= 1280 * 720 * 4);
    assert_eq!(
        size % GOB_SIZE,
        0,
        "Block-linear size must be multiple of GOB_SIZE (512)"
    );

    // Invalid parameters
    assert_eq!(query_block_linear_size(0, 720, 1, 4, 4, 1), Ok(0));
    assert!(query_block_linear_size(1280, 720, 1, 0, 4, 1).is_err());
    assert!(
        query_block_linear_size(1280, 720, 1, 4, 3, 1).is_err(),
        "Non-power-of-two GOB blocks must fail"
    );
}

#[test]
fn step1_test_ffi_null_and_bounds_safety() {
    // SAFETY: pointers are null (rejected) or point at locals/temporaries that outlive the call
    // and are at least as long as the lengths passed.
    unsafe {
        // Null pointers
        assert_eq!(
            ryu_swizzle_block_linear_to_pitch(
                std::ptr::null_mut(),
                1024,
                [0u8; 1024].as_ptr(),
                1024,
                64,
                64,
                256,
                4,
                2
            ),
            SWIZZLE_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_swizzle_pitch_to_block_linear(
                [0u8; 1024].as_mut_ptr(),
                1024,
                std::ptr::null(),
                1024,
                64,
                64,
                256,
                4,
                2
            ),
            SWIZZLE_STATUS_NULL_POINTER
        );

        // Invalid dimensions
        let mut buf = [0u8; 1024];
        assert_eq!(
            ryu_swizzle_block_linear_to_pitch(
                buf.as_mut_ptr(),
                1024,
                buf.as_ptr(),
                1024,
                -1,
                64,
                256,
                4,
                2
            ),
            SWIZZLE_STATUS_INVALID_ARGUMENT
        );

        // Buffer too small
        assert_eq!(
            ryu_swizzle_block_linear_to_pitch(
                buf.as_mut_ptr(),
                10,
                buf.as_ptr(),
                10,
                64,
                64,
                256,
                4,
                2
            ),
            SWIZZLE_STATUS_BUFFER_TOO_SMALL
        );
    }
}

#[test]
fn step1_test_all_bpp_support() {
    let bpps = [1, 2, 4, 8, 16];
    let (w, h) = (64, 32);

    for &bpp in &bpps {
        let stride = align_up(w * bpp, HOST_STRIDE_ALIGNMENT);
        let linear_size = h * stride;
        let bl_size = query_block_linear_size(w, h, 1, bpp, 2, 1).unwrap();

        let linear = vec![0x5Au8; linear_size];
        let mut bl = vec![0u8; bl_size];
        let mut recovered = vec![0u8; linear_size];

        convert_linear_to_block_linear(&mut bl, &linear, w, h, stride, bpp, 2).unwrap();
        convert_block_linear_to_linear(&mut recovered, &bl, w, h, stride, bpp, 2).unwrap();

        assert_eq!(linear, recovered, "BPP {} failed roundtrip!", bpp);
    }
}

// =========================================================================
// STEP 2: Deterministic Differential & Bit-Identical Dual-Execution Verification
// =========================================================================

fn generate_deterministic_pattern(len: usize) -> Vec<u8> {
    let mut data = vec![0u8; len];
    for (i, byte) in data.iter_mut().enumerate() {
        let val = ((i * 1664525 + 1013904223) >> 16) as u8;
        *byte = val;
    }
    data
}

#[test]
fn step2_test_roundtrip_bit_identical_docked_1080p() {
    let (w, h, bpp, gob_y) = (1920, 1080, 4, 4);
    let stride = align_up(w * bpp, HOST_STRIDE_ALIGNMENT);
    let linear_size = h * stride;
    let bl_size = query_block_linear_size(w, h, 1, bpp, gob_y, 1).unwrap();

    let original_linear = generate_deterministic_pattern(linear_size);
    let mut bl_buffer = vec![0u8; bl_size];
    let mut recovered_linear = vec![0u8; linear_size];

    // Pitch -> BlockLinear -> Pitch
    convert_linear_to_block_linear(&mut bl_buffer, &original_linear, w, h, stride, bpp, gob_y)
        .unwrap();
    convert_block_linear_to_linear(&mut recovered_linear, &bl_buffer, w, h, stride, bpp, gob_y)
        .unwrap();

    assert_eq!(
        original_linear, recovered_linear,
        "1080p Docked roundtrip is NOT bit-identical!"
    );
}

#[test]
fn step2_test_dual_execution_sequential_vs_parallel_bit_identical() {
    let test_cases = [
        (256, 256, 4, 2),   // Small square
        (1280, 720, 4, 4),  // Handheld 720p
        (1920, 1080, 4, 4), // Docked 1080p
        (137, 89, 4, 1),    // Non-power-of-2 dimensions
        (512, 512, 8, 4),   // 8 BPP format
    ];

    for &(w, h, bpp, gob_y) in &test_cases {
        let stride = align_up(w * bpp, HOST_STRIDE_ALIGNMENT);
        let linear_size = h * stride;
        let bl_size = query_block_linear_size(w, h, 1, bpp, gob_y, 1).unwrap();

        let linear_input = generate_deterministic_pattern(linear_size);

        // 1. Dual test: Linear to Block-Linear (Sequential vs Parallel)
        let mut bl_seq = vec![0u8; bl_size];
        let mut bl_par = vec![0u8; bl_size];

        convert_linear_to_block_linear(&mut bl_seq, &linear_input, w, h, stride, bpp, gob_y)
            .unwrap();
        convert_linear_to_block_linear_parallel(
            &mut bl_par,
            &linear_input,
            w,
            h,
            stride,
            bpp,
            gob_y,
        )
        .unwrap();

        assert_eq!(
            bl_seq, bl_par,
            "Linear -> Block-Linear Sequential vs Parallel mismatch on {}x{} bpp={}",
            w, h, bpp
        );

        // 2. Dual test: Block-Linear to Linear (Sequential vs Parallel)
        let mut linear_seq = vec![0u8; linear_size];
        let mut linear_par = vec![0u8; linear_size];

        convert_block_linear_to_linear(&mut linear_seq, &bl_seq, w, h, stride, bpp, gob_y).unwrap();
        convert_block_linear_to_linear_parallel(&mut linear_par, &bl_seq, w, h, stride, bpp, gob_y)
            .unwrap();

        assert_eq!(
            linear_seq, linear_par,
            "Block-Linear -> Linear Sequential vs Parallel mismatch on {}x{} bpp={}",
            w, h, bpp
        );

        // 3. Dual test: Output matches original input bit-for-bit
        assert_eq!(
            linear_input, linear_par,
            "Round-trip Parallel output is NOT bit-identical to original input on {}x{}!",
            w, h
        );
    }
}
