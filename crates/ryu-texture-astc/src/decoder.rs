use crate::bitstream::BitStream128;
use crate::integer_encoded::{
    bit_transfer_signed, create_encoding, decode_integer_sequence, replicate, replicate_1_7,
    replicate_8_16, reverse_byte, EIntegerEncoding, IntegerEncoded,
};
use std::cmp::{max, min};

#[derive(Debug, Clone, Copy, Default)]
pub struct AstcPixel {
    pub a: i16,
    pub r: i16,
    pub g: i16,
    pub b: i16,
}

impl AstcPixel {
    #[inline(always)]
    pub const fn new(a: i16, r: i16, g: i16, b: i16) -> Self {
        Self { a, r, g, b }
    }

    #[inline(always)]
    pub fn clamp_byte(&mut self) {
        self.r = self.r.clamp(0, 255);
        self.g = self.g.clamp(0, 255);
        self.b = self.b.clamp(0, 255);
        self.a = self.a.clamp(0, 255);
    }

    #[inline(always)]
    pub fn get_component(&self, index: usize) -> i16 {
        match index {
            0 => self.r,
            1 => self.g,
            2 => self.b,
            3 => self.a,
            _ => 0,
        }
    }

    #[inline(always)]
    pub fn set_component(&mut self, index: usize, val: i16) {
        match index {
            0 => self.r = val,
            1 => self.g = val,
            2 => self.b = val,
            3 => self.a = val,
            _ => {}
        }
    }

    #[inline(always)]
    pub fn pack(&self) -> u32 {
        ((self.a as u32 & 0xFF) << 24)
            | ((self.b as u32 & 0xFF) << 16)
            | ((self.g as u32 & 0xFF) << 8)
            | (self.r as u32 & 0xFF)
    }

    // Adds more precision to the blue channel as described in C.2.14
    #[inline(always)]
    pub fn blue_contract(a: i32, r: i32, g: i32, b: i32) -> Self {
        Self {
            a: a as i16,
            r: ((r + b) >> 1) as i16,
            g: ((g + b) >> 1) as i16,
            b: b as i16,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TexelWeightParams {
    pub width: usize,
    pub height: usize,
    pub max_weight: usize,
    pub dual_plane: bool,
    pub error: bool,
    pub void_extent_ldr: bool,
    pub void_extent_hdr: bool,
}

impl TexelWeightParams {
    #[inline(always)]
    pub fn get_packed_bit_size(&self) -> usize {
        let mut indices = self.height * self.width;
        if self.dual_plane {
            indices *= 2;
        }
        let int_encoded = create_encoding(self.max_weight);
        int_encoded.get_bit_length(indices)
    }

    #[inline(always)]
    pub fn get_num_weight_values(&self) -> usize {
        let mut ret = self.width * self.height;
        if self.dual_plane {
            ret *= 2;
        }
        ret
    }
}

pub fn decode_block_info(bit_stream: &mut BitStream128) -> TexelWeightParams {
    let mut texel_params = TexelWeightParams::default();
    let mode_bits = bit_stream.read_bits(11) as u16;

    // Does this match the void extent block mode?
    if (mode_bits & 0x01FF) == 0x1FC {
        if (mode_bits & 0x200) != 0 {
            texel_params.void_extent_hdr = true;
        } else {
            texel_params.void_extent_ldr = true;
        }

        if (mode_bits & 0x400) == 0 || bit_stream.read_bits(1) == 0 {
            texel_params.error = true;
        }
        return texel_params;
    }

    if (mode_bits & 0xF) == 0 {
        texel_params.error = true;
        return texel_params;
    }

    if (mode_bits & 0x3) == 0 && (mode_bits & 0x1C0) == 0x1C0 {
        texel_params.error = true;
        return texel_params;
    }

    let layout = if (mode_bits & 0x1) != 0 || (mode_bits & 0x2) != 0 {
        if (mode_bits & 0x8) != 0 {
            if (mode_bits & 0x4) != 0 {
                if (mode_bits & 0x100) != 0 {
                    4
                } else {
                    3
                }
            } else {
                2
            }
        } else if (mode_bits & 0x4) != 0 {
            1
        } else {
            0
        }
    } else if (mode_bits & 0x100) != 0 {
        if (mode_bits & 0x80) != 0 {
            if (mode_bits & 0x20) != 0 {
                8
            } else {
                7
            }
        } else {
            9
        }
    } else if (mode_bits & 0x80) != 0 {
        6
    } else {
        5
    };

    let mut r = ((mode_bits >> 4) & 1) as usize;
    if layout < 5 {
        r |= ((mode_bits & 0x3) as usize) << 1;
    } else {
        r |= ((mode_bits & 0xC) as usize) >> 1;
    }

    if !(2..=7).contains(&r) {
        texel_params.error = true;
        return texel_params;
    }

    match layout {
        0 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 7) & 0x3) as usize;
            texel_params.width = b + 4;
            texel_params.height = a + 2;
        }
        1 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 7) & 0x3) as usize;
            texel_params.width = b + 8;
            texel_params.height = a + 2;
        }
        2 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 7) & 0x3) as usize;
            texel_params.width = a + 2;
            texel_params.height = b + 8;
        }
        3 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 7) & 0x1) as usize;
            texel_params.width = a + 2;
            texel_params.height = b + 6;
        }
        4 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 7) & 0x1) as usize;
            texel_params.width = b + 2;
            texel_params.height = a + 2;
        }
        5 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            texel_params.width = 12;
            texel_params.height = a + 2;
        }
        6 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            texel_params.width = a + 2;
            texel_params.height = 12;
        }
        7 => {
            texel_params.width = 6;
            texel_params.height = 10;
        }
        8 => {
            texel_params.width = 10;
            texel_params.height = 6;
        }
        9 => {
            let a = ((mode_bits >> 5) & 0x3) as usize;
            let b = ((mode_bits >> 9) & 0x3) as usize;
            texel_params.width = a + 6;
            texel_params.height = b + 6;
        }
        _ => {
            texel_params.error = true;
            return texel_params;
        }
    }

    let d = layout != 9 && ((mode_bits & 0x400) != 0);
    let h = layout != 9 && ((mode_bits & 0x200) != 0);

    if h {
        const MAX_WEIGHTS: [usize; 6] = [9, 11, 15, 19, 23, 31];
        texel_params.max_weight = MAX_WEIGHTS[r - 2];
    } else {
        const MAX_WEIGHTS: [usize; 6] = [1, 2, 3, 4, 5, 7];
        texel_params.max_weight = MAX_WEIGHTS[r - 2];
    }

    texel_params.dual_plane = d;
    texel_params
}

