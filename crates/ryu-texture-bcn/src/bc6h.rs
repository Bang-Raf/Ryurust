//! BC6H HDR texture decompression (RGBA16F).

use crate::block::{store_tile, Block, RgbaColor32};
use crate::tables::{FIXUP_INDICES, PARTITION_TABLE};

pub const HALF_ONE: i32 = 0x3C00;

#[inline(always)]
fn sign_extend(value: i32, bits: usize) -> i32 {
    let shift = 32 - bits;
    (value << shift) >> shift
}

#[inline(always)]
fn unquantize(value: i32, bits: usize, signed: bool) -> i32 {
    if signed {
        if bits >= 16 {
            value
        } else {
            let sign = value < 0;
            let mut val = if sign { -value } else { value };
            if val == 0 {
                return 0;
            } else if val >= ((1 << (bits - 1)) - 1) {
                val = 0x7FFF;
            } else {
                val = ((val << 15) + 0x4000) >> (bits - 1);
            }
            if sign {
                -val
            } else {
                val
            }
        }
    } else {
        if bits >= 15 || value == 0 {
            value
        } else if value == ((1 << bits) - 1) {
            0xFFFF
        } else {
            ((value << 16) + 0x8000) >> bits
        }
    }
}

#[inline(always)]
fn finish_unquantize(mut value: i32, signed: bool) -> u16 {
    if signed {
        value = if value < 0 {
            -((-value * 31) >> 5)
        } else {
            (value * 31) >> 5
        };
        let mut sign = 0u16;
        if value < 0 {
            sign = 0x8000;
            value = -value;
        }
        sign | (value as u16)
    } else {
        ((value * 31) >> 6) as u16
    }
}

#[inline(always)]
fn bit_reverse_6(mut value: i32) -> i32 {
    value = ((value >> 1) & 0x55) | ((value << 1) & 0xaa);
    value = ((value >> 2) & 0x33) | ((value << 2) & 0xcc);
    value = ((value >> 4) & 0x0f) | ((value << 4) & 0xf0);
    value >> 2
}

