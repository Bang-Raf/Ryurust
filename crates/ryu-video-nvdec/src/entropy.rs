// VP9 Boolean Arithmetic Decoder (Entropy Reader / CABAC)
// Spec compliant VP9 bool decoder for parsing bitstream syntax elements

pub struct BoolDecoder<'a> {
    buffer: &'a [u8],
    pos: usize,
    value: usize,
    range: usize,
    count: i32,
}

impl<'a> BoolDecoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        let mut decoder = Self {
            buffer,
            pos: 0,
            value: 0,
            range: 255,
            count: -8,
        };

        // Initialize value with first 2 bytes if available
        let mut val = 0usize;
        if decoder.buffer.len() > 0 {
            val = (decoder.buffer[0] as usize) << 8;
            decoder.pos = 1;
        }
        if decoder.buffer.len() > 1 {
            val |= decoder.buffer[1] as usize;
            decoder.pos = 2;
        }
        decoder.value = val;
        decoder
    }

    #[inline]
    pub fn read_bit(&mut self, prob: u8) -> u8 {
        let split = 1 + (((self.range - 1) * (prob as usize)) >> 8);
        let big_split = split << 8;

        let bit = if self.value >= big_split {
            self.range -= split;
            self.value -= big_split;
            1
        } else {
            self.range = split;
            0
        };

        // Renormalize
        while self.range < 128 {
            self.range <<= 1;
            self.value <<= 1;

            self.count += 1;
            if self.count == 0 {
                let next_byte = if self.pos < self.buffer.len() {
                    let b = self.buffer[self.pos] as usize;
                    self.pos += 1;
                    b
                } else {
                    0
                };
                self.value |= next_byte;
                self.count = -8;
            }
        }

        bit
    }

    #[inline]
    pub fn read_bool(&mut self) -> bool {
        self.read_bit(128) != 0
    }

    pub fn read_literal(&mut self, bits: u32) -> u32 {
        let mut literal = 0u32;
        for _ in 0..bits {
            literal = (literal << 1) | (self.read_bit(128) as u32);
        }
        literal
    }

    pub fn read_tree(&mut self, tree: &[i8], probs: &[u8]) -> i32 {
        let mut i = 0i32;
        while i >= 0 {
            let prob = probs[(i >> 1) as usize];
            let bit = self.read_bit(prob);
            i = tree[(i + bit as i32) as usize] as i32;
        }
        -i - 1
    }

    pub fn bytes_consumed(&self) -> usize {
        self.pos
    }

    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.buffer.len()
    }
}