#[inline(always)]
fn fill_void_extent_ldr(
    bit_stream: &mut BitStream128,
    output_buffer: &mut [u32],
    block_width: usize,
    block_height: usize,
) {
    for _ in 0..4 {
        bit_stream.read_bits(13);
    }

    let r = bit_stream.read_bits(16);
    let g = bit_stream.read_bits(16);
    let b = bit_stream.read_bits(16);
    let a = bit_stream.read_bits(16);

    let rgba = (r >> 8) | (g & 0xFF00) | ((b & 0xFF00) << 8) | ((a & 0xFF00) << 16);

    for j in 0..block_height {
        for i in 0..block_width {
            output_buffer[j * block_width + i] = rgba;
        }
    }
}

fn unquantize_texel_weight(int_encoded: &IntegerEncoded) -> i32 {
    let bit_value = int_encoded.bit_value as i32;
    let bit_length = int_encoded.number_bits as usize;
    let a = replicate_1_7((bit_value & 1) as usize) as i32;
    let mut b = 0i32;
    let mut c = 0i32;
    let mut d = 0i32;
    let mut result = 0i32;

    match int_encoded.encoding {
        EIntegerEncoding::JustBits => {
            result = replicate(bit_value as u32, bit_length, 6) as i32;
        }
        EIntegerEncoding::Trit => {
            d = int_encoded.trit_value as i32;
            match bit_length {
                0 => {
                    result = match d {
                        0 => 0,
                        1 => 32,
                        2 => 63,
                        _ => 0,
                    };
                }
                1 => c = 50,
                2 => {
                    c = 23;
                    let b2 = (bit_value >> 1) & 1;
                    b = (b2 << 6) | (b2 << 2) | b2;
                }
                3 => {
                    c = 11;
                    let cb = (bit_value >> 1) & 3;
                    b = (cb << 5) | cb;
                }
                _ => return 0,
            }
        }
        EIntegerEncoding::Quint => {
            d = int_encoded.quint_value as i32;
            match bit_length {
                0 => {
                    result = match d {
                        0 => 0,
                        1 => 16,
                        2 => 32,
                        3 => 47,
                        4 => 63,
                        _ => 0,
                    };
                }
                1 => c = 28,
                2 => {
                    c = 13;
                    let b2 = (bit_value >> 1) & 1;
                    b = (b2 << 6) | (b2 << 1);
                }
                _ => return 0,
            }
        }
    }

    if int_encoded.encoding != EIntegerEncoding::JustBits && bit_length > 0 {
        result = d * c + b;
        result ^= a;
        result = (a & 0x20) | (result >> 2);
    }

    debug_assert!(result < 64);
    if result > 32 {
        result += 1;
    }
    result
}

