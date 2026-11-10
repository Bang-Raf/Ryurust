use crate::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcSerializeError {
    OutputBufferTooSmall,
    InvalidData,
}

#[inline(always)]
fn get_pad_size_16(position: i64) -> i64 {
    let rem = position & 0xf;
    if rem != 0 {
        0x10 - rem
    } else {
        0
    }
}

pub fn serialize_cmif(
    msg: &ParsedIpcMessage,
    raw_data: &[u8],
    cmd_ptr: i64,
    recv_list_addr: u64,
    out: &mut [u8],
) -> Result<usize, IpcSerializeError> {
    let mut word0: u32 = msg.msg_type & 0xffff;
    word0 |= (msg.ptr_buff_count & 0xf) << 16;
    word0 |= (msg.send_buff_count & 0xf) << 20;
    word0 |= (msg.recv_buff_count & 0xf) << 24;
    word0 |= (msg.xchg_buff_count & 0xf) << 28;

    let mut handle_stream_len = 0usize;
    if msg.handle_desc_enabled != 0 {
        handle_stream_len = 4;
        if msg.handle_desc.has_pid != 0 {
            handle_stream_len += 8;
        }
        handle_stream_len += (msg.handle_desc.copy_count as usize) * 4;
        handle_stream_len += (msg.handle_desc.move_count as usize) * 4;
    }

    let raw_len_orig = raw_data.len();
    let data_len_aligned = (raw_len_orig + 3) & !3;
    let raw_length = data_len_aligned;

    let offset_for_pad0 =
        cmd_ptr + 8 + (handle_stream_len as i64) + ((msg.ptr_buff_count as i64) * 8);
    let pad0 = get_pad_size_16(offset_for_pad0) as usize;
    let pad1 = 0x10 - pad0;

    let total_data_words = (data_len_aligned + pad0 + pad1) / 4;
    let mut word1: u32 = ((total_data_words as u32) & 0x3ff) | (2 << 10);
    if msg.handle_desc_enabled != 0 {
        word1 |= 1 << 31;
    }

    let total_len =
        8 + handle_stream_len + (msg.ptr_buff_count as usize) * 8 + pad0 + raw_length + pad1 + 8; // recvListAddr (u64)

    if out.len() < total_len {
        return Err(IpcSerializeError::OutputBufferTooSmall);
    }

    let mut pos = 0usize;

    // Helper writer
    let write_u32 = |val: u32, out: &mut [u8], pos: &mut usize| {
        let bytes = val.to_le_bytes();
        out[*pos..*pos + 4].copy_from_slice(&bytes);
        *pos += 4;
    };

    let write_u64 = |val: u64, out: &mut [u8], pos: &mut usize| {
        let bytes = val.to_le_bytes();
        out[*pos..*pos + 8].copy_from_slice(&bytes);
        *pos += 8;
    };

    write_u32(word0, out, &mut pos);
    write_u32(word1, out, &mut pos);

    // Handle descriptor
    if msg.handle_desc_enabled != 0 {
        let mut h_word = 0u32;
        if msg.handle_desc.has_pid != 0 {
            h_word |= 1;
        }
        h_word |= (msg.handle_desc.copy_count & 0xf) << 1;
        h_word |= (msg.handle_desc.move_count & 0xf) << 5;
        write_u32(h_word, out, &mut pos);

        if msg.handle_desc.has_pid != 0 {
            write_u64(msg.handle_desc.pid, out, &mut pos);
        }

        for i in 0..msg.handle_desc.copy_count as usize {
            write_u32(msg.handle_desc.copy_handles[i] as u32, out, &mut pos);
        }

        for i in 0..msg.handle_desc.move_count as usize {
            write_u32(msg.handle_desc.move_handles[i] as u32, out, &mut pos);
        }
    }

    // Ptr buffs
    for i in 0..msg.ptr_buff_count as usize {
        let desc = &msg.ptr_buffs[i];
        let mut w0: u32 = (((desc.position & 0x0f_0000_0000) >> 20) as u32)
            | (((desc.position & 0x70_0000_0000) >> 30) as u32);
        w0 |= (desc.index & 0x03f) << 0;
        w0 |= (desc.index & 0x1c0) << 3;
        w0 |= (desc.size as u32) << 16;

        let w1: u32 = desc.position as u32;

        write_u32(w0, out, &mut pos);
        write_u32(w1, out, &mut pos);
    }

    // pad0
    out[pos..pos + pad0].fill(0);
    pos += pad0;

    // raw data
    out[pos..pos + raw_len_orig].copy_from_slice(raw_data);
    pos += raw_len_orig;
    let pad_raw = raw_length - raw_len_orig;
    if pad_raw > 0 {
        out[pos..pos + pad_raw].fill(0);
        pos += pad_raw;
    }

    // pad1
    out[pos..pos + pad1].fill(0);
    pos += pad1;

    // recvListAddr
    write_u64(recv_list_addr, out, &mut pos);

    Ok(pos)
}

