//! BC3 (DXT5) texture decompression: 8-bit interpolated alpha + BC1 RGB.

use crate::bc1::{bc23_lerp_rgb2, bc23_lerp_rgb3, convert_rgb565_to_rgb888};
use crate::block::store_tile;

#[inline(always)]
pub fn bcn_lerp_alpha_unorm(a0: u8, a1: u8, pal: &mut [u8; 8]) {
    pal[0] = a0;
    pal[1] = a1;
    let a0_32 = a0 as u32;
    let a1_32 = a1 as u32;
    if a0 > a1 {
        pal[2] = ((6 * a0_32 + 1 * a1_32) / 7) as u8;
        pal[3] = ((5 * a0_32 + 2 * a1_32) / 7) as u8;
        pal[4] = ((4 * a0_32 + 3 * a1_32) / 7) as u8;
        pal[5] = ((3 * a0_32 + 4 * a1_32) / 7) as u8;
        pal[6] = ((2 * a0_32 + 5 * a1_32) / 7) as u8;
        pal[7] = ((1 * a0_32 + 6 * a1_32) / 7) as u8;
    } else {
        pal[2] = ((4 * a0_32 + 1 * a1_32) / 5) as u8;
        pal[3] = ((3 * a0_32 + 2 * a1_32) / 5) as u8;
        pal[4] = ((2 * a0_32 + 3 * a1_32) / 5) as u8;
        pal[5] = ((1 * a0_32 + 4 * a1_32) / 5) as u8;
        pal[6] = 0;
        pal[7] = 0xff;
    }
}

#[inline(always)]
pub fn decode_bc3_block(input: &[u8; 16], output: &mut [u32; 16]) {
    let rgb_input: &[u8; 8] = input[8..16].try_into().unwrap();
    let c0c1 = u32::from_le_bytes(rgb_input[0..4].try_into().unwrap());
    let c0 = c0c1 & 0xffff;
    let c1 = c0c1 >> 16;

    let mut clut = [0u32; 4];
    clut[0] = convert_rgb565_to_rgb888(c0);
    clut[1] = convert_rgb565_to_rgb888(c1);
    clut[2] = bc23_lerp_rgb2(clut[0], clut[1]);
    clut[3] = bc23_lerp_rgb3(clut[0], clut[1]);

    let mut rgb_indices = u32::from_le_bytes(rgb_input[4..8].try_into().unwrap());
    let mut rgb_tile = [0u32; 16];
    for i in 0..16 {
        rgb_tile[i] = clut[(rgb_indices & 3) as usize];
        rgb_indices >>= 2;
    }

    let a0 = input[0];
    let a1 = input[1];
    let mut alpha_pal = [0u8; 8];
    bcn_lerp_alpha_unorm(a0, a1, &mut alpha_pal);

    let alpha_word = u64::from_le_bytes(input[0..8].try_into().unwrap());
    let mut alpha_indices = alpha_word >> 16;
    for i in 0..16 {
        let alpha = alpha_pal[(alpha_indices & 7) as usize] as u32;
        output[i] = rgb_tile[i] | (alpha << 24);
        alpha_indices >>= 3;
    }
}

pub fn decode_bc3_surface(
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
    let expected_src = blocks_x * blocks_y * 16;
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

            let block_bytes: &[u8; 16] = src[src_offset..src_offset + 16].try_into().unwrap();
            src_offset += 16;

            decode_bc3_block(block_bytes, &mut tile);

            store_tile(dst, &tile, width, x_base, y_base, bw, bh);
        }
    }
    Ok(())
}