fn unquantize_texel_weights(
    output_buffer: &mut [[i32; 144]; 2],
    weights: &[IntegerEncoded],
    texel_params: &TexelWeightParams,
    block_width: usize,
    block_height: usize,
) {
    let mut unquantized = [[0i32; 144]; 2];
    let mut weight_indices = 0;
    let mut i = 0;

    while i < weights.len() {
        unquantized[0][weight_indices] = unquantize_texel_weight(&weights[i]);

        if texel_params.dual_plane {
            i += 1;
            if i < weights.len() {
                unquantized[1][weight_indices] = unquantize_texel_weight(&weights[i]);
            }
        }

        weight_indices += 1;
        if weight_indices >= texel_params.width * texel_params.height {
            break;
        }
        i += 1;
    }

    // Infill interpolation (Section C.2.18)
    let ds = (1024 + block_width / 2) / (block_width - 1);
    let dt = (1024 + block_height / 2) / (block_height - 1);
    let plane_scale = if texel_params.dual_plane { 2 } else { 1 };

    for plane in 0..plane_scale {
        for t in 0..block_height {
            for s in 0..block_width {
                let cs = ds * s;
                let ct = dt * t;

                let gs = (cs * (texel_params.width - 1) + 32) >> 6;
                let gt = (ct * (texel_params.height - 1) + 32) >> 6;

                let js = gs >> 4;
                let fs = (gs & 0xF) as i32;
                let jt = gt >> 4;
                let ft = (gt & 0xF) as i32;

                let w11 = ((fs * ft + 8) >> 4) as i32;
                let v0 = js + jt * texel_params.width;
                let mut weight = 8i32;
                let wxh = texel_params.width * texel_params.height;

                if v0 < wxh {
                    weight += unquantized[plane][v0] * (16 - fs - ft + w11);
                    if v0 + 1 < wxh {
                        weight += unquantized[plane][v0 + 1] * (fs - w11);
                    }
                }

                if v0 + texel_params.width < wxh {
                    weight += unquantized[plane][v0 + texel_params.width] * (ft - w11);
                    if v0 + texel_params.width + 1 < wxh {
                        weight += unquantized[plane][v0 + texel_params.width + 1] * w11;
                    }
                }

                output_buffer[plane][t * block_width + s] = weight >> 4;
            }
        }
    }
}

#[inline(always)]
fn hash52(mut val: u32) -> i32 {
    val ^= val >> 15;
    val = val.wrapping_sub(val << 17);
    val = val.wrapping_add(val << 7);
    val = val.wrapping_add(val << 4);
    val ^= val >> 5;
    val = val.wrapping_add(val << 16);
    val ^= val >> 7;
    val ^= val >> 3;
    val ^= val << 6;
    val ^= val >> 17;
    val as i32
}

/// ASTC partition selection, ported from `AstcDecoder.SelectPartition` in
/// Ryujinx.Graphics.Texture/Astc (which follows the ASTC specification).
fn select_partition(
    seed: i32,
    mut x: i32,
    mut y: i32,
    mut z: i32,
    partition_count: usize,
    is_small_block: bool,
) -> usize {
    if partition_count == 1 {
        return 0;
    }

    if is_small_block {
        x <<= 1;
        y <<= 1;
        z <<= 1;
    }

    let seed = seed + (partition_count as i32 - 1) * 1024;

    let right_num = hash52(seed as u32);
    let nibble = |shift: u32| ((right_num as u32 >> shift) & 0xF) as i32;
    let mut seeds = [
        nibble(0),
        nibble(4),
        nibble(8),
        nibble(12),
        nibble(16),
        nibble(20),
        nibble(24),
        nibble(28),
        nibble(18),
        nibble(22),
        nibble(26),
        ((right_num as u32).rotate_left(2) & 0xF) as i32,
    ];
    for s in &mut seeds {
        *s *= *s;
    }

    let (seed_hash1, seed_hash2) = if (seed & 1) != 0 {
        (
            if (seed & 2) != 0 { 4 } else { 5 },
            if partition_count == 3 { 6 } else { 5 },
        )
    } else {
        (
            if partition_count == 3 { 6 } else { 5 },
            if (seed & 2) != 0 { 4 } else { 5 },
        )
    };
    let seed_hash3 = if (seed & 0x10) != 0 {
        seed_hash1
    } else {
        seed_hash2
    };

    let [s01, s02, s03, s04, s05, s06, s07, s08, s09, s10, s11, s12] = seeds;
    let (s01, s03, s05, s07) = (
        s01 >> seed_hash1,
        s03 >> seed_hash1,
        s05 >> seed_hash1,
        s07 >> seed_hash1,
    );
    let (s02, s04, s06, s08) = (
        s02 >> seed_hash2,
        s04 >> seed_hash2,
        s06 >> seed_hash2,
        s08 >> seed_hash2,
    );
    let (s09, s10, s11, s12) = (
        s09 >> seed_hash3,
        s10 >> seed_hash3,
        s11 >> seed_hash3,
        s12 >> seed_hash3,
    );

    let a = (s01 * x + s02 * y + s11 * z + (right_num >> 14)) & 0x3F;
    let b = (s03 * x + s04 * y + s12 * z + (right_num >> 10)) & 0x3F;
    let mut c = (s05 * x + s06 * y + s09 * z + (right_num >> 6)) & 0x3F;
    let mut d = (s07 * x + s08 * y + s10 * z + (right_num >> 2)) & 0x3F;

    if partition_count < 4 {
        d = 0;
    }
    if partition_count < 3 {
        c = 0;
    }

    if a >= b && a >= c && a >= d {
        0
    } else if b >= c && b >= d {
        1
    } else if c >= d {
        2
    } else {
        3
    }
}