pub fn serialize_tipc(
    msg: &ParsedIpcMessage,
    raw_data: &[u8],
    out: &mut [u8],
) -> Result<usize, IpcSerializeError> {
    let mut word0: u32 = msg.msg_type & 0xffff;
    word0 |= (msg.send_buff_count & 0xf) << 20;
    word0 |= (msg.recv_buff_count & 0xf) << 24;
    word0 |= (msg.xchg_buff_count & 0xf) << 28;

    let mut handle_stream_len = 0usize;
    if msg.handle_desc_enabled != 0 {
        handle_stream_len = 4;
        if msg.handle_desc.has_pid != 0 {
            handle_stream_len += 8;
        }
        handle_stream_len += (msg.handle_desc.copy_count as usize) * 4;
        handle_stream_len += (msg.handle_desc.move_count as usize) * 4;
    }

    let raw_len_orig = raw_data.len();
    let data_len_words = ((raw_len_orig + 3) & !3) / 4;

    let mut word1: u32 = (data_len_words as u32) & 0x3ff;
    if msg.handle_desc_enabled != 0 {
        word1 |= 1 << 31;
    }

    let total_len = 8 + handle_stream_len + raw_len_orig;
    if out.len() < total_len {
        return Err(IpcSerializeError::OutputBufferTooSmall);
    }

    let mut pos = 0usize;

    let write_u32 = |val: u32, out: &mut [u8], pos: &mut usize| {
        let bytes = val.to_le_bytes();
        out[*pos..*pos + 4].copy_from_slice(&bytes);
        *pos += 4;
    };

    let write_u64 = |val: u64, out: &mut [u8], pos: &mut usize| {
        let bytes = val.to_le_bytes();
        out[*pos..*pos + 8].copy_from_slice(&bytes);
        *pos += 8;
    };

    write_u32(word0, out, &mut pos);
    write_u32(word1, out, &mut pos);

    if msg.handle_desc_enabled != 0 {
        let mut h_word = 0u32;
        if msg.handle_desc.has_pid != 0 {
            h_word |= 1;
        }
        h_word |= (msg.handle_desc.copy_count & 0xf) << 1;
        h_word |= (msg.handle_desc.move_count & 0xf) << 5;
        write_u32(h_word, out, &mut pos);

        if msg.handle_desc.has_pid != 0 {
            write_u64(msg.handle_desc.pid, out, &mut pos);
        }

        for i in 0..msg.handle_desc.copy_count as usize {
            write_u32(msg.handle_desc.copy_handles[i] as u32, out, &mut pos);
        }

        for i in 0..msg.handle_desc.move_count as usize {
            write_u32(msg.handle_desc.move_handles[i] as u32, out, &mut pos);
        }
    }

    out[pos..pos + raw_len_orig].copy_from_slice(raw_data);
    pos += raw_len_orig;

    Ok(pos)
}
