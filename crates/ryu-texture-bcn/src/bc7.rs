//! BC7 (BPTC) texture decompression.

use crate::block::{store_tile, Block, RgbaColor32};
use crate::tables::{BC7_MODE_INFOS, FIXUP_INDICES, PARTITION_TABLE, WEIGHTS};

#[inline(always)]
fn unquantize_component(component: i32, bits: usize, p_bit: i32) -> i32 {
    let shift = 8 - bits;
    let mut value = component << shift;

    if p_bit >= 0 {
        value |= value >> (bits + 1);
        value |= p_bit << (shift - 1);
    } else {
        value |= value >> bits;
    }

    value
}

#[inline(always)]
fn unquantize(color: &mut RgbaColor32, color_depth: usize, alpha_depth: usize, p_bit: i32) {
    color.r = unquantize_component(color.r, color_depth, p_bit);
    color.g = unquantize_component(color.g, color_depth, p_bit);
    color.b = unquantize_component(color.b, color_depth, p_bit);
    color.a = if alpha_depth != 0 {
        unquantize_component(color.a, alpha_depth, p_bit)
    } else {
        255
    };
}

#[inline(always)]
fn interpolate(
    color1: RgbaColor32,
    color2: RgbaColor32,
    color_weight_index: usize,
    alpha_weight_index: usize,
    color_index_bit_count: usize,
    alpha_index_bit_count: usize,
) -> RgbaColor32 {
    let color_weight = WEIGHTS[color_index_bit_count - 2][color_weight_index] as i32;
    let alpha_weight = WEIGHTS[alpha_index_bit_count - 2][alpha_weight_index] as i32;

    let inv_color_weight = 64 - color_weight;
    let inv_alpha_weight = 64 - alpha_weight;

    RgbaColor32 {
        r: (color1.r * inv_color_weight + color2.r * color_weight + 32) >> 6,
        g: (color1.g * inv_color_weight + color2.g * color_weight + 32) >> 6,
        b: (color1.b * inv_color_weight + color2.b * color_weight + 32) >> 6,
        a: (color1.a * inv_alpha_weight + color2.a * alpha_weight + 32) >> 6,
    }
}

pub fn decode_bc7_block(block: Block, output: &mut [u32; 16]) {
    let mode = ((block.low as u8 as u32) | 0x100).trailing_zeros() as usize;
    if mode == 8 {
        output.fill(0);
        return;
    }

    let mode_info = &BC7_MODE_INFOS[mode];
    let mut offset = mode + 1;

    let partition = block.decode(&mut offset, mode_info.partition_bit_count) as usize;
    let rotation = block.decode(&mut offset, mode_info.rotation_bit_count) as usize;
    let index_mode = block.decode(&mut offset, mode_info.index_mode_bit_count) as usize;

    let endpoint_count = mode_info.subset_count * 2;
    let mut endpoints = [RgbaColor32::new(0, 0, 0, 255); 6];
    let mut p_values = [0u8; 6];

    for ep in &mut endpoints[..endpoint_count] {
        ep.r = block.decode(&mut offset, mode_info.color_depth) as i32;
    }
    for ep in &mut endpoints[..endpoint_count] {
        ep.g = block.decode(&mut offset, mode_info.color_depth) as i32;
    }
    for ep in &mut endpoints[..endpoint_count] {
        ep.b = block.decode(&mut offset, mode_info.color_depth) as i32;
    }
    if mode_info.alpha_depth != 0 {
        for ep in &mut endpoints[..endpoint_count] {
            ep.a = block.decode(&mut offset, mode_info.alpha_depth) as i32;
        }
    }

    for p in &mut p_values[..mode_info.p_bits] {
        *p = block.decode(&mut offset, 1) as u8;
    }

    for (i, ep) in endpoints[..endpoint_count].iter_mut().enumerate() {
        let p_bit = if mode_info.p_bits != 0 {
            let p_index = (i * mode_info.p_bits) / endpoint_count;
            p_values[p_index] as i32
        } else {
            -1
        };
        unquantize(ep, mode_info.color_depth, mode_info.alpha_depth, p_bit);
    }

    let partition_table = &PARTITION_TABLE[mode_info.subset_count - 1][partition];
    let fixup_table = &FIXUP_INDICES[mode_info.subset_count - 1][partition];

    let mut color_indices = [0u8; 16];
    for i in 0..16 {
        let subset = partition_table[i] as usize;
        let bit_count = if i == fixup_table[subset] {
            mode_info.color_index_bit_count - 1
        } else {
            mode_info.color_index_bit_count
        };
        color_indices[i] = block.decode(&mut offset, bit_count) as u8;
    }

    let mut alpha_indices = [0u8; 16];
    if mode_info.alpha_index_bit_count != 0 {
        for (i, alpha_index) in alpha_indices.iter_mut().enumerate() {
            let bit_count = if i != 0 {
                mode_info.alpha_index_bit_count
            } else {
                mode_info.alpha_index_bit_count - 1
            };
            *alpha_index = block.decode(&mut offset, bit_count) as u8;
        }
    }

    for i in 0..16 {
        let subset = partition_table[i] as usize;
        let color1 = endpoints[subset * 2];
        let color2 = endpoints[subset * 2 + 1];

        let mut color = if mode_info.alpha_index_bit_count != 0 {
            if index_mode == 0 {
                interpolate(
                    color1,
                    color2,
                    color_indices[i] as usize,
                    alpha_indices[i] as usize,
                    mode_info.color_index_bit_count,
                    mode_info.alpha_index_bit_count,
                )
            } else {
                interpolate(
                    color1,
                    color2,
                    alpha_indices[i] as usize,
                    color_indices[i] as usize,
                    mode_info.alpha_index_bit_count,
                    mode_info.color_index_bit_count,
                )
            }
        } else {
            interpolate(
                color1,
                color2,
                color_indices[i] as usize,
                color_indices[i] as usize,
                mode_info.color_index_bit_count,
                mode_info.color_index_bit_count,
            )
        };

        if rotation != 0 {
            let a = color.a;
            match rotation {
                1 => {
                    color.a = color.r;
                    color.r = a;
                }
                2 => {
                    color.a = color.g;
                    color.g = a;
                }
                3 => {
                    color.a = color.b;
                    color.b = a;
                }
                _ => {}
            }
        }

        output[i] = color.get_color8().to_u32();
    }
}

pub fn decode_bc7_surface(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
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

            let block = Block::from_slice(&src[src_offset..src_offset + 16]);
            src_offset += 16;

            decode_bc7_block(block, &mut tile);

            store_tile(dst, &tile, width, x_base, y_base, bw, bh);
        }
    }
    Ok(())
}
