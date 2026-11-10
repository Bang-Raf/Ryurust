use ryu_video_nvdec::*;
use std::ptr;

#[test]
fn test_step1_null_pointer_safety() {
    unsafe {
        let dummy = [0u8; 64];
        let idct_in = [0i32; 64];
        let cfg = Vp9FrameConfig {
            width: 16,
            height: 16,
            is_key_frame: 1,
            intra_only: 1,
            filter_level: 10,
            sharpness: 0,
            base_qindex: 100,
            transform_mode: 0,
            lossless: 0,
        };

        // IDCT 4x4
        assert_eq!(
            ryu_nvdec_vp9_idct4x4_add(ptr::null(), dummy.as_ptr() as *mut u8, 4, 1),
            NVDEC_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_nvdec_vp9_idct4x4_add(idct_in.as_ptr(), ptr::null_mut(), 4, 1),
            NVDEC_STATUS_NULL_POINTER
        );

        // IDCT 8x8
        assert_eq!(
            ryu_nvdec_vp9_idct8x8_add(ptr::null(), dummy.as_ptr() as *mut u8, 8, 1),
            NVDEC_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_nvdec_vp9_idct8x8_add(idct_in.as_ptr(), ptr::null_mut(), 8, 1),
            NVDEC_STATUS_NULL_POINTER
        );

        // IDCT 16x16
        assert_eq!(
            ryu_nvdec_vp9_idct16x16_add(ptr::null(), dummy.as_ptr() as *mut u8, 16, 1),
            NVDEC_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_nvdec_vp9_idct16x16_add(idct_in.as_ptr(), ptr::null_mut(), 16, 1),
            NVDEC_STATUS_NULL_POINTER
        );

        // IDCT 32x32
        assert_eq!(
            ryu_nvdec_vp9_idct32x32_add(ptr::null(), dummy.as_ptr() as *mut u8, 32, 1),
            NVDEC_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_nvdec_vp9_idct32x32_add(idct_in.as_ptr(), ptr::null_mut(), 32, 1),
            NVDEC_STATUS_NULL_POINTER
        );

        // Loop Filter Edge
        assert_eq!(
            ryu_nvdec_vp9_loop_filter_edge(ptr::null_mut(), 64, 8, 8, 10, 5, 2, 8, 0),
            NVDEC_STATUS_NULL_POINTER
        );

        // Loop Filter Frame
        assert_eq!(
            ryu_nvdec_vp9_loop_filter_frame(
                ptr::null_mut(),
                256,
                dummy.as_ptr() as *mut u8,
                64,
                dummy.as_ptr() as *mut u8,
                64,
                16,
                16,
                16,
                8,
                10,
                0
            ),
            NVDEC_STATUS_NULL_POINTER
        );

        // Decode Frame
        assert_eq!(
            ryu_nvdec_decode_vp9_frame(
                ptr::null(),
                10,
                &cfg,
                dummy.as_ptr() as *mut u8,
                256,
                dummy.as_ptr() as *mut u8,
                64,
                dummy.as_ptr() as *mut u8,
                64,
                16,
                8
            ),
            NVDEC_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_nvdec_decode_vp9_frame(
                dummy.as_ptr(),
                10,
                ptr::null(),
                dummy.as_ptr() as *mut u8,
                256,
                dummy.as_ptr() as *mut u8,
                64,
                dummy.as_ptr() as *mut u8,
                64,
                16,
                8
            ),
            NVDEC_STATUS_NULL_POINTER
        );
    }
}

#[test]
fn test_step1_boundary_and_invariants() {
    unsafe {
        let mut dummy = [0u8; 128];
        let idct_in = [0i32; 64];

        // Invalid stride
        assert_eq!(
            ryu_nvdec_vp9_idct4x4_add(idct_in.as_ptr(), dummy.as_mut_ptr(), 2, 1),
            NVDEC_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            ryu_nvdec_vp9_idct8x8_add(idct_in.as_ptr(), dummy.as_mut_ptr(), 4, 1),
            NVDEC_STATUS_INVALID_ARGUMENT
        );

        // Invalid transform size
        assert_eq!(
            ryu_nvdec_vp9_loop_filter_edge(dummy.as_mut_ptr(), 128, 16, 8, 10, 5, 2, 7, 0),
            NVDEC_STATUS_INVALID_ARGUMENT
        );

        // Struct alignment
        assert_eq!(std::mem::size_of::<Vp9FrameConfig>(), 36);
        assert_eq!(ryu_nvdec_get_version(), 0x00010000);
    }
}

#[test]
fn test_step2_idct4x4_exact_math() {
    let mut dest_dc = [128u8; 16];
    let mut dest_full = [128u8; 16];

    let mut coeffs = [0i32; 16];
    coeffs[0] = 64; // Pure DC coefficient

    idct4x4_1_add(&coeffs, &mut dest_dc, 4);
    idct4x4_16_add(&coeffs, &mut dest_full, 4);

    // DC-only path and full 16-coeff path MUST produce 100% bit-identical output!
    assert_eq!(dest_dc, dest_full);

    // Verify all pixels in block shifted by the exact calculated DC offset
    let expected = dest_dc[0];
    for &p in dest_dc.iter() {
        assert_eq!(p, expected);
    }
}