fn compute_endpoints(
    endpoints: &mut [AstcPixel; 2],
    color_values: &[i32],
    color_endpoint_mode: u32,
    pos: &mut usize,
) -> bool {
    match color_endpoint_mode {
        0 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            *pos += 2;
            endpoints[0] = AstcPixel::new(0xFF, v0 as i16, v0 as i16, v0 as i16);
            endpoints[1] = AstcPixel::new(0xFF, v1 as i16, v1 as i16, v1 as i16);
        }
        1 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            *pos += 2;
            let l0 = ((v0 >> 2) | (v1 & 0xC0)) as i32;
            let l1 = min(l0 + (v1 & 0x3F) as i32, 0xFF);
            endpoints[0] = AstcPixel::new(0xFF, l0 as i16, l0 as i16, l0 as i16);
            endpoints[1] = AstcPixel::new(0xFF, l1 as i16, l1 as i16, l1 as i16);
        }
        4 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            let v2 = color_values[*pos + 2] as u32;
            let v3 = color_values[*pos + 3] as u32;
            *pos += 4;
            endpoints[0] = AstcPixel::new(v2 as i16, v0 as i16, v0 as i16, v0 as i16);
            endpoints[1] = AstcPixel::new(v3 as i16, v1 as i16, v1 as i16, v1 as i16);
        }
        5 => {
            let v0 = color_values[*pos];
            let v1 = color_values[*pos + 1];
            let v2 = color_values[*pos + 2];
            let v3 = color_values[*pos + 3];
            *pos += 4;
            let (v1, v0) = bit_transfer_signed(v1, v0);
            let (v3, v2) = bit_transfer_signed(v3, v2);
            endpoints[0] = AstcPixel::new(v2 as i16, v0 as i16, v0 as i16, v0 as i16);
            endpoints[1] = AstcPixel::new(
                (v2 + v3) as i16,
                (v0 + v1) as i16,
                (v0 + v1) as i16,
                (v0 + v1) as i16,
            );
            endpoints[0].clamp_byte();
            endpoints[1].clamp_byte();
        }
        6 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            let v2 = color_values[*pos + 2] as u32;
            let v3 = color_values[*pos + 3] as u32;
            *pos += 4;
            endpoints[0] = AstcPixel::new(
                0xFF,
                ((v0 * v3) >> 8) as i16,
                ((v1 * v3) >> 8) as i16,
                ((v2 * v3) >> 8) as i16,
            );
            endpoints[1] = AstcPixel::new(0xFF, v0 as i16, v1 as i16, v2 as i16);
        }
        8 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            let v2 = color_values[*pos + 2] as u32;
            let v3 = color_values[*pos + 3] as u32;
            let v4 = color_values[*pos + 4] as u32;
            let v5 = color_values[*pos + 5] as u32;
            *pos += 6;
            if v1 + v3 + v5 >= v0 + v2 + v4 {
                endpoints[0] = AstcPixel::new(0xFF, v0 as i16, v2 as i16, v4 as i16);
                endpoints[1] = AstcPixel::new(0xFF, v1 as i16, v3 as i16, v5 as i16);
            } else {
                endpoints[0] = AstcPixel::blue_contract(0xFF, v1 as i32, v3 as i32, v5 as i32);
                endpoints[1] = AstcPixel::blue_contract(0xFF, v0 as i32, v2 as i32, v4 as i32);
            }
        }
        9 => {
            let v0 = color_values[*pos];
            let v1 = color_values[*pos + 1];
            let v2 = color_values[*pos + 2];
            let v3 = color_values[*pos + 3];
            let v4 = color_values[*pos + 4];
            let v5 = color_values[*pos + 5];
            *pos += 6;
            let (v1, v0) = bit_transfer_signed(v1, v0);
            let (v3, v2) = bit_transfer_signed(v3, v2);
            let (v5, v4) = bit_transfer_signed(v5, v4);

            if v1 + v3 + v5 >= 0 {
                endpoints[0] = AstcPixel::new(0xFF, v0 as i16, v2 as i16, v4 as i16);
                endpoints[1] =
                    AstcPixel::new(0xFF, (v0 + v1) as i16, (v2 + v3) as i16, (v4 + v5) as i16);
            } else {
                endpoints[0] = AstcPixel::blue_contract(0xFF, v0 + v1, v2 + v3, v4 + v5);
                endpoints[1] = AstcPixel::blue_contract(0xFF, v0, v2, v4);
            }
            endpoints[0].clamp_byte();
            endpoints[1].clamp_byte();
        }
        10 => {
            let v0 = color_values[*pos] as u32;
            let v1 = color_values[*pos + 1] as u32;
            let v2 = color_values[*pos + 2] as u32;
            let v3 = color_values[*pos + 3] as u32;
            let v4 = color_values[*pos + 4] as u32;
            let v5 = color_values[*pos + 5] as u32;
            *pos += 6;
            endpoints[0] = AstcPixel::new(
                v4 as i16,
                ((v0 * v3) >> 8) as i16,
                ((v1 * v3) >> 8) as i16,
                ((v2 * v3) >> 8) as i16,
            );
            endpoints[1] = AstcPixel::new(v5 as i16, v0 as i16, v1 as i16, v2 as i16);
        }
        12 => {
            let v = [
                color_values[*pos] as u32,
                color_values[*pos + 1] as u32,
                color_values[*pos + 2] as u32,
                color_values[*pos + 3] as u32,
                color_values[*pos + 4] as u32,
                color_values[*pos + 5] as u32,
                color_values[*pos + 6] as u32,
                color_values[*pos + 7] as u32,
            ];
            *pos += 8;
            if v[1] + v[3] + v[5] >= v[0] + v[2] + v[4] {
                endpoints[0] = AstcPixel::new(v[6] as i16, v[0] as i16, v[2] as i16, v[4] as i16);
                endpoints[1] = AstcPixel::new(v[7] as i16, v[1] as i16, v[3] as i16, v[5] as i16);
            } else {
                endpoints[0] =
                    AstcPixel::blue_contract(v[7] as i32, v[1] as i32, v[3] as i32, v[5] as i32);
                endpoints[1] =
                    AstcPixel::blue_contract(v[6] as i32, v[0] as i32, v[2] as i32, v[4] as i32);
            }
        }
        13 => {
            let v0 = color_values[*pos];
            let v1 = color_values[*pos + 1];
            let v2 = color_values[*pos + 2];
            let v3 = color_values[*pos + 3];
            let v4 = color_values[*pos + 4];
            let v5 = color_values[*pos + 5];
            let v6 = color_values[*pos + 6];
            let v7 = color_values[*pos + 7];
            *pos += 8;
            let (v1, v0) = bit_transfer_signed(v1, v0);
            let (v3, v2) = bit_transfer_signed(v3, v2);
            let (v5, v4) = bit_transfer_signed(v5, v4);
            let (v7, v6) = bit_transfer_signed(v7, v6);

            if v1 + v3 + v5 >= 0 {
                endpoints[0] = AstcPixel::new(v6 as i16, v0 as i16, v2 as i16, v4 as i16);
                endpoints[1] = AstcPixel::new(
                    (v7 + v6) as i16,
                    (v0 + v1) as i16,
                    (v2 + v3) as i16,
                    (v4 + v5) as i16,
                );
            } else {
                endpoints[0] = AstcPixel::blue_contract(v6 + v7, v0 + v1, v2 + v3, v4 + v5);
                endpoints[1] = AstcPixel::blue_contract(v6, v0, v2, v4);
            }
            endpoints[0].clamp_byte();
            endpoints[1].clamp_byte();
        }
        _ => return false,
    }
    true
}

