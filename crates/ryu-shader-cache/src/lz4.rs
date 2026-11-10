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

        // Copy literals. Running out of input is corruption; running out of output
        // space is a too-small destination.
        if lit_len > in_len - in_pos {
            return Err(Lz4Error::CorruptedInput);
        }
        if lit_len > out_cap - out_pos {
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

        if match_len > out_cap - out_pos {
            return Err(Lz4Error::OutputBufferTooSmall);
        }

        copy_match(output, out_pos, offset, match_len);
        out_pos += match_len;
    }

    Ok(out_pos)
}

/// Copies an LZ4 match of `len` bytes starting `offset` bytes back from `out_pos`.
///
/// Equivalent to the byte-by-byte `output[out_pos + i] = output[out_pos - offset + i]`
/// even when the regions overlap (`offset < len`): the match then repeats the last
/// `offset` bytes, so it is copied in `offset`-sized chunks whose source is always
/// fully written before it is read.
#[inline(always)]
fn copy_match(output: &mut [u8], out_pos: usize, offset: usize, len: usize) {
    let src = out_pos - offset;
    if offset >= len {
        output.copy_within(src..src + len, out_pos);
    } else if offset == 1 {
        let b = output[src];
        output[out_pos..out_pos + len].fill(b);
    } else {
        let mut copied = 0;
        while copied < len {
            let n = offset.min(len - copied);
            output.copy_within(src + copied..src + copied + n, out_pos + copied);
            copied += n;
        }
    }
}
