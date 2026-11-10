#[derive(Debug, Clone, Copy)]
pub struct BitStream128 {
    data: u128,
    bits_left: usize,
}

impl BitStream128 {
    #[inline(always)]
    pub fn new(bytes: &[u8; 16]) -> Self {
        Self {
            data: u128::from_le_bytes(*bytes),
            bits_left: 128,
        }
    }

    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            data: 0,
            bits_left: 128,
        }
    }

    #[inline(always)]
    pub fn bits_left(&self) -> usize {
        self.bits_left
    }

    #[inline(always)]
    pub fn to_le_bytes(&self) -> [u8; 16] {
        self.data.to_le_bytes()
    }

    #[inline(always)]
    pub fn read_bits(&mut self, bit_count: usize) -> u32 {
        if bit_count == 0 {
            return 0;
        }
        debug_assert!(bit_count <= 32);
        debug_assert!(bit_count <= self.bits_left);

        let mask = if bit_count == 32 {
            0xFFFF_FFFFu128
        } else {
            (1u128 << bit_count) - 1
        };

        let value = (self.data & mask) as u32;
        self.data >>= bit_count;
        self.bits_left -= bit_count;
        value
    }

    #[inline(always)]
    pub fn write_bits(&mut self, value: u32, bit_count: usize) {
        if bit_count == 0 {
            return;
        }
        debug_assert!(bit_count <= 32);
        debug_assert!(bit_count <= self.bits_left);

        let shift = 128 - self.bits_left;
        let mask = if bit_count == 32 {
            0xFFFF_FFFFu128
        } else {
            (1u128 << bit_count) - 1
        };

        self.data |= ((value as u128) & mask) << shift;
        self.bits_left -= bit_count;
    }
}
