//! BC2 (DXT3) texture decompression: explicit 4-bit alpha + BC1 RGB.

use crate::bc1::{bc23_lerp_rgb2, bc23_lerp_rgb3, convert_rgb565_to_rgb888};
use crate::block::store_tile;

#[inline(always)]
pub fn decode_bc2_block(input: &[u8; 16], output: &mut [u32; 16]) {
    let rgb_input: &[u8; 8] = input[8..16].try_into().unwrap();
    let c0c1 = u32::from_le_bytes(rgb_input[0..4].try_into().unwrap());
    let c0 = c0c1 & 0xffff;
    let c1 = c0c1 >> 16;

    let mut clut = [0u32; 4];
    clut[0] = convert_rgb565_to_rgb888(c0);
    clut[1] = convert_rgb565_to_rgb888(c1);
    clut[2] = bc23_lerp_rgb2(clut[0], clut[1]);
    clut[3] = bc23_lerp_rgb3(clut[0], clut[1]);

    let mut indices = u32::from_le_bytes(rgb_input[4..8].try_into().unwrap());
    let mut rgb_tile = [0u32; 16];
    for i in 0..16 {
        rgb_tile[i] = clut[(indices & 3) as usize];
        indices >>= 2;
    }

    let mut alpha_bits = u64::from_le_bytes(input[0..8].try_into().unwrap());
    for i in 0..16 {
        let a4 = (alpha_bits & 0xf) as u32;
        let a8 = (a4 | (a4 << 4)) << 24;
        output[i] = rgb_tile[i] | a8;
        alpha_bits >>= 4;
    }
}

pub fn decode_bc2_surface(
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

            decode_bc2_block(block_bytes, &mut tile);

            store_tile(dst, &tile, width, x_base, y_base, bw, bh);
        }
    }
    Ok(())
}
