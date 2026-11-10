use crate::ffi::VIC_STATUS_INVALID_ARGUMENT;
use crate::types::{VicColorMatrix, VicPixel, VicRectangle};

#[inline(always)]
fn transform_pixel(src_px: VicPixel, matrix: &VicColorMatrix) -> VicPixel {
    if matrix.matrix_enable != 0 {
        let in_r = src_px.r as i32;
        let in_g = src_px.g as i32;
        let in_b = src_px.b as i32;

        let mut r = in_r * matrix.coeff00 + in_g * matrix.coeff01 + in_b * matrix.coeff02;
        let mut g = in_r * matrix.coeff10 + in_g * matrix.coeff11 + in_b * matrix.coeff12;
        let mut b = in_r * matrix.coeff20 + in_g * matrix.coeff21 + in_b * matrix.coeff22;

        r >>= matrix.r_shift;
        g >>= matrix.r_shift;
        b >>= matrix.r_shift;

        r += matrix.coeff03;
        g += matrix.coeff13;
        b += matrix.coeff23;

        r >>= 8;
        g >>= 8;
        b >>= 8;

        let cl_r = r.clamp(matrix.clamp_low, matrix.clamp_high);
        let cl_g = g.clamp(matrix.clamp_low, matrix.clamp_high);
        let cl_b = b.clamp(matrix.clamp_low, matrix.clamp_high);

        VicPixel {
            r: cl_r as u16,
            g: cl_g as u16,
            b: cl_b as u16,
            a: src_px.a,
        }
    } else {
        let cl_r = (src_px.r as i32).clamp(matrix.clamp_low, matrix.clamp_high);
        let cl_g = (src_px.g as i32).clamp(matrix.clamp_low, matrix.clamp_high);
        let cl_b = (src_px.b as i32).clamp(matrix.clamp_low, matrix.clamp_high);

        VicPixel {
            r: cl_r as u16,
            g: cl_g as u16,
            b: cl_b as u16,
            a: src_px.a,
        }
    }
}

/// Blends a source surface into destination surface using the specified color matrix and target rectangle.
/// Matches `Blender.BlendOne(dst, src, ref slot, targetRect)`.
///
/// `dst` must hold at least `dst_width * dst_height` pixels and `src` at least
/// `src_width * src_height`; otherwise nothing is written and
/// `Err(VIC_STATUS_INVALID_ARGUMENT)` is returned. The check is done up front, so the serial
/// and the `parallel` path can never index outside the buffers.
pub fn blend_surface(
    dst: &mut [VicPixel],
    dst_width: usize,
    dst_height: usize,
    src: &[VicPixel],
    src_width: usize,
    src_height: usize,
    rect: &VicRectangle,
    matrix: &VicColorMatrix,
) -> Result<(), i32> {
    if dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0 {
        return Ok(());
    }

    let dst_needed = dst_width.checked_mul(dst_height);
    let src_needed = src_width.checked_mul(src_height);
    match (dst_needed, src_needed) {
        (Some(d), Some(s)) if dst.len() >= d && src.len() >= s => {}
        _ => return Err(VIC_STATUS_INVALID_ARGUMENT),
    }

    let x1 = rect.x.max(0) as usize;
    let y1 = rect.y.max(0) as usize;
    // Widths/heights larger than i32::MAX clamp to i32::MAX here; that only matters for
    // the min() below, which is bounded by the rectangle anyway.
    let src_w = i32::try_from(src_width).unwrap_or(i32::MAX);
    let src_h = i32::try_from(src_height).unwrap_or(i32::MAX);
    let dst_w = i32::try_from(dst_width).unwrap_or(i32::MAX);
    let dst_h = i32::try_from(dst_height).unwrap_or(i32::MAX);
    let x2 = rect
        .x
        .saturating_add(rect.width)
        .min(src_w)
        .min(dst_w)
        .max(0) as usize;
    let y2 = rect
        .y
        .saturating_add(rect.height)
        .min(src_h)
        .min(dst_h)
        .max(0) as usize;

    if x1 >= x2 || y1 >= y2 {
        return Ok(());
    }

    // Validated above: y2 <= min(src_height, dst_height) and x2 <= min(src_width, dst_width),
    // so every row slice below is in bounds.
    let dst_rows = &mut dst[y1 * dst_width..y2 * dst_width];
    let src_rows = &src[y1 * src_width..y2 * src_width];

    #[cfg(feature = "parallel")]
    {
        if (y2 - y1) >= 64 {
            use rayon::prelude::*;

            dst_rows
                .par_chunks_mut(dst_width)
                .zip(src_rows.par_chunks(src_width))
                .for_each(|(d_row, s_row)| blend_row(&mut d_row[x1..x2], &s_row[x1..x2], matrix));
            return Ok(());
        }
    }

    for (d_row, s_row) in dst_rows
        .chunks_mut(dst_width)
        .zip(src_rows.chunks(src_width))
    {
        blend_row(&mut d_row[x1..x2], &s_row[x1..x2], matrix);
    }

    Ok(())
}

#[inline(always)]
fn blend_row(dst: &mut [VicPixel], src: &[VicPixel], matrix: &VicColorMatrix) {
    for (d, &s) in dst.iter_mut().zip(src) {
        *d = transform_pixel(s, matrix);
    }
}