/// Decodes the colour endpoint integers into `output_values`.
///
/// Returns `false` if the block encodes more colour values than `output_values` can
/// hold (an illegal block; the C# reference throws an index exception here).
fn decode_color_values(
    output_values: &mut [i32],
    color_bit_stream: &mut BitStream128,
    modes: &[u32],
    number_partitions: usize,
    number_bits_for_color_data: usize,
) -> bool {
    let mut number_values = 0;
    for &mode in &modes[..number_partitions] {
        number_values += (((mode >> 2) + 1) as usize) << 1;
    }

    let mut range = 256usize;
    while range > 1 {
        range -= 1;
        let int_encoded = create_encoding(range);
        let bit_length = int_encoded.get_bit_length(number_values);
        if bit_length <= number_bits_for_color_data {
            while range > 1 {
                range -= 1;
                let new_encoded = create_encoding(range);
                if !new_encoded.matches_encoding(&int_encoded) {
                    break;
                }
            }
            range += 1;
            break;
        }
    }

    let mut integer_encoded_sequence = [IntegerEncoded::new(EIntegerEncoding::JustBits, 0); 64];
    let mut count = 0;
    decode_integer_sequence(
        &mut integer_encoded_sequence,
        &mut count,
        color_bit_stream,
        range,
        number_values,
    );

    if count > output_values.len() {
        return false;
    }

    let mut output_indices = 0;
    for int_encoded in &integer_encoded_sequence[..count] {
        let bit_length = int_encoded.number_bits as usize;
        let bit_value = int_encoded.bit_value as i32;
        let a = replicate((bit_value & 1) as u32, 1, 9) as i32;

        match int_encoded.encoding {
            EIntegerEncoding::JustBits => {
                output_values[output_indices] = replicate(bit_value as u32, bit_length, 8) as i32;
                output_indices += 1;
            }
            EIntegerEncoding::Trit => {
                let d = int_encoded.trit_value as i32;
                let (c, b) = match bit_length {
                    1 => (204, 0),
                    2 => {
                        let b2 = (bit_value >> 1) & 1;
                        (93, (b2 << 8) | (b2 << 4) | (b2 << 2) | (b2 << 1))
                    }
                    3 => {
                        let cb = (bit_value >> 1) & 3;
                        (44, (cb << 7) | (cb << 2) | cb)
                    }
                    4 => {
                        let dcb = (bit_value >> 1) & 7;
                        (22, (dcb << 6) | dcb)
                    }
                    5 => {
                        let edcb = (bit_value >> 1) & 0xF;
                        (11, (edcb << 5) | (edcb >> 2))
                    }
                    6 => {
                        let fedcb = (bit_value >> 1) & 0x1F;
                        (5, (fedcb << 4) | (fedcb >> 4))
                    }
                    _ => (0, 0),
                };

                let mut t = d * c + b;
                t ^= a;
                t = (a & 0x80) | (t >> 2);
                output_values[output_indices] = t;
                output_indices += 1;
            }
            EIntegerEncoding::Quint => {
                let d = int_encoded.quint_value as i32;
                let (c, b) = match bit_length {
                    1 => (113, 0),
                    2 => {
                        let b2 = (bit_value >> 1) & 1;
                        (54, (b2 << 8) | (b2 << 3) | (b2 << 2))
                    }
                    3 => {
                        let cb = (bit_value >> 1) & 3;
                        (26, (cb << 7) | (cb << 1) | (cb >> 1))
                    }
                    4 => {
                        let dcb = (bit_value >> 1) & 7;
                        (13, (dcb << 6) | (dcb >> 1))
                    }
                    5 => {
                        let edcb = (bit_value >> 1) & 0xF;
                        (6, (edcb << 5) | (edcb >> 3))
                    }
                    _ => (0, 0),
                };

                let mut t = d * c + b;
                t ^= a;
                t = (a & 0x80) | (t >> 2);
                output_values[output_indices] = t;
                output_indices += 1;
            }
        }
    }
    true
}

