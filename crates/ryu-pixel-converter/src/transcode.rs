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

// All routines below operate on byte slices with no alignment requirement: pixels are
// read and written with `from_le_bytes` / `to_le_bytes` over `chunks_exact`, which the
// compiler lowers to unaligned loads/stores (and vectorises on the contiguous path).

#[inline(always)]
fn read_u16(src: &[u8], index: usize) -> u16 {
    u16::from_le_bytes([src[index * 2], src[index * 2 + 1]])
}

#[inline(always)]
fn write_u32(dst: &mut [u8], index: usize, value: u32) {
    dst[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
}

/// Shared body of the 16 bpp -> 32 bpp transcoders.
#[inline(always)]
fn convert_16_to_32(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    convert: impl Fn(u16) -> u32,
) -> Result<(), &'static str> {
    if src.is_empty() {
        return Ok(());
    }
    if dst.len() < src.len() * 2 {
        return Err("Destination buffer too small");
    }
    let (remainder, out_remainder, height) = get_line_remainders(src.len(), width, 2, 4);

    if remainder == 0 && out_remainder == 0 {
        // Fixed-size array chunks carry no per-pixel bounds checks, so this
        // vectorises like the old `&[u16]` cast did, without needing alignment.
        let (src_px, _) = src.as_chunks::<2>();
        let (dst_px, _) = dst.as_chunks_mut::<4>();
        for (s, d) in src_px.iter().zip(dst_px.iter_mut()) {
            *d = convert(u16::from_le_bytes(*s)).to_le_bytes();
        }
    } else {
        let src_pixels = src.len() / 2;
        let dst_pixels = dst.len() / 4;
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src_pixels && out_offset < dst_pixels {
                    write_u32(dst, out_offset, convert(read_u16(src, offset)));
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

    if remainder == 0 && out_remainder == 0 {
        // Fast contiguous path
        let (dst_px, _) = dst.as_chunks_mut::<2>();
        for (&s, d) in src.iter().zip(dst_px.iter_mut()) {
            *d = (s as u16).to_le_bytes();
        }
    } else {
        let dst_pixels = dst.len() / 2;
        let mut offset = 0;
        let mut out_offset = 0;
        for _ in 0..height {
            for _ in 0..width {
                if offset < src.len() && out_offset < dst_pixels {
                    dst[out_offset * 2..out_offset * 2 + 2]
                        .copy_from_slice(&(src[offset] as u16).to_le_bytes());
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
    convert_16_to_32(src, dst, width, convert_r5g6b5_pixel)
}

pub fn convert_r5g5b5_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    force_alpha: bool,
) -> Result<(), &'static str> {
    convert_16_to_32(src, dst, width, |p| convert_r5g5b5_pixel(p, force_alpha))
}

pub fn convert_a1b5g5r5_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    convert_16_to_32(src, dst, width, convert_a1b5g5r5_pixel)
}

pub fn convert_r4g4b4a4_to_r8g8b8a8(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
) -> Result<(), &'static str> {
    convert_16_to_32(src, dst, width, convert_r4g4b4a4_pixel)
}

/// Error returned by [`convert_bgra8_to_rgba8`] when the buffers differ in length.
pub const ERR_BUFFER_LENGTH_MISMATCH: &str = "Buffer length mismatch";
/// Error returned by [`convert_bgra8_to_rgba8`] when the length is not a whole number
/// of 4-byte pixels.
pub const ERR_LENGTH_NOT_PIXEL_MULTIPLE: &str = "Buffer length is not a multiple of 4";

/// Swaps the R and B channels of every BGRA8 pixel in `src` into `dst`.
///
/// `src` and `dst` must have the same length, and that length must be a multiple of 4
/// (whole pixels); otherwise an error is returned and `dst` is left untouched.
pub fn convert_bgra8_to_rgba8(src: &[u8], dst: &mut [u8]) -> Result<(), &'static str> {
    if src.len() != dst.len() {
        return Err(ERR_BUFFER_LENGTH_MISMATCH);
    }
    if src.len() % 4 != 0 {
        return Err(ERR_LENGTH_NOT_PIXEL_MULTIPLE);
    }
    let (src_px, _) = src.as_chunks::<4>();
    let (dst_px, _) = dst.as_chunks_mut::<4>();
    for (s, d) in src_px.iter().zip(dst_px.iter_mut()) {
        *d = convert_bgra8_pixel(u32::from_le_bytes(*s)).to_le_bytes();
    }
    Ok(())
}
