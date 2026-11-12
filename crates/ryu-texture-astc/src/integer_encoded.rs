use crate::bitstream::BitStream128;
pub use crate::encodings_data::{QUINT_ENCODINGS, TRIT_ENCODINGS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EIntegerEncoding {
    JustBits,
    Quint,
    Trit,
}

#[derive(Debug, Clone, Copy)]
pub struct IntegerEncoded {
    pub encoding: EIntegerEncoding,
    pub number_bits: u8,
    pub trit_value: u8,
    pub quint_value: u8,
    pub bit_value: u32,
}

impl IntegerEncoded {
    pub const fn new(encoding: EIntegerEncoding, num_bits: u8) -> Self {
        Self {
            encoding,
            number_bits: num_bits,
            trit_value: 0,
            quint_value: 0,
            bit_value: 0,
        }
    }

    #[inline(always)]
    pub fn matches_encoding(&self, other: &Self) -> bool {
        self.encoding == other.encoding && self.number_bits == other.number_bits
    }

    #[inline(always)]
    pub fn get_bit_length(&self, number_vals: usize) -> usize {
        let mut total_bits = (self.number_bits as usize) * number_vals;
        match self.encoding {
            EIntegerEncoding::Trit => {
                total_bits += (number_vals * 8).div_ceil(5);
            }
            EIntegerEncoding::Quint => {
                total_bits += (number_vals * 7).div_ceil(3);
            }
            EIntegerEncoding::JustBits => {}
        }
        total_bits
    }
}

// Generate the 256-entry encoding table at compile-time
pub static ENCODINGS: [IntegerEncoded; 256] = {
    let mut table = [IntegerEncoded::new(EIntegerEncoding::JustBits, 0); 256];
    let mut i = 0;
    while i < 256 {
        table[i] = create_encoding_calc(i);
        i += 1;
    }
    table
};

const fn create_encoding_calc(mut max_val: usize) -> IntegerEncoded {
    while max_val > 0 {
        let check = max_val + 1;

        // Is max_val a power of two?
        if (check & (check - 1)) == 0 {
            return IntegerEncoded::new(
                EIntegerEncoding::JustBits,
                (max_val as u32).count_ones() as u8,
            );
        }

        // Is max_val of the type 3*2^n - 1?
        if check.is_multiple_of(3) && (((check / 3) & ((check / 3) - 1)) == 0) {
            return IntegerEncoded::new(
                EIntegerEncoding::Trit,
                ((check / 3 - 1) as u32).count_ones() as u8,
            );
        }

        // Is max_val of the type 5*2^n - 1?
        if check.is_multiple_of(5) && (((check / 5) & ((check / 5) - 1)) == 0) {
            return IntegerEncoded::new(
                EIntegerEncoding::Quint,
                ((check / 5 - 1) as u32).count_ones() as u8,
            );
        }

        max_val -= 1;
    }

    IntegerEncoded::new(EIntegerEncoding::JustBits, 0)
}

#[inline(always)]
pub fn create_encoding(max_val: usize) -> IntegerEncoded {
    debug_assert!(max_val < 256);
    ENCODINGS[max_val]
}

pub const fn replicate(value: u32, mut number_bits: usize, to_bit: usize) -> u32 {
    if number_bits == 0 || to_bit == 0 {
        return 0;
    }

    let temp_value = value & ((1 << number_bits) - 1);
    let mut ret_value = temp_value;
    let mut res_length = number_bits;

    while res_length < to_bit {
        let mut comp = 0;
        if number_bits > to_bit - res_length {
            let new_shift = to_bit - res_length;
            comp = number_bits - new_shift;
            number_bits = new_shift;
        }

        ret_value <<= number_bits;
        ret_value |= temp_value >> comp;
        res_length += number_bits;
    }

    ret_value
}

pub static REPLICATE_8_16_TABLE: [u16; 512] = {
    let mut table = [0u16; 512];
    let mut i = 0;
    while i < 512 {
        table[i] = replicate(i as u32, 8, 16) as u16;
        i += 1;
    }
    table
};

pub static REPLICATE_1_7_TABLE: [u8; 512] = {
    let mut table = [0u8; 512];
    let mut i = 0;
    while i < 512 {
        table[i] = replicate(i as u32, 1, 7) as u8;
        i += 1;
    }
    table
};

#[inline(always)]
pub fn replicate_8_16(val: usize) -> u16 {
    debug_assert!(val < 512);
    REPLICATE_8_16_TABLE[val]
}

#[inline(always)]
pub fn replicate_1_7(val: usize) -> u8 {
    debug_assert!(val < 512);
    REPLICATE_1_7_TABLE[val]
}

// Transfers a bit as described in C.2.14
#[inline(always)]
pub fn bit_transfer_signed(mut a: i32, mut b: i32) -> (i32, i32) {
    b >>= 1;
    b |= a & 0x80;
    a >>= 1;
    a &= 0x3F;
    if (a & 0x20) != 0 {
        a -= 0x40;
    }
    (a, b)
}

// Reverses bits in a byte
#[inline(always)]
pub fn reverse_byte(b: u8) -> u8 {
    b.reverse_bits()
}

#[inline(always)]
pub fn get_trit_encoding(index: usize) -> &'static [u8] {
    &TRIT_ENCODINGS[index * 5..index * 5 + 5]
}

