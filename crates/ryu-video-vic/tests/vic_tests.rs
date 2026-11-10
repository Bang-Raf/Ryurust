use ryu_video_vic::blender::blend_surface;
use ryu_video_vic::ffi::*;
use ryu_video_vic::scaler::{deinterlace_bob, deinterlace_motion_adaptive, deinterlace_weave};
use ryu_video_vic::types::{VicColorMatrix, VicPixel, VicRectangle};
use std::mem::size_of;

// =========================================================================
// STEP 1: SAFETY, INVARIANTS, NULL POINTER & BOUNDARY VERIFICATION
// =========================================================================

#[test]
fn test_step1_struct_sizes_and_alignments() {
    assert_eq!(
        size_of::<VicPixel>(),
        8,
        "VicPixel must be 8 bytes (RGBA 16-bit)"
    );
    assert_eq!(
        size_of::<VicRectangle>(),
        16,
        "VicRectangle must be 16 bytes"
    );
    assert_eq!(
        size_of::<VicColorMatrix>(),
        64,
        "VicColorMatrix must be 64 bytes"
    );
}

#[test]
fn test_step1_null_pointer_safety() {
    let rect = VicRectangle::default();
    let matrix = VicColorMatrix::default();

    unsafe {
        assert_eq!(
            ryu_vic_blend_surface(
                std::ptr::null_mut(),
                0,
                10,
                10,
                std::ptr::null(),
                0,
                10,
                10,
                &rect,
                &matrix,
            ),
            VIC_STATUS_NULL_POINTER
        );

        assert_eq!(
            ryu_vic_deinterlace_weave(std::ptr::null_mut(), 0, std::ptr::null(), 0, 10, 10, true),
            VIC_STATUS_NULL_POINTER
        );

        assert_eq!(
            ryu_vic_deinterlace_bob(std::ptr::null_mut(), 0, 10, 10, true),
            VIC_STATUS_NULL_POINTER
        );

        assert_eq!(
            ryu_vic_deinterlace_motion_adaptive(
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                10,
                10,
                true,
            ),
            VIC_STATUS_NULL_POINTER
        );
    }
}

#[test]
fn test_step1_empty_dimensions_handling() {
    let mut dst = [VicPixel::default(); 16];
    let src = [VicPixel::default(); 16];
    let rect = VicRectangle::default();
    let matrix = VicColorMatrix::default();

    unsafe {
        assert_eq!(
            ryu_vic_blend_surface(
                dst.as_mut_ptr(),
                16,
                0,
                0,
                src.as_ptr(),
                16,
                0,
                0,
                &rect,
                &matrix,
            ),
            VIC_STATUS_OK
        );
    }
}

// =========================================================================
// STEP 2: DETERMINISTIC DUAL-EXECUTION & BIT-IDENTICAL VERIFICATION
// =========================================================================

#[test]
fn test_step2_blend_surface_passthrough_bit_identical() {
    const W: usize = 64;
    const H: usize = 64;
    let mut dst = vec![VicPixel::default(); W * H];
    let mut src = vec![VicPixel::default(); W * H];

    for y in 0..H {
        for x in 0..W {
            src[y * W + x] = VicPixel {
                r: ((x * 17) % 2000) as u16,
                g: ((y * 23) % 2000) as u16,
                b: (((x + y) * 31) % 2000) as u16,
                a: 1023,
            };
        }
    }

    let rect = VicRectangle {
        x: 0,
        y: 0,
        width: W as i32,
        height: H as i32,
    };
    let matrix = VicColorMatrix {
        matrix_enable: 0,
        clamp_low: 64,
        clamp_high: 960,
        ..Default::default()
    };

    blend_surface(&mut dst, W, H, &src, W, H, &rect, &matrix);

    for y in 0..H {
        for x in 0..W {
            let sp = src[y * W + x];
            let dp = dst[y * W + x];

            assert_eq!(dp.r, (sp.r as i32).clamp(64, 960) as u16);
            assert_eq!(dp.g, (sp.g as i32).clamp(64, 960) as u16);
            assert_eq!(dp.b, (sp.b as i32).clamp(64, 960) as u16);
            assert_eq!(dp.a, sp.a);
        }
    }
}