#[test]
fn test_step2_idct8x8_exact_math() {
    let mut dest_dc = [128u8; 64];
    let mut dest_full = [128u8; 64];

    let mut coeffs = [0i32; 64];
    coeffs[0] = 120; // Pure DC coefficient

    idct8x8_1_add(&coeffs, &mut dest_dc, 8);
    idct8x8_64_add(&coeffs, &mut dest_full, 8);

    // Bit-identical verification
    assert_eq!(dest_dc, dest_full);
}

#[test]
fn test_step2_idct16x16_and_32x32_exact_math() {
    let mut dest_dc16 = [128u8; 256];
    let mut dest_full16 = [128u8; 256];

    let mut coeffs16 = [0i32; 256];
    coeffs16[0] = 200;

    idct16x16_1_add(&coeffs16, &mut dest_dc16, 16);
    idct16x16_256_add(&coeffs16, &mut dest_full16, 16);
    assert_eq!(dest_dc16, dest_full16);

    let mut dest_dc32 = [128u8; 1024];
    let mut dest_full32 = [128u8; 1024];

    let mut coeffs32 = [0i32; 1024];
    coeffs32[0] = 300;

    idct32x32_1_add(&coeffs32, &mut dest_dc32, 32);
    idct32x32_1024_add(&coeffs32, &mut dest_full32, 32);
    assert_eq!(dest_dc32, dest_full32);
}

#[test]
fn test_step2_hybrid_transform_iht() {
    let mut coeffs = [0i32; 16];
    coeffs[0] = 100;
    coeffs[1] = 25;
    coeffs[4] = -15;

    let mut dest_dct = [128u8; 16];
    let mut dest_adst = [128u8; 16];

    iht4x4_16_add(&coeffs, &mut dest_dct, 4, 0); // DCT_DCT
    iht4x4_16_add(&coeffs, &mut dest_adst, 4, 3); // ADST_ADST

    // Results must be deterministic and valid
    assert_ne!(dest_dct, dest_adst);
    assert!(dest_dct[0] > 0);
    assert!(dest_adst[0] > 0);
}

#[test]
fn test_step2_loop_filter_deblocking() {
    // Construct a blocking step discontinuity of 15 (120 vs 135)
    let pitch = 8;
    let mut buffer = [120u8; 64];
    for r in 4..8 {
        for c in 0..8 {
            buffer[r * pitch + c] = 135;
        }
    }

    let original = buffer;
    let (blimit, limit, thresh) = compute_loop_filter_limits(20, 0);
    lpf_horizontal_8(&mut buffer, 4 * pitch, pitch, blimit, limit, thresh);

    // The boundary row 3 (was 120) and row 4 (was 135) must be smoothed!
    assert!(
        buffer[3 * pitch] > original[3 * pitch],
        "Row 3 should be smoothed upwards (from 120)"
    );
    assert!(
        buffer[4 * pitch] < original[4 * pitch],
        "Row 4 should be smoothed downwards (from 135)"
    );
}

#[test]
fn test_step2_bool_decoder_entropy() {
    // Simple bitstream: 0x80, 0x00, 0xFF, 0x55
    let data = [0x80, 0x00, 0xFF, 0x55, 0xAA];
    let mut decoder = BoolDecoder::new(&data);

    let mut bits = Vec::new();
    for _ in 0..16 {
        bits.push(decoder.read_bit(128));
    }

    assert_eq!(bits.len(), 16);
    // Repeatability test: reading same bitstream produces identical bits
    let mut decoder2 = BoolDecoder::new(&data);
    let mut bits2 = Vec::new();
    for _ in 0..16 {
        bits2.push(decoder2.read_bit(128));
    }
    assert_eq!(bits, bits2);
}

#[test]
fn test_step2_end_to_end_frame_decode() {
    let config = Vp9FrameConfig {
        width: 64,
        height: 64,
        is_key_frame: 1,
        intra_only: 1,
        filter_level: 16,
        sharpness: 1,
        base_qindex: 80,
        transform_mode: 1,
        lossless: 0,
    };

    let mut y = vec![0u8; 64 * 64];
    let mut u = vec![0u8; 32 * 32];
    let mut v = vec![0u8; 32 * 32];

    let bitstream = [
        0x82, 0x49, 0x83, 0x42, 0x3F, 0x00, 0x3F, 0x00, 0x00, 0xFF, 0x80, 0x40, 0x20,
    ];

    // Decode frame
    let res = Vp9DecoderPipeline::decode_frame(&bitstream, &config, &mut y, &mut u, &mut v, 64, 32);
    assert!(res.is_ok());

    // Dual-execution repeatability: second run on identical input
    let mut y2 = vec![0u8; 64 * 64];
    let mut u2 = vec![0u8; 32 * 32];
    let mut v2 = vec![0u8; 32 * 32];
    let res2 =
        Vp9DecoderPipeline::decode_frame(&bitstream, &config, &mut y2, &mut u2, &mut v2, 64, 32);
    assert!(res2.is_ok());

    assert_eq!(
        y, y2,
        "Y plane must be 100% bit-identical across successive runs"
    );
    assert_eq!(
        u, u2,
        "U plane must be 100% bit-identical across successive runs"
    );
    assert_eq!(
        v, v2,
        "V plane must be 100% bit-identical across successive runs"
    );
}