#[inline(always)]
pub fn get_quint_encoding(index: usize) -> &'static [u8] {
    &QUINT_ENCODINGS[index * 3..index * 3 + 3]
}

pub fn decode_integer_sequence(
    output: &mut [IntegerEncoded],
    output_count: &mut usize,
    bit_stream: &mut BitStream128,
    max_range: usize,
    number_values: usize,
) {
    let int_encoded = create_encoding(max_range);
    let mut number_values_decoded = 0;

    while number_values_decoded < number_values {
        match int_encoded.encoding {
            EIntegerEncoding::Quint => {
                let interleaved_bits: [usize; 3] = [3, 2, 2];
                let mut m = [0u32; 3];
                let mut encoded = 0u64;
                let mut encoded_bits_read = 0;

                for i in 0..3 {
                    m[i] = bit_stream.read_bits(int_encoded.number_bits as usize);
                    let encoded_bits = bit_stream.read_bits(interleaved_bits[i]) as u64;
                    encoded |= encoded_bits << encoded_bits_read;
                    encoded_bits_read += interleaved_bits[i];
                }

                let encodings = get_quint_encoding(encoded as usize);
                for i in 0..3 {
                    let mut item =
                        IntegerEncoded::new(EIntegerEncoding::Quint, int_encoded.number_bits);
                    item.bit_value = m[i];
                    item.quint_value = encodings[i];
                    if *output_count < output.len() {
                        output[*output_count] = item;
                        *output_count += 1;
                    }
                }
                number_values_decoded += 3;
            }
            EIntegerEncoding::Trit => {
                let mut m = [0u32; 5];
                m[0] = bit_stream.read_bits(int_encoded.number_bits as usize);
                let mut encoded = bit_stream.read_bits(2) as usize;
                m[1] = bit_stream.read_bits(int_encoded.number_bits as usize);
                encoded |= (bit_stream.read_bits(2) as usize) << 2;
                m[2] = bit_stream.read_bits(int_encoded.number_bits as usize);
                encoded |= (bit_stream.read_bits(1) as usize) << 4;
                m[3] = bit_stream.read_bits(int_encoded.number_bits as usize);
                encoded |= (bit_stream.read_bits(2) as usize) << 5;
                m[4] = bit_stream.read_bits(int_encoded.number_bits as usize);
                encoded |= (bit_stream.read_bits(1) as usize) << 7;

                let encodings = get_trit_encoding(encoded);
                let mut item = IntegerEncoded::new(EIntegerEncoding::Trit, int_encoded.number_bits);
                for i in 0..5 {
                    item.bit_value = m[i];
                    item.trit_value = encodings[i];
                    if *output_count < output.len() {
                        output[*output_count] = item;
                        *output_count += 1;
                    }
                }
                number_values_decoded += 5;
            }
            EIntegerEncoding::JustBits => {
                let mut item =
                    IntegerEncoded::new(EIntegerEncoding::JustBits, int_encoded.number_bits);
                item.bit_value = bit_stream.read_bits(int_encoded.number_bits as usize);
                if *output_count < output.len() {
                    output[*output_count] = item;
                    *output_count += 1;
                }
                number_values_decoded += 1;
            }
        }
    }
}