#[test]
fn test_step2_blend_surface_color_matrix_bit_identical() {
    const W: usize = 128;
    const H: usize = 128;
    let mut dst = vec![VicPixel::default(); W * H];
    let mut src = vec![VicPixel::default(); W * H];

    for y in 0..H {
        for x in 0..W {
            src[y * W + x] = VicPixel {
                r: ((x * 19) % 1024) as u16,
                g: ((y * 29) % 1024) as u16,
                b: (((x * y) * 7) % 1024) as u16,
                a: 1023,
            };
        }
    }

    // Typical Rec.709 YUV -> RGB color matrix used by Nintendo Switch FMV video engine
    let matrix = VicColorMatrix {
        matrix_enable: 1,
        r_shift: 10,
        coeff00: 1164,
        coeff01: 0,
        coeff02: 1793,
        coeff03: -248000,
        coeff10: 1164,
        coeff11: -213,
        coeff12: -533,
        coeff13: 76800,
        coeff20: 1164,
        coeff21: 2112,
        coeff22: 0,
        coeff23: -289000,
        clamp_low: 0,
        clamp_high: 1023,
    };

    let rect = VicRectangle {
        x: 10,
        y: 10,
        width: 100,
        height: 100,
    };

    blend_surface(&mut dst, W, H, &src, W, H, &rect, &matrix);

    // Reference evaluation matching Blender.MatrixMultiply
    for y in 10..110 {
        for x in 10..110 {
            let sp = src[y * W + x];
            let in_r = sp.r as i32;
            let in_g = sp.g as i32;
            let in_b = sp.b as i32;

            let mut ref_r = in_r * matrix.coeff00 + in_g * matrix.coeff01 + in_b * matrix.coeff02;
            let mut ref_g = in_r * matrix.coeff10 + in_g * matrix.coeff11 + in_b * matrix.coeff12;
            let mut ref_b = in_r * matrix.coeff20 + in_g * matrix.coeff21 + in_b * matrix.coeff22;

            ref_r >>= matrix.r_shift;
            ref_g >>= matrix.r_shift;
            ref_b >>= matrix.r_shift;

            ref_r += matrix.coeff03;
            ref_g += matrix.coeff13;
            ref_b += matrix.coeff23;

            ref_r >>= 8;
            ref_g >>= 8;
            ref_b >>= 8;

            ref_r = ref_r.clamp(matrix.clamp_low, matrix.clamp_high);
            ref_g = ref_g.clamp(matrix.clamp_low, matrix.clamp_high);
            ref_b = ref_b.clamp(matrix.clamp_low, matrix.clamp_high);

            let dp = dst[y * W + x];
            assert_eq!(dp.r, ref_r as u16, "Red mismatch at ({}, {})", x, y);
            assert_eq!(dp.g, ref_g as u16, "Green mismatch at ({}, {})", x, y);
            assert_eq!(dp.b, ref_b as u16, "Blue mismatch at ({}, {})", x, y);
            assert_eq!(dp.a, sp.a, "Alpha mismatch at ({}, {})", x, y);
        }
    }
}

#[test]
fn test_step2_deinterlace_weave_and_bob_deterministic() {
    let width = 64;
    let field_size = 128; // stride
    let total_size = field_size * 4;

    let mut data_weave = vec![0u8; total_size];
    let prev_data: Vec<u8> = (0..total_size).map(|i| (i % 256) as u8).collect();

    deinterlace_weave(&mut data_weave, &prev_data, width, field_size, true);

    // Verify Weave copied alternating rows
    assert_eq!(
        &data_weave[field_size..field_size + width],
        &prev_data[0..width]
    );

    let mut data_bob = vec![0u8; total_size];
    for i in 0..width {
        data_bob[i] = (i * 3) as u8;
    }

    deinterlace_bob(&mut data_bob, width, field_size, true);
    // Verify Bob copied row 0 to row 1
    assert_eq!(
        &data_bob[field_size..field_size + width],
        &data_bob[0..width]
    );
}

#[test]
fn test_step2_deinterlace_motion_adaptive_differential() {
    let width = 64;
    let field_size = 64;
    let total_size = field_size * 4;

    let mut data = vec![100u8; total_size];
    let prev_data = vec![50u8; total_size];
    let mut next_data = vec![50u8; total_size];

    // Introduce motion in second half
    for i in 32..64 {
        next_data[i] = 75; // changed between prev and next -> should trigger Bob!
    }

    deinterlace_motion_adaptive(&mut data, &prev_data, &next_data, width, field_size, true);

    // Pixels 0..32 had no motion: prev == next -> should use Weave (prev_data = 50)
    for i in 0..32 {
        assert_eq!(data[field_size + i], 50, "Weave expected at index {}", i);
    }
    // Pixels 32..64 had motion: prev != next -> should use Bob (other field = 100)
    for i in 32..64 {
        assert_eq!(data[field_size + i], 100, "Bob expected at index {}", i);
    }
}
