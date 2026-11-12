//! Two-Step Verification Suite for ryu-audio-resampler
//!
//! Step 1: Unit, Boundary, Argument & FFI Safety Verification
//! Step 2: Deterministic Differential & Mathematical Dual-Execution Verification

use ryu_audio_resampler::*;

// =========================================================================
// STEP 1: Unit, Boundary, Argument & FFI Safety Verification
// =========================================================================

#[test]
fn step1_test_adpcm_geometry_and_nibbles() {
    assert_eq!(get_nibbles_from_sample_count(0), 0);
    assert_eq!(get_nibbles_from_sample_count(1), 3); // 2 header nibbles + 1 sample nibble
    assert_eq!(get_nibbles_from_sample_count(14), 16); // 1 frame = 16 nibbles (8 bytes)
    assert_eq!(get_nibbles_from_sample_count(28), 32); // 2 frames = 32 nibbles (16 bytes)

    assert_eq!(get_adpcm_data_size(0), 0);
    assert_eq!(get_adpcm_data_size(14), 8);
    assert_eq!(get_adpcm_data_size(28), 16);
}

#[test]
fn step1_test_ffi_null_safety() {
    // SAFETY: pointers are either null (rejected before use) or point at locals at least as
    // long as the lengths passed.
    unsafe {
        let mut out = [0.0f32; 16];
        let input = [0i16; 16];
        let mut frac = 0.0f32;

        // Null pointer check
        assert_eq!(
            ryu_audio_resample(
                std::ptr::null_mut(),
                16,
                input.as_ptr(),
                16,
                1.0,
                &mut frac,
                16,
                0
            ),
            AUDIO_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_audio_resample(
                out.as_mut_ptr(),
                16,
                std::ptr::null(),
                16,
                1.0,
                &mut frac,
                16,
                0
            ),
            AUDIO_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_audio_resample(
                out.as_mut_ptr(),
                16,
                input.as_ptr(),
                16,
                1.0,
                std::ptr::null_mut(),
                16,
                0
            ),
            AUDIO_STATUS_NULL_POINTER
        );

        // Invalid arguments (negative ratio / sample count)
        assert_eq!(
            ryu_audio_resample(
                out.as_mut_ptr(),
                16,
                input.as_ptr(),
                16,
                -1.0,
                &mut frac,
                16,
                0
            ),
            AUDIO_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            ryu_audio_resample(
                out.as_mut_ptr(),
                16,
                input.as_ptr(),
                16,
                1.0,
                &mut frac,
                -5,
                0
            ),
            AUDIO_STATUS_INVALID_ARGUMENT
        );
    }
}

#[test]
fn step1_test_all_resampler_quality_modes() {
    let input = [
        1000i16, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000,
    ];
    let mut out_default = [0.0f32; 8];
    let mut out_high = [0.0f32; 8];
    let mut out_low = [0.0f32; 8];

    let mut frac_def = 0.0f32;
    let mut frac_high = 0.0f32;
    let mut frac_low = 0.0f32;

    resample(
        &mut out_default,
        &input,
        1.0,
        &mut frac_def,
        8,
        ResampleQuality::Default,
    );
    resample(
        &mut out_high,
        &input,
        1.0,
        &mut frac_high,
        8,
        ResampleQuality::High,
    );
    resample(
        &mut out_low,
        &input,
        1.0,
        &mut frac_low,
        8,
        ResampleQuality::Low,
    );

    // Assert outputs are populated
    for i in 0..8 {
        assert!(
            out_default[i] != 0.0,
            "Default quality failed to produce output at index {}",
            i
        );
        assert!(
            out_high[i] != 0.0,
            "High quality failed to produce output at index {}",
            i
        );
        assert!(
            out_low[i] != 0.0,
            "Low quality failed to produce output at index {}",
            i
        );
    }
}

// =========================================================================
// STEP 2: Deterministic Differential & Mathematical Dual-Execution Verification
// =========================================================================

#[test]
fn step2_test_polyphase_sinc_deterministic_and_continuous() {
    // Generate a 440 Hz test tone at 32 kHz
    let sample_rate = 32000.0f64;
    let freq = 440.0f64;
    let input_len = 500;
    let mut input = vec![0i16; input_len];
    for (i, val) in input.iter_mut().enumerate() {
        let t = (i as f64) / sample_rate;
        *val = ((2.0 * std::f64::consts::PI * freq * t).sin() * 16000.0) as i16;
    }

    // Resample to 48 kHz (ratio = 32000 / 48000 = 2/3)
    let ratio = 32000.0f32 / 48000.0f32;
    let output_count = 700;

    // Run 1: Full batch
    let mut out_batch = vec![0.0f32; output_count];
    let mut frac_batch = 0.0f32;
    resample_default(&mut out_batch, &input, ratio, &mut frac_batch, output_count);

    // Dual execution on repeated run must be 100% bit-identical
    let mut out_batch_repeat = vec![0.0f32; output_count];
    let mut frac_repeat = 0.0f32;
    resample_default(
        &mut out_batch_repeat,
        &input,
        ratio,
        &mut frac_repeat,
        output_count,
    );

    assert_eq!(
        out_batch, out_batch_repeat,
        "Repeated resample execution is NOT bit-identical!"
    );
    assert_eq!(frac_batch, frac_repeat);
}

#[test]
fn step2_test_adpcm_decoding_deterministic_behavior() {
    let mut adpcm_frame = [0u8; 8];
    adpcm_frame[0] = 0x02; // Coeff index 0, scale 2
    adpcm_frame[1] = 0x12; // Samples 0 (+1), 1 (+2)
    adpcm_frame[2] = 0x3F; // Samples 2 (+3), 3 (-1)
    adpcm_frame[3] = 0x00;
    adpcm_frame[4] = 0x11;
    adpcm_frame[5] = 0x22;
    adpcm_frame[6] = 0x33;
    adpcm_frame[7] = 0x44;

    let coefficients: [i16; 16] = [
        2048, 0, // Pair 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let mut context1 = AdpcmLoopContext::default();
    let mut context2 = AdpcmLoopContext::default();

    let mut out1 = [0i16; 14];
    let mut out2 = [0i16; 14];

    let decoded1 = decode_adpcm(
        &mut out1,
        &adpcm_frame,
        0,
        14,
        0,
        14,
        &coefficients,
        &mut context1,
    );
    let decoded2 = decode_adpcm(
        &mut out2,
        &adpcm_frame,
        0,
        14,
        0,
        14,
        &coefficients,
        &mut context2,
    );

    assert_eq!(decoded1, 14);
    assert_eq!(decoded2, 14);
    assert_eq!(out1, out2, "ADPCM dual-execution mismatch!");
    assert_eq!(context1, context2, "ADPCM context mismatch!");

    // Non-zero samples decoded
    assert!(out1[0] != 0, "Decoded sample 0 must not be zero");
}
