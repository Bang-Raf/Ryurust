//! BC1 (DXT1) texture decompression.

use crate::block::store_tile;

#[inline(always)]
pub fn convert_rgb565_to_rgb888(value: u32) -> u32 {
    let mut b = (value & 0x1f) << 19;
    let mut g = (value << 5) & 0xfc00;
    let mut r = (value >> 8) & 0xf8;

    b |= b >> 5;
    g |= g >> 6;
    r |= r >> 5;

    r | (g & 0xff00) | (b & 0xff0000)
}

#[inline(always)]
pub fn bc23_lerp_rgb2(color0: u32, color1: u32) -> u32 {
    let r0 = color0 & 0xff;
    let g0 = (color0 >> 8) & 0xff;
    let b0 = (color0 >> 16) & 0xff;

    let r1 = color1 & 0xff;
    let g1 = (color1 >> 8) & 0xff;
    let b1 = (color1 >> 16) & 0xff;

    let mix_r = (2 * r0 + r1) / 3;
    let mix_g = (2 * g0 + g1) / 3;
    let mix_b = (2 * b0 + b1) / 3;

    mix_r | (mix_g << 8) | (mix_b << 16)
}

#[inline(always)]
pub fn bc23_lerp_rgb3(color0: u32, color1: u32) -> u32 {
    let r0 = color0 & 0xff;
    let g0 = (color0 >> 8) & 0xff;
    let b0 = (color0 >> 16) & 0xff;

    let r1 = color1 & 0xff;
    let g1 = (color1 >> 8) & 0xff;
    let b1 = (color1 >> 16) & 0xff;

    let mix_r = (2 * r1 + r0) / 3;
    let mix_g = (2 * g1 + g0) / 3;
    let mix_b = (2 * b1 + b0) / 3;

    mix_r | (mix_g << 8) | (mix_b << 16)
}

#[inline(always)]
fn bc1_lerp_rgb2(color0: u32, color1: u32, c0: u32, c1: u32) -> u32 {
    if c0 > c1 {
        bc23_lerp_rgb2(color0, color1) | 0xff000000
    } else {
        let carry = color0 & color1;
        let add_halve = ((color0 ^ color1) >> 1) & 0x7f7f7f;
        (add_halve + carry) | 0xff000000
    }
}

#[inline(always)]
fn bc1_lerp_rgb3(color0: u32, color1: u32, c0: u32, c1: u32) -> u32 {
    if c0 > c1 {
        bc23_lerp_rgb3(color0, color1) | 0xff000000
    } else {
        0
    }
}

#[inline(always)]
pub fn decode_bc1_block(input: &[u8; 8], output: &mut [u32; 16]) {
    let c0c1 = u32::from_le_bytes(input[0..4].try_into().unwrap());
    let c0 = c0c1 & 0xffff;
    let c1 = c0c1 >> 16;

    let mut clut = [0u32; 4];
    clut[0] = convert_rgb565_to_rgb888(c0) | 0xff000000;
    clut[1] = convert_rgb565_to_rgb888(c1) | 0xff000000;
    clut[2] = bc1_lerp_rgb2(clut[0], clut[1], c0, c1);
    clut[3] = bc1_lerp_rgb3(clut[0], clut[1], c0, c1);

    let mut indices = u32::from_le_bytes(input[4..8].try_into().unwrap());
    for i in 0..16 {
        output[i] = clut[(indices & 3) as usize];
        indices >>= 2;
    }
}

pub fn decode_bc1_surface(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let expected_src = blocks_x * blocks_y * 8;
    let expected_dst = width * height * 4;
    if src.len() < expected_src {
        return Err("Source buffer too small");
    }
    if dst.len() < expected_dst {
        return Err("Destination buffer too small");
    }

    let mut tile = [0u32; 16];

    let mut src_offset = 0;
    for by in 0..blocks_y {
        let y_base = by * 4;
        let bh = (height - y_base).min(4);
        for bx in 0..blocks_x {
            let x_base = bx * 4;
            let bw = (width - x_base).min(4);

            let block_bytes: &[u8; 8] = src[src_offset..src_offset + 8].try_into().unwrap();
            src_offset += 8;

            decode_bc1_block(block_bytes, &mut tile);

            store_tile(dst, &tile, width, x_base, y_base, bw, bh);
        }
    }
    Ok(())
}