/// ASTC error colour (opaque magenta) as packed RGBA8, written for illegal blocks.
pub const ASTC_ERROR_COLOR: u32 = 0xFFFF_00FF;

/// Decompresses one 16-byte ASTC block into `output_buffer` (RGBA8 packed as `u32`).
///
/// Returns `false` for illegal or unsupported blocks; in that case the first
/// `block_width * block_height` texels of `output_buffer` are filled with the ASTC
/// error colour ([`ASTC_ERROR_COLOR`], opaque magenta) as the ASTC specification
/// requires. Never panics for any 16-byte input, provided `output_buffer` holds at
/// least `block_width * block_height` texels.
pub fn decompress_block(
    input_block: &[u8; 16],
    output_buffer: &mut [u32],
    block_width: usize,
    block_height: usize,
) -> bool {
    let ok = decompress_block_inner(input_block, output_buffer, block_width, block_height);
    if !ok {
        let texels = min(block_width * block_height, output_buffer.len());
        output_buffer[..texels].fill(ASTC_ERROR_COLOR);
    }
    ok
}

fn decompress_block_inner(
    input_block: &[u8; 16],
    output_buffer: &mut [u32],
    block_width: usize,
    block_height: usize,
) -> bool {
    let mut bit_stream = BitStream128::new(input_block);
    let texel_params = decode_block_info(&mut bit_stream);

    if texel_params.error {
        return false;
    }

    if texel_params.void_extent_ldr {
        fill_void_extent_ldr(&mut bit_stream, output_buffer, block_width, block_height);
        return true;
    }

    if texel_params.void_extent_hdr
        || texel_params.width > block_width
        || texel_params.height > block_height
    {
        return false;
    }

    let number_partitions = bit_stream.read_bits(2) as usize + 1;
    if number_partitions == 4 && texel_params.dual_plane {
        return false;
    }

    let mut color_endpoint_mode = [0u32; 4];
    let partition_index;
    let mut base_color_endpoint_mode = 0u32;

    if number_partitions == 1 {
        color_endpoint_mode[0] = bit_stream.read_bits(4);
        partition_index = 0;
    } else {
        partition_index = bit_stream.read_bits(10) as i32;
        base_color_endpoint_mode = bit_stream.read_bits(6);
    }

    let base_mode = base_color_endpoint_mode & 3;
    let number_weight_bits = texel_params.get_packed_bit_size();
    if bit_stream.bits_left() < number_weight_bits {
        return false;
    }
    let mut remaining_bits = bit_stream.bits_left() - number_weight_bits;

    let mut extra_color_endpoint_mode_bits = 0usize;
    if base_mode != 0 {
        match number_partitions {
            2 => extra_color_endpoint_mode_bits += 2,
            3 => extra_color_endpoint_mode_bits += 5,
            4 => extra_color_endpoint_mode_bits += 8,
            _ => return false,
        }
    }

    if remaining_bits < extra_color_endpoint_mode_bits {
        return false;
    }
    remaining_bits -= extra_color_endpoint_mode_bits;

    let plane_selector_bits = if texel_params.dual_plane { 2 } else { 0 };
    if remaining_bits < plane_selector_bits {
        return false;
    }
    remaining_bits -= plane_selector_bits;

    let color_data_bits = remaining_bits;
    let mut color_endpoint_stream = BitStream128::empty();
    while remaining_bits > 0 {
        let n = min(remaining_bits, 8);
        let bits = bit_stream.read_bits(n);
        color_endpoint_stream.write_bits(bits, n);
        remaining_bits -= n;
    }

    let plane_indices = bit_stream.read_bits(plane_selector_bits) as usize;

    if base_mode != 0 {
        let extra_color_endpoint_mode = bit_stream.read_bits(extra_color_endpoint_mode_bits);
        let mut temp_cem = (extra_color_endpoint_mode << 6) | base_color_endpoint_mode;
        temp_cem >>= 2;

        let mut c = [false; 4];
        for item in c.iter_mut().take(number_partitions) {
            *item = (temp_cem & 1) != 0;
            temp_cem >>= 1;
        }

        let mut m = [0u8; 4];
        for item in m.iter_mut().take(number_partitions) {
            *item = (temp_cem & 3) as u8;
            temp_cem >>= 2;
        }

        for i in 0..number_partitions {
            color_endpoint_mode[i] = base_mode;
            if !c[i] {
                color_endpoint_mode[i] -= 1;
            }
            color_endpoint_mode[i] <<= 2;
            color_endpoint_mode[i] |= m[i] as u32;
        }
    } else if number_partitions > 1 {
        let temp_cem = base_color_endpoint_mode >> 2;
        for item in color_endpoint_mode.iter_mut().take(number_partitions) {
            *item = temp_cem;
        }
    }

    let mut color_values = [0i32; 32];
    if !decode_color_values(
        &mut color_values,
        &mut color_endpoint_stream,
        &color_endpoint_mode,
        number_partitions,
        color_data_bits,
    ) {
        return false;
    }

    let mut endpoints = [[AstcPixel::default(); 2]; 4];
    let mut color_pos = 0;
    for i in 0..number_partitions {
        if !compute_endpoints(
            &mut endpoints[i],
            &color_values,
            color_endpoint_mode[i],
            &mut color_pos,
        ) {
            return false;
        }
    }

    // Read texel weight data in reverse
    let mut texel_weight_data = *input_block;
    for i in 0..8 {
        let a = reverse_byte(texel_weight_data[i]);
        let b = reverse_byte(texel_weight_data[15 - i]);
        texel_weight_data[i] = b;
        texel_weight_data[15 - i] = a;
    }

    let clear_byte_start = (texel_params.get_packed_bit_size() >> 3) + 1;
    if clear_byte_start <= 16 {
        texel_weight_data[clear_byte_start - 1] &=
            ((1 << (texel_params.get_packed_bit_size() % 8)) - 1) as u8;
        for item in texel_weight_data.iter_mut().take(16).skip(clear_byte_start) {
            *item = 0;
        }
    }

    let mut weight_bit_stream = BitStream128::new(&texel_weight_data);
    let mut texel_weight_values = [IntegerEncoded::new(EIntegerEncoding::JustBits, 0); 144];
    let mut num_weights = 0;
    decode_integer_sequence(
        &mut texel_weight_values,
        &mut num_weights,
        &mut weight_bit_stream,
        texel_params.max_weight,
        texel_params.get_num_weight_values(),
    );

    let mut weights = [[0i32; 144]; 2];
    unquantize_texel_weights(
        &mut weights,
        &texel_weight_values[..num_weights],
        &texel_params,
        block_width,
        block_height,
    );

    let is_small_block = (block_height * block_width) < 32;

    // Interpolate texels and write RGBA8 output
    for j in 0..block_height {
        for i in 0..block_width {
            let partition = select_partition(
                partition_index,
                i as i32,
                j as i32,
                0,
                number_partitions,
                is_small_block,
            );
            let mut pixel = AstcPixel::default();

            for component in 0..4 {
                let comp0 = replicate_8_16(endpoints[partition][0].get_component(component) as usize)
                    as i32;
                let comp1 = replicate_8_16(endpoints[partition][1].get_component(component) as usize)
                    as i32;

                let plane = if texel_params.dual_plane && (((plane_indices + 1) & 3) == component) {
                    1
                } else {
                    0
                };

                let weight = weights[plane][j * block_width + i];
                let final_component = (comp0 * (64 - weight) + comp1 * weight + 32) / 64;

                if final_component == 65535 {
                    pixel.set_component(component, 255);
                } else {
                    let final_component_float = final_component as f64;
                    let val = (255.0 * (final_component_float / 65536.0) + 0.5) as i16;
                    pixel.set_component(component, val);
                }
            }

            output_buffer[j * block_width + i] = pixel.pack();
        }
    }

    true
}

