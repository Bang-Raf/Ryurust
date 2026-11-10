//! Core pixel format transcoding and surface blitting routines.

pub const HOST_STRIDE_ALIGNMENT: usize = 4;

#[inline(always)]
pub fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

#[inline(always)]
pub fn get_line_remainders(
    length: usize,
    width: usize,
    bpp: usize,
    out_bpp: usize,
) -> (usize, usize, usize) {
    if bpp == 0 || width == 0 {
        return (0, 0, 0);
    }
    let stride = align_up(width * bpp, HOST_STRIDE_ALIGNMENT);
    let remainder = stride / bpp - width;

    let out_stride = align_up(width * out_bpp, HOST_STRIDE_ALIGNMENT);
    let out_remainder = out_stride / out_bpp - width;

    let height = if stride > 0 { length / stride } else { 0 };
    (remainder, out_remainder, height)
}

// -------------------------------------------------------------------------
// Pixel Math Formulas (100% Bit-Identical to Ryubing PixelConverter.cs)
// -------------------------------------------------------------------------

#[inline(always)]
pub fn convert_r5g6b5_pixel(packed: u16) -> u32 {
    let p = packed as u32;
    let mut out = 0xff000000;
    out |= (p << 3) & 0x000000f8;
    out |= (p << 8) & 0x00f80000;
    out |= (out >> 5) & 0x00070007;
    out |= ((p << 5) & 0x0000fc00) | ((p >> 1) & 0x00000300);
    out
}

#[inline(always)]
pub fn convert_r5g5b5_pixel(packed: u16, force_alpha: bool) -> u32 {
    let p = packed as u32;
    let a = if force_alpha { 1 } else { p >> 15 };
    let mut out = a * 0xff000000;
    out |= (p << 3) & 0x000000f8;
    out |= (p << 6) & 0x0000f800;
    out |= (p << 9) & 0x00f80000;
    out |= (out >> 5) & 0x00070707;
    out
}

#[inline(always)]
pub fn convert_a1b5g5r5_pixel(packed: u16) -> u32 {
    let p = packed as u32;
    let a = p >> 15;
    let mut out = a * 0xff000000;
    out |= (p >> 8) & 0x000000f8;
    out |= (p << 5) & 0x0000f800;
    out |= (p << 18) & 0x00f80000;
    out |= (out >> 5) & 0x00070707;
    out
}

#[inline(always)]
pub fn convert_r4g4b4a4_pixel(packed: u16) -> u32 {
    let p = packed as u32;
    let mut out = p & 0x0000000f;
    out |= (p << 4) & 0x00000f00;
    out |= (p << 8) & 0x000f0000;
    out |= (p << 12) & 0x0f000000;
    out.wrapping_mul(0x11)
}

#[inline(always)]
pub fn convert_bgra8_pixel(c: u32) -> u32 {
    ((c & 0x000000ff) << 16) | ((c & 0x00ff0000) >> 16) | (c & 0xff00ff00)
}

// -------------------------------------------------------------------------
// Surface Transcoding Routines
// -------------------------------------------------------------------------

pub fn convert_r4g4_to_r4g4b4a4(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }

    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 1, 2);

    let dst16: &mut [u16] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u16, dst.len() / 2) };

    if remainder == 0 && out_remainder == 0 {
        // Fast contiguous path
        for i in 0..src.len() {
            dst16[i] = src[i] as u16;
        }
    } else {
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src.len() && out_offset < dst16.len() {
                    dst16[out_offset] = src[offset] as u16;
                    out_offset += 1;
                    offset += 1;
                }
            }
            offset += remainder;
            out_offset += out_remainder;
        }
    }
    Ok(())
}

pub fn convert_r5g6b5_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }
    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 2, 4);

    let src16: &[u16] =
        unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u16, src.len() / 2) };
    let dst32: &mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len() / 4) };

    if remainder == 0 && out_remainder == 0 {
        let count = src16.len().min(dst32.len());
        for i in 0..count {
            dst32[i] = convert_r5g6b5_pixel(src16[i]);
        }
    } else {
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src16.len() && out_offset < dst32.len() {
                    dst32[out_offset] = convert_r5g6b5_pixel(src16[offset]);
                    out_offset += 1;
                    offset += 1;
                }
            }
            offset += remainder;
            out_offset += out_remainder;
        }
    }
    Ok(())
}

pub fn convert_r5g5b5_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    force_alpha: bool,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }
    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 2, 4);

    let src16: &[u16] =
        unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u16, src.len() / 2) };
    let dst32: &mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len() / 4) };

    if remainder == 0 && out_remainder == 0 {
        let count = src16.len().min(dst32.len());
        for i in 0..count {
            dst32[i] = convert_r5g5b5_pixel(src16[i], force_alpha);
        }
    } else {
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src16.len() && out_offset < dst32.len() {
                    dst32[out_offset] = convert_r5g5b5_pixel(src16[offset], force_alpha);
                    out_offset += 1;
                    offset += 1;
                }
            }
            offset += remainder;
            out_offset += out_remainder;
        }
    }
    Ok(())
}

pub fn convert_a1b5g5r5_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }
    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 2, 4);

    let src16: &[u16] =
        unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u16, src.len() / 2) };
    let dst32: &mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len() / 4) };

    if remainder == 0 && out_remainder == 0 {
        let count = src16.len().min(dst32.len());
        for i in 0..count {
            dst32[i] = convert_a1b5g5r5_pixel(src16[i]);
        }
    } else {
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src16.len() && out_offset < dst32.len() {
                    dst32[out_offset] = convert_a1b5g5r5_pixel(src16[offset]);
                    out_offset += 1;
                    offset += 1;
                }
            }
            offset += remainder;
            out_offset += out_remainder;
        }
    }
    Ok(())
}

pub fn convert_r4g4b4a4_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }
    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 2, 4);

    let src16: &[u16] =
        unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u16, src.len() / 2) };
    let dst32: &mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len() / 4) };

    if remainder == 0 && out_remainder == 0 {
        let count = src16.len().min(dst32.len());
        for i in 0..count {
            dst32[i] = convert_r4g4b4a4_pixel(src16[i]);
        }
    } else {
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src16.len() && out_offset < dst32.len() {
                    dst32[out_offset] = convert_r4g4b4a4_pixel(src16[offset]);
                    out_offset += 1;
                    offset += 1;
                }
            }
            offset += remainder;
            out_offset += out_remainder;
        }
    }
    Ok(())
}

pub fn convert_bgra8_to_rgba8(src: &[u8], dst: &mut [u8]) -> Result<(), &'static str> {
    if src.len() != dst.len() {
        return Err("Buffer length mismatch");
    }
    let src32: &[u32] =
        unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u32, src.len() / 4) };
    let dst32: &mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len() / 4) };
    for (s, d) in src32.iter().zip(dst32.iter_mut()) {
        *d = convert_bgra8_pixel(*s);
    }
    Ok(())
}