#[inline(always)]
fn interpolate(
    color1: RgbaColor32,
    color2: RgbaColor32,
    weight_index: usize,
    index_bit_count: usize,
) -> RgbaColor32 {
    let weight = (((weight_index << 7) / ((1 << index_bit_count) - 1)) + 1) >> 1;
    let inv_weight = 64 - weight as i32;
    let w = weight as i32;

    RgbaColor32 {
        r: (color1.r * inv_weight + color2.r * w + 32) >> 6,
        g: (color1.g * inv_weight + color2.g * w + 32) >> 6,
        b: (color1.b * inv_weight + color2.b * w + 32) >> 6,
        a: (color1.a * inv_weight + color2.a * w + 32) >> 6,
    }
}
pub fn decode_end_points(
    block: &Block,
    end_points: &mut [RgbaColor32; 4],
    mode: usize,
    signed: bool,
) -> usize {
    let low = block.low;
    let high = block.high;

    let (mut r0, mut g0, mut b0, mut r1, mut g1, mut b1) = (0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
    let (mut r2, mut g2, mut b2, mut r3, mut g3, mut b3) = (0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
    let mut subset_count: usize = 0;

    match mode {
        0 => {
            r0 = ((low >> 5) as i32) & 0x3FF;
            g0 = ((low >> 15) as i32) & 0x3FF;
            b0 = ((low >> 25) as i32) & 0x3FF;

            if signed {
                r0 = sign_extend(r0, 10);
                g0 = sign_extend(g0, 10);
                b0 = sign_extend(b0, 10);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 5);
            g1 = g0 + sign_extend(((low >> 45) as i32), 5);
            b1 = b0 + sign_extend(((low >> 55) as i32), 5);

            r2 = r0 + sign_extend(((high >> 1) as i32), 5);
            g2 = g0 + sign_extend(((((low << 2) & 0x10) | ((low >> 41) & 0xF)) as i32), 5);
            b2 = b0
                + sign_extend(
                    ((((low << 1) & 0x10) | ((high << 3) & 0x08) | (low >> 61)) as i32),
                    5,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 5);
            g3 = g0 + sign_extend(((((low >> 36) & 0x10) | ((low >> 51) & 0xF)) as i32), 5);
            b3 = b0
                + sign_extend(
                    ((((low) & 0x10)
                        | ((high >> 9) & 0x08)
                        | ((high >> 4) & 0x04)
                        | ((low >> 59) & 0x02)
                        | ((low >> 50) & 0x01)) as i32),
                    5,
                );

            r0 = unquantize(r0, 10, signed);
            g0 = unquantize(g0, 10, signed);
            b0 = unquantize(b0, 10, signed);

            r1 = unquantize(r1 & 0x3FF, 10, signed);
            g1 = unquantize(g1 & 0x3FF, 10, signed);
            b1 = unquantize(b1 & 0x3FF, 10, signed);

            r2 = unquantize(r2 & 0x3FF, 10, signed);
            g2 = unquantize(g2 & 0x3FF, 10, signed);
            b2 = unquantize(b2 & 0x3FF, 10, signed);

            r3 = unquantize(r3 & 0x3FF, 10, signed);
            g3 = unquantize(g3 & 0x3FF, 10, signed);
            b3 = unquantize(b3 & 0x3FF, 10, signed);

            subset_count = 2;
        }
        1 => {
            r0 = ((low >> 5) as i32) & 0x7F;
            g0 = ((low >> 15) as i32) & 0x7F;
            b0 = ((low >> 25) as i32) & 0x7F;

            if signed {
                r0 = sign_extend(r0, 7);
                g0 = sign_extend(g0, 7);
                b0 = sign_extend(b0, 7);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 6);
            g1 = g0 + sign_extend(((low >> 45) as i32), 6);
            b1 = b0 + sign_extend(((low >> 55) as i32), 6);

            r2 = r0 + sign_extend(((high >> 1) as i32), 6);
            g2 = g0
                + sign_extend(
                    ((((low << 3) & 0x20) | ((low >> 20) & 0x10) | ((low >> 41) & 0x0F)) as i32),
                    6,
                );
            b2 = b0
                + sign_extend(
                    ((((low >> 17) & 0x20)
                        | ((low >> 10) & 0x10)
                        | ((high << 3) & 0x08)
                        | (low >> 61)) as i32),
                    6,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 6);
            g3 = g0 + sign_extend(((((low << 1) & 0x30) | ((low >> 51) & 0xF)) as i32), 6);
            b3 = b0
                + sign_extend(
                    ((((low >> 28) & 0x20)
                        | ((low >> 30) & 0x10)
                        | ((low >> 29) & 0x08)
                        | ((low >> 21) & 0x04)
                        | ((low >> 12) & 0x03)) as i32),
                    6,
                );

            r0 = unquantize(r0, 7, signed);
            g0 = unquantize(g0, 7, signed);
            b0 = unquantize(b0, 7, signed);

            r1 = unquantize(r1 & 0x7F, 7, signed);
            g1 = unquantize(g1 & 0x7F, 7, signed);
            b1 = unquantize(b1 & 0x7F, 7, signed);

            r2 = unquantize(r2 & 0x7F, 7, signed);
            g2 = unquantize(g2 & 0x7F, 7, signed);
            b2 = unquantize(b2 & 0x7F, 7, signed);

            r3 = unquantize(r3 & 0x7F, 7, signed);
            g3 = unquantize(g3 & 0x7F, 7, signed);
            b3 = unquantize(b3 & 0x7F, 7, signed);

            subset_count = 2;
        }
        2 => {
            r0 = ((((low >> 30) & 0x400) | ((low >> 5) & 0x3FF)) as i32);
            g0 = ((((low >> 39) & 0x400) | ((low >> 15) & 0x3FF)) as i32);
            b0 = ((((low >> 49) & 0x400) | ((low >> 25) & 0x3FF)) as i32);

            if signed {
                r0 = sign_extend(r0, 11);
                g0 = sign_extend(g0, 11);
                b0 = sign_extend(b0, 11);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 5);
            g1 = g0 + sign_extend(((low >> 45) as i32), 4);
            b1 = b0 + sign_extend(((low >> 55) as i32), 4);

            r2 = r0 + sign_extend(((high >> 1) as i32), 5);
            g2 = g0 + sign_extend(((low >> 41) as i32), 4);
            b2 = b0 + sign_extend(((((high << 3) & 8) | (low >> 61)) as i32), 4);

            r3 = r0 + sign_extend(((high >> 7) as i32), 5);
            g3 = g0 + sign_extend(((low >> 51) as i32), 4);
            b3 = b0
                + sign_extend(
                    ((((high >> 9) & 8) | ((high >> 4) & 4) | ((low >> 59) & 2) | ((low >> 50) & 1))
                        as i32),
                    4,
                );

            r0 = unquantize(r0, 11, signed);
            g0 = unquantize(g0, 11, signed);
            b0 = unquantize(b0, 11, signed);

            r1 = unquantize(r1 & 0x7FF, 11, signed);
            g1 = unquantize(g1 & 0x7FF, 11, signed);
            b1 = unquantize(b1 & 0x7FF, 11, signed);

            r2 = unquantize(r2 & 0x7FF, 11, signed);
            g2 = unquantize(g2 & 0x7FF, 11, signed);
            b2 = unquantize(b2 & 0x7FF, 11, signed);

            r3 = unquantize(r3 & 0x7FF, 11, signed);
            g3 = unquantize(g3 & 0x7FF, 11, signed);
            b3 = unquantize(b3 & 0x7FF, 11, signed);

            subset_count = 2;
        }
        3 => {
            r0 = ((low >> 5) as i32) & 0x3FF;
            g0 = ((low >> 15) as i32) & 0x3FF;
            b0 = ((low >> 25) as i32) & 0x3FF;

            r1 = ((low >> 35) as i32) & 0x3FF;
            g1 = ((low >> 45) as i32) & 0x3FF;
            b1 = ((((high << 9) & 0x200) | (low >> 55)) as i32);

            if signed {
                r0 = sign_extend(r0, 10);
                g0 = sign_extend(g0, 10);
                b0 = sign_extend(b0, 10);

                r1 = sign_extend(r1, 10);
                g1 = sign_extend(g1, 10);
                b1 = sign_extend(b1, 10);
            }

            r0 = unquantize(r0, 10, signed);
            g0 = unquantize(g0, 10, signed);
            b0 = unquantize(b0, 10, signed);

            r1 = unquantize(r1, 10, signed);
            g1 = unquantize(g1, 10, signed);
            b1 = unquantize(b1, 10, signed);

            subset_count = 1;
        }
        6 => {
            r0 = ((((low >> 29) & 0x400) | ((low >> 5) & 0x3FF)) as i32);
            g0 = ((((low >> 40) & 0x400) | ((low >> 15) & 0x3FF)) as i32);
            b0 = ((((low >> 49) & 0x400) | ((low >> 25) & 0x3FF)) as i32);

            if signed {
                r0 = sign_extend(r0, 11);
                g0 = sign_extend(g0, 11);
                b0 = sign_extend(b0, 11);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 4);
            g1 = g0 + sign_extend(((low >> 45) as i32), 5);
            b1 = b0 + sign_extend(((low >> 55) as i32), 4);

            r2 = r0 + sign_extend(((high >> 1) as i32), 4);
            g2 = g0 + sign_extend(((((high >> 7) & 0x10) | ((low >> 41) & 0x0F)) as i32), 5);
            b2 = b0 + sign_extend(((((high << 3) & 0x08) | (low >> 61)) as i32), 4);

            r3 = r0 + sign_extend(((high >> 7) as i32), 4);
            g3 = g0 + sign_extend(((((low >> 36) & 0x10) | ((low >> 51) & 0x0F)) as i32), 5);
            b3 = b0
                + sign_extend(
                    ((((high >> 9) & 8) | ((high >> 4) & 4) | ((low >> 59) & 2) | ((high >> 5) & 1))
                        as i32),
                    4,
                );

            r0 = unquantize(r0, 11, signed);
            g0 = unquantize(g0, 11, signed);
            b0 = unquantize(b0, 11, signed);

            r1 = unquantize(r1 & 0x7FF, 11, signed);
            g1 = unquantize(g1 & 0x7FF, 11, signed);
            b1 = unquantize(b1 & 0x7FF, 11, signed);

            r2 = unquantize(r2 & 0x7FF, 11, signed);
            g2 = unquantize(g2 & 0x7FF, 11, signed);
            b2 = unquantize(b2 & 0x7FF, 11, signed);

            r3 = unquantize(r3 & 0x7FF, 11, signed);
            g3 = unquantize(g3 & 0x7FF, 11, signed);
            b3 = unquantize(b3 & 0x7FF, 11, signed);

            subset_count = 2;
        }
        7 => {
            r0 = ((((low >> 34) & 0x400) | ((low >> 5) & 0x3FF)) as i32);
            g0 = ((((low >> 44) & 0x400) | ((low >> 15) & 0x3FF)) as i32);
            b0 = ((((high << 10) & 0x400) | ((low >> 25) & 0x3FF)) as i32);

            if signed {
                r0 = sign_extend(r0, 11);
                g0 = sign_extend(g0, 11);
                b0 = sign_extend(b0, 11);
            }

            r1 = (r0 + sign_extend(((low >> 35) as i32), 9)) & 0x7FF;
            g1 = (g0 + sign_extend(((low >> 45) as i32), 9)) & 0x7FF;
            b1 = (b0 + sign_extend(((low >> 55) as i32), 9)) & 0x7FF;

            r0 = unquantize(r0, 11, signed);
            g0 = unquantize(g0, 11, signed);
            b0 = unquantize(b0, 11, signed);

            r1 = unquantize(r1, 11, signed);
            g1 = unquantize(g1, 11, signed);
            b1 = unquantize(b1, 11, signed);

            subset_count = 1;
        }
        10 => {
            r0 = ((((low >> 29) & 0x400) | ((low >> 5) & 0x3FF)) as i32);
            g0 = ((((low >> 39) & 0x400) | ((low >> 15) & 0x3FF)) as i32);
            b0 = ((((low >> 50) & 0x400) | ((low >> 25) & 0x3FF)) as i32);

            if signed {
                r0 = sign_extend(r0, 11);
                g0 = sign_extend(g0, 11);
                b0 = sign_extend(b0, 11);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 4);
            g1 = g0 + sign_extend(((low >> 45) as i32), 4);
            b1 = b0 + sign_extend(((low >> 55) as i32), 5);

            r2 = r0 + sign_extend(((high >> 1) as i32), 4);
            g2 = g0 + sign_extend(((low >> 41) as i32), 4);
            b2 = b0
                + sign_extend(
                    ((((low >> 36) & 0x10) | ((high << 3) & 8) | (low >> 61)) as i32),
                    5,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 4);
            g3 = g0 + sign_extend(((low >> 51) as i32), 4);
            b3 = b0
                + sign_extend(
                    ((((high >> 7) & 0x10)
                        | ((high >> 9) & 0x08)
                        | ((high >> 4) & 0x06)
                        | ((low >> 50) & 0x01)) as i32),
                    5,
                );

            r0 = unquantize(r0, 11, signed);
            g0 = unquantize(g0, 11, signed);
            b0 = unquantize(b0, 11, signed);

            r1 = unquantize(r1 & 0x7FF, 11, signed);
            g1 = unquantize(g1 & 0x7FF, 11, signed);
            b1 = unquantize(b1 & 0x7FF, 11, signed);

            r2 = unquantize(r2 & 0x7FF, 11, signed);
            g2 = unquantize(g2 & 0x7FF, 11, signed);
            b2 = unquantize(b2 & 0x7FF, 11, signed);

            r3 = unquantize(r3 & 0x7FF, 11, signed);
            g3 = unquantize(g3 & 0x7FF, 11, signed);
            b3 = unquantize(b3 & 0x7FF, 11, signed);

            subset_count = 2;
        }
        11 => {
            r0 = ((((low >> 32) & 0x800) | ((low >> 34) & 0x400) | ((low >> 5) & 0x3FF)) as i32);
            g0 = ((((low >> 42) & 0x800) | ((low >> 44) & 0x400) | ((low >> 15) & 0x3FF)) as i32);
            b0 = ((((low >> 52) & 0x800) | ((high << 10) & 0x400) | ((low >> 25) & 0x3FF)) as i32);

            if signed {
                r0 = sign_extend(r0, 12);
                g0 = sign_extend(g0, 12);
                b0 = sign_extend(b0, 12);
            }

            r1 = (r0 + sign_extend(((low >> 35) as i32), 8)) & 0xFFF;
            g1 = (g0 + sign_extend(((low >> 45) as i32), 8)) & 0xFFF;
            b1 = (b0 + sign_extend(((low >> 55) as i32), 8)) & 0xFFF;

            r0 = unquantize(r0, 12, signed);
            g0 = unquantize(g0, 12, signed);
            b0 = unquantize(b0, 12, signed);

            r1 = unquantize(r1, 12, signed);
            g1 = unquantize(g1, 12, signed);
            b1 = unquantize(b1, 12, signed);

            subset_count = 1;
        }
        14 => {
            r0 = ((low >> 5) as i32) & 0x1FF;
            g0 = ((low >> 15) as i32) & 0x1FF;
            b0 = ((low >> 25) as i32) & 0x1FF;

            if signed {
                r0 = sign_extend(r0, 9);
                g0 = sign_extend(g0, 9);
                b0 = sign_extend(b0, 9);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 5);
            g1 = g0 + sign_extend(((low >> 45) as i32), 5);
            b1 = b0 + sign_extend(((low >> 55) as i32), 5);

            r2 = r0 + sign_extend(((high >> 1) as i32), 5);
            g2 = g0 + sign_extend(((((low >> 20) & 0x10) | ((low >> 41) & 0xF)) as i32), 5);
            b2 = b0
                + sign_extend(
                    ((((low >> 10) & 0x10) | ((high << 3) & 8) | (low >> 61)) as i32),
                    5,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 5);
            g3 = g0 + sign_extend(((((low >> 36) & 0x10) | ((low >> 51) & 0xF)) as i32), 5);
            b3 = b0
                + sign_extend(
                    ((((low >> 30) & 0x10)
                        | ((high >> 9) & 0x08)
                        | ((high >> 4) & 0x04)
                        | ((low >> 59) & 0x02)
                        | ((low >> 50) & 0x01)) as i32),
                    5,
                );

            r0 = unquantize(r0, 9, signed);
            g0 = unquantize(g0, 9, signed);
            b0 = unquantize(b0, 9, signed);

            r1 = unquantize(r1 & 0x1FF, 9, signed);
            g1 = unquantize(g1 & 0x1FF, 9, signed);
            b1 = unquantize(b1 & 0x1FF, 9, signed);

            r2 = unquantize(r2 & 0x1FF, 9, signed);
            g2 = unquantize(g2 & 0x1FF, 9, signed);
            b2 = unquantize(b2 & 0x1FF, 9, signed);

            r3 = unquantize(r3 & 0x1FF, 9, signed);
            g3 = unquantize(g3 & 0x1FF, 9, signed);
            b3 = unquantize(b3 & 0x1FF, 9, signed);

            subset_count = 2;
        }
        15 => {
            r0 = (bit_reverse_6(((low >> 39) as i32) & 0x3F) << 10) | (((low >> 5) as i32) & 0x3FF);
            g0 =
                (bit_reverse_6(((low >> 49) as i32) & 0x3F) << 10) | (((low >> 15) as i32) & 0x3FF);
            b0 = ((bit_reverse_6((low >> 59) as i32) | ((high & 1) as i32)) << 10)
                | (((low >> 25) as i32) & 0x3FF);

            if signed {
                r0 = sign_extend(r0, 16);
                g0 = sign_extend(g0, 16);
                b0 = sign_extend(b0, 16);
            }

            r1 = (r0 + sign_extend(((low >> 35) as i32), 4)) & 0xFFFF;
            g1 = (g0 + sign_extend(((low >> 45) as i32), 4)) & 0xFFFF;
            b1 = (b0 + sign_extend(((low >> 55) as i32), 4)) & 0xFFFF;

            subset_count = 1;
        }
        18 => {
            r0 = ((low >> 5) as i32) & 0xFF;
            g0 = ((low >> 15) as i32) & 0xFF;
            b0 = ((low >> 25) as i32) & 0xFF;

            if signed {
                r0 = sign_extend(r0, 8);
                g0 = sign_extend(g0, 8);
                b0 = sign_extend(b0, 8);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 6);
            g1 = g0 + sign_extend(((low >> 45) as i32), 5);
            b1 = b0 + sign_extend(((low >> 55) as i32), 5);

            r2 = r0 + sign_extend(((high >> 1) as i32), 6);
            g2 = g0 + sign_extend(((((low >> 20) & 0x10) | ((low >> 41) & 0xF)) as i32), 5);
            b2 = b0
                + sign_extend(
                    ((((low >> 10) & 0x10) | ((high << 3) & 8) | (low >> 61)) as i32),
                    5,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 6);
            g3 = g0 + sign_extend(((((low >> 9) & 0x10) | ((low >> 51) & 0xF)) as i32), 5);
            b3 = b0
                + sign_extend(
                    ((((low >> 30) & 0x18)
                        | ((low >> 21) & 0x04)
                        | ((low >> 59) & 0x02)
                        | ((low >> 50) & 0x01)) as i32),
                    5,
                );

            r0 = unquantize(r0, 8, signed);
            g0 = unquantize(g0, 8, signed);
            b0 = unquantize(b0, 8, signed);

            r1 = unquantize(r1 & 0xFF, 8, signed);
            g1 = unquantize(g1 & 0xFF, 8, signed);
            b1 = unquantize(b1 & 0xFF, 8, signed);

            r2 = unquantize(r2 & 0xFF, 8, signed);
            g2 = unquantize(g2 & 0xFF, 8, signed);
            b2 = unquantize(b2 & 0xFF, 8, signed);

            r3 = unquantize(r3 & 0xFF, 8, signed);
            g3 = unquantize(g3 & 0xFF, 8, signed);
            b3 = unquantize(b3 & 0xFF, 8, signed);

            subset_count = 2;
        }
        22 => {
            r0 = ((low >> 5) as i32) & 0xFF;
            g0 = ((low >> 15) as i32) & 0xFF;
            b0 = ((low >> 25) as i32) & 0xFF;

            if signed {
                r0 = sign_extend(r0, 8);
                g0 = sign_extend(g0, 8);
                b0 = sign_extend(b0, 8);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 5);
            g1 = g0 + sign_extend(((low >> 45) as i32), 6);
            b1 = b0 + sign_extend(((low >> 55) as i32), 5);

            r2 = r0 + sign_extend(((high >> 1) as i32), 5);
            g2 = g0
                + sign_extend(
                    ((((low >> 18) & 0x20) | ((low >> 20) & 0x10) | ((low >> 41) & 0xF)) as i32),
                    6,
                );
            b2 = b0
                + sign_extend(
                    ((((low >> 10) & 0x10) | ((high << 3) & 0x08) | (low >> 61)) as i32),
                    5,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 5);
            g3 = g0
                + sign_extend(
                    ((((low >> 28) & 0x20) | ((low >> 36) & 0x10) | ((low >> 51) & 0x0F)) as i32),
                    6,
                );
            b3 = b0
                + sign_extend(
                    ((((low >> 30) & 0x10)
                        | ((high >> 9) & 0x08)
                        | ((high >> 4) & 0x04)
                        | ((low >> 59) & 0x02)
                        | ((low >> 13) & 0x01)) as i32),
                    5,
                );

            r0 = unquantize(r0, 8, signed);
            g0 = unquantize(g0, 8, signed);
            b0 = unquantize(b0, 8, signed);

            r1 = unquantize(r1 & 0xFF, 8, signed);
            g1 = unquantize(g1 & 0xFF, 8, signed);
            b1 = unquantize(b1 & 0xFF, 8, signed);

            r2 = unquantize(r2 & 0xFF, 8, signed);
            g2 = unquantize(g2 & 0xFF, 8, signed);
            b2 = unquantize(b2 & 0xFF, 8, signed);

            r3 = unquantize(r3 & 0xFF, 8, signed);
            g3 = unquantize(g3 & 0xFF, 8, signed);
            b3 = unquantize(b3 & 0xFF, 8, signed);

            subset_count = 2;
        }
        26 => {
            r0 = ((low >> 5) as i32) & 0xFF;
            g0 = ((low >> 15) as i32) & 0xFF;
            b0 = ((low >> 25) as i32) & 0xFF;

            if signed {
                r0 = sign_extend(r0, 8);
                g0 = sign_extend(g0, 8);
                b0 = sign_extend(b0, 8);
            }

            r1 = r0 + sign_extend(((low >> 35) as i32), 5);
            g1 = g0 + sign_extend(((low >> 45) as i32), 5);
            b1 = b0 + sign_extend(((low >> 55) as i32), 6);

            r2 = r0 + sign_extend(((high >> 1) as i32), 5);
            g2 = g0 + sign_extend(((((low >> 20) & 0x10) | ((low >> 41) & 0xF)) as i32), 5);
            b2 = b0
                + sign_extend(
                    ((((low >> 18) & 0x20)
                        | ((low >> 10) & 0x10)
                        | ((high << 3) & 0x08)
                        | (low >> 61)) as i32),
                    6,
                );

            r3 = r0 + sign_extend(((high >> 7) as i32), 5);
            g3 = g0 + sign_extend(((((low >> 36) & 0x10) | ((low >> 51) & 0xF)) as i32), 5);
            b3 = b0
                + sign_extend(
                    ((((low >> 28) & 0x20)
                        | ((low >> 30) & 0x10)
                        | ((high >> 9) & 0x08)
                        | ((high >> 4) & 0x04)
                        | ((low >> 12) & 0x02)
                        | ((low >> 50) & 0x01)) as i32),
                    6,
                );

            r0 = unquantize(r0, 8, signed);
            g0 = unquantize(g0, 8, signed);
            b0 = unquantize(b0, 8, signed);

            r1 = unquantize(r1 & 0xFF, 8, signed);
            g1 = unquantize(g1 & 0xFF, 8, signed);
            b1 = unquantize(b1 & 0xFF, 8, signed);

            r2 = unquantize(r2 & 0xFF, 8, signed);
            g2 = unquantize(g2 & 0xFF, 8, signed);
            b2 = unquantize(b2 & 0xFF, 8, signed);

            r3 = unquantize(r3 & 0xFF, 8, signed);
            g3 = unquantize(g3 & 0xFF, 8, signed);
            b3 = unquantize(b3 & 0xFF, 8, signed);

            subset_count = 2;
        }
        30 => {
            r0 = ((low >> 5) as i32) & 0x3F;
            g0 = ((low >> 15) as i32) & 0x3F;
            b0 = ((low >> 25) as i32) & 0x3F;

            r1 = ((low >> 35) as i32) & 0x3F;
            g1 = ((low >> 45) as i32) & 0x3F;
            b1 = ((low >> 55) as i32) & 0x3F;

            r2 = ((high >> 1) as i32) & 0x3F;
            g2 = ((((low >> 16) & 0x20) | ((low >> 20) & 0x10) | ((low >> 41) & 0xF)) as i32);
            b2 = ((((low >> 17) & 0x20) | ((low >> 10) & 0x10) | ((high << 3) & 0x08) | (low >> 61))
                as i32);

            r3 = ((high >> 7) as i32) & 0x3F;
            g3 = ((((low >> 26) & 0x20) | ((low >> 7) & 0x10) | ((low >> 51) & 0xF)) as i32);
            b3 = ((((low >> 28) & 0x20)
                | ((low >> 30) & 0x10)
                | ((low >> 29) & 0x08)
                | ((low >> 21) & 0x04)
                | ((low >> 12) & 0x03)) as i32);

            if signed {
                r0 = sign_extend(r0, 6);
                g0 = sign_extend(g0, 6);
                b0 = sign_extend(b0, 6);

                r1 = sign_extend(r1, 6);
                g1 = sign_extend(g1, 6);
                b1 = sign_extend(b1, 6);

                r2 = sign_extend(r2, 6);
                g2 = sign_extend(g2, 6);
                b2 = sign_extend(b2, 6);

                r3 = sign_extend(r3, 6);
                g3 = sign_extend(g3, 6);
                b3 = sign_extend(b3, 6);
            }

            r0 = unquantize(r0, 6, signed);
            g0 = unquantize(g0, 6, signed);
            b0 = unquantize(b0, 6, signed);

            r1 = unquantize(r1, 6, signed);
            g1 = unquantize(g1, 6, signed);
            b1 = unquantize(b1, 6, signed);

            r2 = unquantize(r2, 6, signed);
            g2 = unquantize(g2, 6, signed);
            b2 = unquantize(b2, 6, signed);

            r3 = unquantize(r3, 6, signed);
            g3 = unquantize(g3, 6, signed);
            b3 = unquantize(b3, 6, signed);

            subset_count = 2;
        }
        _ => {
            subset_count = 0;
        }
    }

    if (subset_count > 0) {
        end_points[0] = RgbaColor32::new(r0, g0, b0, HALF_ONE);
        end_points[1] = RgbaColor32::new(r1, g1, b1, HALF_ONE);

        if (subset_count > 1) {
            end_points[2] = RgbaColor32::new(r2, g2, b2, HALF_ONE);
            end_points[3] = RgbaColor32::new(r3, g3, b3, HALF_ONE);
        }
    }

    subset_count
}

pub fn decode_bc6h_block(block: Block, output: &mut [u64; 16], signed: bool) {
    let mut mode = (block.low & 3) as usize;
    if (mode & 2) != 0 {
        mode = (block.low & 0x1f) as usize;
    }

    let mut endpoints = [RgbaColor32::new(0, 0, 0, HALF_ONE); 4];
    let subset_count = decode_end_points(&block, &mut endpoints, mode, signed);
    if subset_count == 0 {
        let opaque_black = (HALF_ONE as u64) << 48;
        output.fill(opaque_black);
        return;
    }

    let partition: usize;
    let index_bit_count: usize;
    let mut indices: u64;

    if subset_count > 1 {
        partition = ((block.high >> 13) & 0x1f) as usize;
        index_bit_count = 3;

        let fixup_index = FIXUP_INDICES[subset_count - 1][partition][1] * 3;
        let low_mask = (u64::MAX >> (65 - fixup_index)) << 3;
        let high_mask = u64::MAX << (fixup_index + 3);

        indices = ((block.high >> 16) & high_mask)
            | ((block.high >> 17) & low_mask)
            | ((block.high >> 18) & 3);
    } else {
        partition = 0;
        index_bit_count = 4;
        indices = (block.high & !0xf) | ((block.high >> 1) & 7);
    }

    let index_mask = (1u64 << index_bit_count) - 1;

    for i in 0..16 {
        let index = (indices & index_mask) as usize;
        let end_point_base = (PARTITION_TABLE[subset_count - 1][partition][i] as usize) << 1;

        let color1 = endpoints[end_point_base];
        let color2 = endpoints[end_point_base + 1];

        let color = interpolate(color1, color2, index, index_bit_count);

        output[i] = (finish_unquantize(color.r, signed) as u64)
            | ((finish_unquantize(color.g, signed) as u64) << 16)
            | ((finish_unquantize(color.b, signed) as u64) << 32)
            | ((HALF_ONE as u64) << 48);

        indices >>= index_bit_count;
    }
}

pub fn decode_bc6h_surface(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    signed: bool,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let expected_src = blocks_x * blocks_y * 16;
    let expected_dst = width * height * 8;
    if src.len() < expected_src {
        return Err("Source buffer too small");
    }
    if dst.len() < expected_dst {
        return Err("Destination buffer too small");
    }

    let mut tile = [0u64; 16];

    let mut src_offset = 0;
    for by in 0..blocks_y {
        let y_base = by * 4;
        let bh = (height - y_base).min(4);
        for bx in 0..blocks_x {
            let x_base = bx * 4;
            let bw = (width - x_base).min(4);

            let block = Block::from_slice(&src[src_offset..src_offset + 16]);
            src_offset += 16;

            decode_bc6h_block(block, &mut tile, signed);

            store_tile(dst, &tile, width, x_base, y_base, bw, bh);
        }
    }
    Ok(())
}