#[derive(Debug, Clone, Copy)]
pub struct AstcLevel {
    pub image_size_x: usize,
    pub image_size_y: usize,
    pub image_size_z: usize,
    pub block_count_x: usize,
    pub block_count_y: usize,
    pub start_block: usize,
    pub output_byte_offset: usize,
}

impl AstcLevel {
    #[inline(always)]
    pub fn total_block_count(&self) -> usize {
        self.block_count_x * self.block_count_y * self.image_size_z
    }

    #[inline(always)]
    pub fn pixel_count(&self) -> usize {
        self.image_size_x * self.image_size_y * self.image_size_z
    }
}

pub fn query_decompressed_size(
    width: usize,
    height: usize,
    depth: usize,
    levels: usize,
    layers: usize,
) -> usize {
    let mut total_pixels = 0;
    for i in 0..levels {
        let level_w = max(1, width >> i);
        let level_h = max(1, height >> i);
        let level_d = max(1, depth >> i);
        total_pixels += level_w * level_h * level_d * layers;
    }
    total_pixels * 4
}

pub fn calculate_levels(
    width: usize,
    height: usize,
    depth: usize,
    levels: usize,
    layers: usize,
    block_width: usize,
    block_height: usize,
) -> (Vec<AstcLevel>, usize) {
    let mut astc_levels = Vec::with_capacity(levels * layers);
    let mut current_input_block = 0;
    let mut current_output_offset = 0;

    for i in 0..levels {
        for _ in 0..layers {
            let img_x = max(1, width >> i);
            let img_y = max(1, height >> i);
            let img_z = max(1, depth >> i);
            let blk_x = (img_x + block_width - 1) / block_width;
            let blk_y = (img_y + block_height - 1) / block_height;

            let level = AstcLevel {
                image_size_x: img_x,
                image_size_y: img_y,
                image_size_z: img_z,
                block_count_x: blk_x,
                block_count_y: blk_y,
                start_block: current_input_block,
                output_byte_offset: current_output_offset,
            };

            current_input_block += level.total_block_count();
            current_output_offset += level.pixel_count() * 4;
            astc_levels.push(level);
        }
    }

    (astc_levels, current_input_block)
}

