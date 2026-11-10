use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes128;

#[inline]
pub fn add_counter_offset(counter: [u8; 16], offset_blocks: u64) -> [u8; 16] {
    let mut high = u64::from_be_bytes(counter[0..8].try_into().unwrap());
    let low = u64::from_be_bytes(counter[8..16].try_into().unwrap());
    let (new_low, carry) = low.overflowing_add(offset_blocks);
    if carry {
        high = high.wrapping_add(1);
    }
    let mut res = [0u8; 16];
    res[0..8].copy_from_slice(&high.to_be_bytes());
    res[8..16].copy_from_slice(&new_low.to_be_bytes());
    res
}

#[inline]
pub fn increment_counter(counter: &mut [u8; 16]) {
    for byte in counter.iter_mut().rev() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

pub fn decrypt_nca_ctr(
    src: &[u8],
    dst: &mut [u8],
    key: &[u8; 16],
    base_counter: &[u8; 16],
    offset_bytes: u64,
) -> Result<(), &'static str> {
    if src.len() != dst.len() {
        return Err("src and dst length mismatch");
    }
    if src.is_empty() {
        return Ok(());
    }

    let cipher = Aes128::new(GenericArray::from_slice(key));
    let block_index = offset_bytes / 16;
    let intra_offset = (offset_bytes % 16) as usize;

    let mut current_counter = add_counter_offset(*base_counter, block_index);
    let mut keystream = GenericArray::clone_from_slice(&current_counter);
    cipher.encrypt_block(&mut keystream);

    let mut src_pos = 0;
    let mut dst_pos = 0;
    let total_len = src.len();

    // Handle intra-block prefix if offset is not 16-byte aligned
    if intra_offset > 0 {
        let prefix_len = (16 - intra_offset).min(total_len);
        for i in 0..prefix_len {
            dst[dst_pos + i] = src[src_pos + i] ^ keystream[intra_offset + i];
        }
        src_pos += prefix_len;
        dst_pos += prefix_len;
        increment_counter(&mut current_counter);
    }

    // Process 4 blocks (64 bytes) at once using hardware AES pipelining
    while src_pos + 64 <= total_len {
        let c0 = current_counter;
        increment_counter(&mut current_counter);
        let c1 = current_counter;
        increment_counter(&mut current_counter);
        let c2 = current_counter;
        increment_counter(&mut current_counter);
        let c3 = current_counter;
        increment_counter(&mut current_counter);

        let mut blocks = [
            GenericArray::clone_from_slice(&c0),
            GenericArray::clone_from_slice(&c1),
            GenericArray::clone_from_slice(&c2),
            GenericArray::clone_from_slice(&c3),
        ];

        cipher.encrypt_blocks(&mut blocks);

        for (b_idx, block) in blocks.iter().enumerate() {
            let offset = b_idx * 16;
            let src_chunk = &src[src_pos + offset..src_pos + offset + 16];
            let dst_chunk = &mut dst[dst_pos + offset..dst_pos + offset + 16];

            let s0 = u64::from_ne_bytes(src_chunk[0..8].try_into().unwrap());
            let s1 = u64::from_ne_bytes(src_chunk[8..16].try_into().unwrap());
            let k0 = u64::from_ne_bytes(block[0..8].try_into().unwrap());
            let k1 = u64::from_ne_bytes(block[8..16].try_into().unwrap());

            dst_chunk[0..8].copy_from_slice(&(s0 ^ k0).to_ne_bytes());
            dst_chunk[8..16].copy_from_slice(&(s1 ^ k1).to_ne_bytes());
        }

        src_pos += 64;
        dst_pos += 64;
    }

    // Process remaining full 16-byte blocks
    while src_pos + 16 <= total_len {
        keystream.copy_from_slice(&current_counter);
        cipher.encrypt_block(&mut keystream);

        let src_chunk = &src[src_pos..src_pos + 16];
        let dst_chunk = &mut dst[dst_pos..dst_pos + 16];

        let s0 = u64::from_ne_bytes(src_chunk[0..8].try_into().unwrap());
        let s1 = u64::from_ne_bytes(src_chunk[8..16].try_into().unwrap());
        let k0 = u64::from_ne_bytes(keystream[0..8].try_into().unwrap());
        let k1 = u64::from_ne_bytes(keystream[8..16].try_into().unwrap());

        dst_chunk[0..8].copy_from_slice(&(s0 ^ k0).to_ne_bytes());
        dst_chunk[8..16].copy_from_slice(&(s1 ^ k1).to_ne_bytes());

        src_pos += 16;
        dst_pos += 16;
        increment_counter(&mut current_counter);
    }

    // Process tail bytes if any
    if src_pos < total_len {
        keystream.copy_from_slice(&current_counter);
        cipher.encrypt_block(&mut keystream);

        let tail_len = total_len - src_pos;
        for i in 0..tail_len {
            dst[dst_pos + i] = src[src_pos + i] ^ keystream[i];
        }
    }

    Ok(())
}
