#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lz4Error {
    CorruptedInput,
    OutputBufferTooSmall,
    InvalidOffset,
}

pub fn decompress_lz4_block(input: &[u8], output: &mut [u8]) -> Result<usize, Lz4Error> {
    let mut in_pos = 0;
    let mut out_pos = 0;
    let in_len = input.len();
    let out_cap = output.len();

    while in_pos < in_len {
        let token = input[in_pos];
        in_pos += 1;

        // 1. Literal length
        let mut lit_len = (token >> 4) as usize;
        if lit_len == 15 {
            loop {
                if in_pos >= in_len {
                    return Err(Lz4Error::CorruptedInput);
                }
                let s = input[in_pos] as usize;
                in_pos += 1;
                lit_len += s;
                if s != 255 {
                    break;
                }
            }
        }

        // Copy literals
        if in_pos + lit_len > in_len || out_pos + lit_len > out_cap {
            return Err(Lz4Error::OutputBufferTooSmall);
        }

        output[out_pos..out_pos + lit_len].copy_from_slice(&input[in_pos..in_pos + lit_len]);
        in_pos += lit_len;
        out_pos += lit_len;

        if in_pos >= in_len {
            break;
        }

        // 2. Match offset (2 bytes LE)
        if in_pos + 2 > in_len {
            return Err(Lz4Error::CorruptedInput);
        }
        let offset = input[in_pos] as usize | ((input[in_pos + 1] as usize) << 8);
        in_pos += 2;

        if offset == 0 || offset > out_pos {
            return Err(Lz4Error::InvalidOffset);
        }

        // 3. Match length
        let mut match_len = (token & 0x0F) as usize;
        if match_len == 15 {
            loop {
                if in_pos >= in_len {
                    return Err(Lz4Error::CorruptedInput);
                }
                let s = input[in_pos] as usize;
                in_pos += 1;
                match_len += s;
                if s != 255 {
                    break;
                }
            }
        }
        match_len += 4; // Min match length is 4

        if out_pos + match_len > out_cap {
            return Err(Lz4Error::OutputBufferTooSmall);
        }

        // Copy matching bytes (may overlap)
        let match_src = out_pos - offset;
        for i in 0..match_len {
            output[out_pos + i] = output[match_src + i];
        }
        out_pos += match_len;
    }

    Ok(out_pos)
}