fn write_decompressed_block(
    block_pixels: &[u32],
    output: &mut [u8],
    block_index: usize,
    level: &AstcLevel,
    block_width: usize,
    block_height: usize,
) {
    let stride = level.image_size_x * 4;
    let block_coord_x = block_index % level.block_count_x;
    let block_coord_y = block_index / level.block_count_x;

    let pixel_coord_x = block_coord_x * block_width;
    let pixel_coord_y = block_coord_y * block_height;

    let output_pixels_x = min(pixel_coord_x + block_width, level.image_size_x) - pixel_coord_x;
    let output_pixels_y = min(
        pixel_coord_y + block_height,
        level.image_size_y * level.image_size_z,
    ) - pixel_coord_y;

    let output_start = pixel_coord_x * 4 + pixel_coord_y * stride;
    let mut output_offset = output_start;
    let mut input_offset = 0;

    for _ in 0..output_pixels_y {
        let src_bytes =
            bytemuck_cast_u32_slice(&block_pixels[input_offset..input_offset + output_pixels_x]);
        let dest = &mut output[output_offset..output_offset + output_pixels_x * 4];
        dest.copy_from_slice(src_bytes);

        input_offset += block_width;
        output_offset += stride;
    }
}

#[inline(always)]
fn bytemuck_cast_u32_slice(slice: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4) }
}

pub fn decode_astc_to_rgba8(
    data: &[u8],
    output: &mut [u8],
    block_width: usize,
    block_height: usize,
    width: usize,
    height: usize,
    depth: usize,
    levels: usize,
    layers: usize,
) -> bool {
    let (astc_levels, total_blocks) = calculate_levels(
        width,
        height,
        depth,
        levels,
        layers,
        block_width,
        block_height,
    );
    if data.len() < total_blocks * 16 {
        return false;
    }

    let mut decompressed_data = [0u32; 144];
    let mut current_level_idx = 0;

    for block_idx in 0..total_blocks {
        while current_level_idx + 1 < astc_levels.len()
            && block_idx
                >= astc_levels[current_level_idx].start_block
                    + astc_levels[current_level_idx].total_block_count()
        {
            current_level_idx += 1;
        }

        let level = &astc_levels[current_level_idx];
        let block_bytes: &[u8; 16] = data[block_idx * 16..(block_idx + 1) * 16]
            .try_into()
            .unwrap();

        // Illegal blocks come back filled with the ASTC error colour.
        decompress_block(
            block_bytes,
            &mut decompressed_data,
            block_width,
            block_height,
        );

        let local_block_idx = block_idx - level.start_block;
        write_decompressed_block(
            &decompressed_data,
            &mut output[level.output_byte_offset..],
            local_block_idx,
            level,
            block_width,
            block_height,
        );
    }

    true
}

#[cfg(feature = "parallel")]
pub fn decode_astc_to_rgba8_parallel(
    data: &[u8],
    output: &mut [u8],
    block_width: usize,
    block_height: usize,
    width: usize,
    height: usize,
    depth: usize,
    levels: usize,
    layers: usize,
) -> bool {
    use rayon::prelude::*;

    let (astc_levels, total_blocks) = calculate_levels(
        width,
        height,
        depth,
        levels,
        layers,
        block_width,
        block_height,
    );
    if data.len() < total_blocks * 16 {
        return false;
    }

    let output_ptr = output.as_mut_ptr() as usize;

    (0..total_blocks).into_par_iter().for_each(|block_idx| {
        let mut level_idx = 0;
        for (i, lvl) in astc_levels.iter().enumerate() {
            if block_idx < lvl.start_block + lvl.total_block_count() {
                level_idx = i;
                break;
            }
        }
        let level = &astc_levels[level_idx];
        let block_bytes: &[u8; 16] = data[block_idx * 16..(block_idx + 1) * 16]
            .try_into()
            .unwrap();

        let mut decompressed_data = [0u32; 144];
        // Illegal blocks come back filled with the ASTC error colour.
        decompress_block(
            block_bytes,
            &mut decompressed_data,
            block_width,
            block_height,
        );

        let local_block_idx = block_idx - level.start_block;
        unsafe {
            let out_slice = std::slice::from_raw_parts_mut(
                (output_ptr + level.output_byte_offset) as *mut u8,
                level.pixel_count() * 4,
            );
            write_decompressed_block(
                &decompressed_data,
                out_slice,
                local_block_idx,
                level,
                block_width,
                block_height,
            );
        }
    });

    true
}
