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
pub fn blend_surface(
    dst: &mut [VicPixel],
    dst_width: usize,
    dst_height: usize,
    src: &[VicPixel],
    src_width: usize,
    src_height: usize,
    rect: &VicRectangle,
    matrix: &VicColorMatrix,
) {
    if dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0 {
        return;
    }

    let x1 = rect.x.max(0) as usize;
    let y1 = rect.y.max(0) as usize;
    let x2 = (rect.x + rect.width)
        .min(src_width as i32)
        .min(dst_width as i32)
        .max(0) as usize;
    let y2 = (rect.y + rect.height)
        .min(src_height as i32)
        .min(dst_height as i32)
        .max(0) as usize;

    if x1 >= x2 || y1 >= y2 {
        return;
    }

    #[cfg(feature = "parallel")]
    {
        if (y2 - y1) >= 64 {
            use rayon::prelude::*;

            let dst_ptr = dst.as_mut_ptr() as usize;
            let src_ptr = src.as_ptr() as usize;

            (y1..y2).into_par_iter().for_each(|y| unsafe {
                let d_row = (dst_ptr as *mut VicPixel).add(y * dst_width);
                let s_row = (src_ptr as *const VicPixel).add(y * src_width);

                for x in x1..x2 {
                    let sp = *s_row.add(x);
                    *d_row.add(x) = transform_pixel(sp, matrix);
                }
            });
            return;
        }
    }

    for y in y1..y2 {
        let s_off = y * src_width;
        let d_off = y * dst_width;

        let s_row = &src[s_off..s_off + src_width];
        let d_row = &mut dst[d_off..d_off + dst_width];

        for x in x1..x2 {
            d_row[x] = transform_pixel(s_row[x], matrix);
        }
    }
}
