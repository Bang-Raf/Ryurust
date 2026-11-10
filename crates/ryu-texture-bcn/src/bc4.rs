//! BC4 (RGTC1) texture decompression: single 8-bit channel (unorm or snorm).

use crate::bc3::bcn_lerp_alpha_unorm;

#[inline(always)]
pub fn bcn_lerp_alpha_snorm(a0_u: u8, a1_u: u8, pal: &mut [u8; 8]) {
    let a0 = a0_u as i8 as i32;
    let a1 = a1_u as i8 as i32;
    pal[0] = a0_u;
    pal[1] = a1_u;
    if a0 > a1 {
        pal[2] = (((6 * a0) + (1 * a1)) / 7) as i8 as u8;
        pal[3] = (((5 * a0) + (2 * a1)) / 7) as i8 as u8;
        pal[4] = (((4 * a0) + (3 * a1)) / 7) as i8 as u8;
        pal[5] = (((3 * a0) + (4 * a1)) / 7) as i8 as u8;
        pal[6] = (((2 * a0) + (5 * a1)) / 7) as i8 as u8;
        pal[7] = (((1 * a0) + (6 * a1)) / 7) as i8 as u8;
    } else {
        pal[2] = (((4 * a0) + (1 * a1)) / 5) as i8 as u8;
        pal[3] = (((3 * a0) + (2 * a1)) / 5) as i8 as u8;
        pal[4] = (((2 * a0) + (3 * a1)) / 5) as i8 as u8;
        pal[5] = (((1 * a0) + (4 * a1)) / 5) as i8 as u8;
        pal[6] = 0x80;
        pal[7] = 0x7f;
    }
}

#[inline(always)]
pub fn decode_bc4_block(input: &[u8; 8], output: &mut [u8; 16], signed: bool) {
    let a0 = input[0];
    let a1 = input[1];
    let mut pal = [0u8; 8];
    if signed {
        bcn_lerp_alpha_snorm(a0, a1, &mut pal);
    } else {
        bcn_lerp_alpha_unorm(a0, a1, &mut pal);
    }

    let word = u64::from_le_bytes(input[0..8].try_into().unwrap());
    let mut indices = word >> 16;
    for i in 0..16 {
        output[i] = pal[(indices & 7) as usize];
        indices >>= 3;
    }
}

pub fn decode_bc4_surface_stride(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    signed: bool,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let expected_src = blocks_x * blocks_y * 8;
    let expected_dst = stride * height;
    if src.len() < expected_src {
        return Err("Source buffer too small");
    }
    if dst.len() < expected_dst {
        return Err("Destination buffer too small");
    }

    let mut tile = [0u8; 16];
    let mut src_offset = 0;
    for by in 0..blocks_y {
        let y_base = by * 4;
        let bh = (height - y_base).min(4);
        for bx in 0..blocks_x {
            let x_base = bx * 4;
            let bw = (width - x_base).min(4);

            let block_bytes: &[u8; 8] = src[src_offset..src_offset + 8].try_into().unwrap();
            src_offset += 8;

            decode_bc4_block(block_bytes, &mut tile, signed);

            for ty in 0..bh {
                let dst_row = (y_base + ty) * stride + x_base;
                let tile_row = ty * 4;
                dst[dst_row..dst_row + bw].copy_from_slice(&tile[tile_row..tile_row + bw]);
            }
        }
    }
    Ok(())
}

pub fn decode_bc4_surface(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    signed: bool,
) -> Result<(), &'static str> {
    let aligned_stride = (width + 3) & !3;
    let stride = if dst.len() >= aligned_stride * height {
        aligned_stride
    } else {
        width
    };
    decode_bc4_surface_stride(src, dst, width, height, stride, signed)
}
