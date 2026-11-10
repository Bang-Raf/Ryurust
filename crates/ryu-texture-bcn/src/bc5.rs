//! BC5 (RGTC2) texture decompression: two 8-bit channels RG (unorm or snorm).

use crate::bc4::decode_bc4_block;

#[inline(always)]
pub fn decode_bc5_block(input: &[u8; 16], output_rg: &mut [u16; 16], signed: bool) {
    let r_input: &[u8; 8] = input[0..8].try_into().unwrap();
    let g_input: &[u8; 8] = input[8..16].try_into().unwrap();

    let mut r_tile = [0u8; 16];
    let mut g_tile = [0u8; 16];

    decode_bc4_block(r_input, &mut r_tile, signed);
    decode_bc4_block(g_input, &mut g_tile, signed);

    for i in 0..16 {
        output_rg[i] = (r_tile[i] as u16) | ((g_tile[i] as u16) << 8);
    }
}

pub fn decode_bc5_surface_stride(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    stride_pixels: usize,
    signed: bool,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let expected_src = blocks_x * blocks_y * 16;
    let expected_dst = stride_pixels * height * 2;
    if src.len() < expected_src {
        return Err("Source buffer too small");
    }
    if dst.len() < expected_dst {
        return Err("Destination buffer too small");
    }

    let mut tile = [0u16; 16];
    let dst16: &mut [u16] = unsafe {
        std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u16, stride_pixels * height)
    };

    let mut src_offset = 0;
    for by in 0..blocks_y {
        let y_base = by * 4;
        let bh = (height - y_base).min(4);
        for bx in 0..blocks_x {
            let x_base = bx * 4;
            let bw = (width - x_base).min(4);

            let block_bytes: &[u8; 16] = src[src_offset..src_offset + 16].try_into().unwrap();
            src_offset += 16;

            decode_bc5_block(block_bytes, &mut tile, signed);

            for ty in 0..bh {
                let dst_row = (y_base + ty) * stride_pixels + x_base;
                let tile_row = ty * 4;
                for tx in 0..bw {
                    dst16[dst_row + tx] = tile[tile_row + tx];
                }
            }
        }
    }
    Ok(())
}

pub fn decode_bc5_surface(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    signed: bool,
) -> Result<(), &'static str> {
    let aligned_stride = (width + 1) & !1;
    let stride = if dst.len() >= aligned_stride * height * 2 {
        aligned_stride
    } else {
        width
    };
    decode_bc5_surface_stride(src, dst, width, height, stride, signed)
}