// =========================================================================
// IDCT 32x32 ZERO-ROW CHECK AND 1-D IDCT32 ACCURACY
// =========================================================================

#[test]
fn test_row_is_zero_uses_full_i32() {
    assert!(row_is_zero(&[0i32; 32]));
    let mut row = [0i32; 32];
    row[5] = 65536; // `as i16` would make this 0
    assert!(!row_is_zero(&row));
    row[5] = -131072;
    assert!(!row_is_zero(&row));
    row[5] = 1 << 31;
    assert!(!row_is_zero(&row));
    row[5] = 0;
    row[31] = 1;
    assert!(!row_is_zero(&row));
}

/// Reference 32x32 IDCT-add that never skips a row.
fn reference_idct32x32_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let mut inter = [0i32; 1024];
    for i in 0..32 {
        idct32(
            &input[i * 32..(i + 1) * 32],
            &mut inter[i * 32..(i + 1) * 32],
        );
    }
    let mut col_in = [0i32; 32];
    let mut col_out = [0i32; 32];
    for i in 0..32 {
        for j in 0..32 {
            col_in[j] = inter[j * 32 + i];
        }
        idct32(&col_in, &mut col_out);
        for j in 0..32 {
            let p = dest[j * stride + i] as i32 + ((col_out[j] + 32) >> 6);
            dest[j * stride + i] = p.clamp(0, 255) as u8;
        }
    }
}

#[test]
fn test_idct32x32_rows_with_multiple_of_65536_coefficients() {
    // Rows whose only non-zero coefficients are multiples of 65536 must not be skipped by the
    // zero-row shortcut; the result must equal the no-shortcut reference.
    let mut coeffs = [0i32; 1024];
    coeffs[0] = 300;
    coeffs[3 * 32 + 7] = 65536;
    coeffs[9 * 32] = 65536 + 40; // low 16 bits non-zero too
    coeffs[17 * 32 + 1] = -65536 * 3;
    coeffs[31 * 32 + 31] = 25;

    let mut got = [100u8; 1024];
    let mut expected = [100u8; 1024];
    idct32x32_1024_add(&coeffs, &mut got, 32);
    reference_idct32x32_add(&coeffs, &mut expected, 32);
    assert_eq!(got.to_vec(), expected.to_vec());
}

#[test]
fn test_idct32_matches_float_reference() {
    // VP9 1-D IDCT: out[n] = X0 / sqrt(2) + sum_{k>=1} X[k] * cos(pi * (2n + 1) * k / 64).
    let mut seed = 0x1234_5678u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for trial in 0..200 {
        let mut input = [0i32; 32];
        if trial < 32 {
            input[trial] = 1000; // single-frequency basis vectors
        } else {
            for c in input.iter_mut() {
                *c = (next() % 801) as i32 - 400;
            }
        }
        let mut out = [0i32; 32];
        idct32(&input, &mut out);
        for (n, &o) in out.iter().enumerate() {
            let mut r = input[0] as f64 / std::f64::consts::SQRT_2;
            for (k, &x) in input.iter().enumerate().skip(1) {
                r += x as f64 * (std::f64::consts::PI * (2 * n + 1) as f64 * k as f64 / 64.0).cos();
            }
            assert!(
                (o as f64 - r).abs() <= 8.0,
                "trial {trial} n {n}: got {o}, reference {r:.2}"
            );
        }
    }
}

#[test]
fn test_decode_frame_reads_separate_u_and_v_dc() {
    let config = Vp9FrameConfig {
        width: 64,
        height: 64,
        is_key_frame: 1,
        intra_only: 1,
        filter_level: 0,
        sharpness: 0,
        base_qindex: 80,
        transform_mode: 1,
        lossless: 0,
    };
    let mut seed = 0xdead_beefu32;
    let bitstream: Vec<u8> = (0..512)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let mut y = vec![0u8; 64 * 64];
    let mut u = vec![0u8; 32 * 32];
    let mut v = vec![0u8; 32 * 32];
    Vp9DecoderPipeline::decode_frame(&bitstream, &config, &mut y, &mut u, &mut v, 64, 32).unwrap();
    // Previously U and V were reconstructed from the same (luma) DC and were always equal.
    assert!(
        u.iter().any(|&p| p != 128),
        "some chroma block must be coded"
    );
    assert_ne!(u, v, "U and V must come from separate DC values");
    // Each coded 8x8 chroma block is flat (DC only), so U and V differ block-wise.
    let differing_blocks = (0..4)
        .flat_map(|by| (0..4).map(move |bx| (bx, by)))
        .filter(|&(bx, by)| u[by * 8 * 32 + bx * 8] != v[by * 8 * 32 + bx * 8])
        .count();
    assert!(differing_blocks > 0);
}
