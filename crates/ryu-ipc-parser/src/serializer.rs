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

/// Largest count representable in the 4-bit header fields.
const MAX_HEADER_COUNT: u32 = 0xf;

#[inline(always)]
fn write_u32(val: u32, out: &mut [u8], pos: &mut usize) {
    out[*pos..*pos + 4].copy_from_slice(&val.to_le_bytes());
    *pos += 4;
}

#[inline(always)]
fn write_u64(val: u64, out: &mut [u8], pos: &mut usize) {
    out[*pos..*pos + 8].copy_from_slice(&val.to_le_bytes());
    *pos += 8;
}

/// Rejects counts that do not fit the 4-bit header fields (they would be
/// truncated in the header while the full count is written to the stream).
fn validate_counts(msg: &ParsedIpcMessage, include_ptr: bool) -> Result<(), IpcSerializeError> {
    let ptr = if include_ptr { msg.ptr_buff_count } else { 0 };
    let (copy, mov) = if msg.handle_desc_enabled != 0 {
        (msg.handle_desc.copy_count, msg.handle_desc.move_count)
    } else {
        (0, 0)
    };
    let counts = [
        msg.send_buff_count,
        msg.recv_buff_count,
        msg.xchg_buff_count,
        ptr,
        copy,
        mov,
    ];
    if counts.iter().any(|&c| c > MAX_HEADER_COUNT) {
        return Err(IpcSerializeError::InvalidData);
    }
    Ok(())
}

fn handle_stream_len(msg: &ParsedIpcMessage) -> usize {
    if msg.handle_desc_enabled == 0 {
        return 0;
    }
    let mut len = 4;
    if msg.handle_desc.has_pid != 0 {
        len += 8;
    }
    len += (msg.handle_desc.copy_count as usize) * 4;
    len += (msg.handle_desc.move_count as usize) * 4;
    len
}

fn write_handle_desc(msg: &ParsedIpcMessage, out: &mut [u8], pos: &mut usize) {
    if msg.handle_desc_enabled == 0 {
        return;
    }
    let mut h_word = 0u32;
    if msg.handle_desc.has_pid != 0 {
        h_word |= 1;
    }
    h_word |= (msg.handle_desc.copy_count & 0xf) << 1;
    h_word |= (msg.handle_desc.move_count & 0xf) << 5;
    write_u32(h_word, out, pos);

    if msg.handle_desc.has_pid != 0 {
        write_u64(msg.handle_desc.pid, out, pos);
    }

    for i in 0..msg.handle_desc.copy_count as usize {
        write_u32(msg.handle_desc.copy_handles[i] as u32, out, pos);
    }

    for i in 0..msg.handle_desc.move_count as usize {
        write_u32(msg.handle_desc.move_handles[i] as u32, out, pos);
    }
}

/// Size in bytes of one send/receive/exchange (A/B/W) descriptor.
const BUFF_DESC_SIZE: usize = 12;

#[inline(always)]
fn buff_desc_bytes(msg: &ParsedIpcMessage) -> usize {
    (msg.send_buff_count + msg.recv_buff_count + msg.xchg_buff_count) as usize * BUFF_DESC_SIZE
}

/// Encodes A/B/W descriptors in the order the parser (and HIPC) expects:
/// send, receive, exchange. Inverse of `IpcBuffDesc(BinaryReader)` in C#:
/// word0 = size[31:0], word1 = position[31:0],
/// word2 = flags[1:0] | position[38:36] << 2 | size[35:32] << 24 | position[35:32] << 28.
fn write_buff_descs(msg: &ParsedIpcMessage, out: &mut [u8], pos: &mut usize) {
    let lists = [
        (msg.send_buff_count, &msg.send_buffs),
        (msg.recv_buff_count, &msg.recv_buffs),
        (msg.xchg_buff_count, &msg.xchg_buffs),
    ];
    for (count, descs) in lists {
        for desc in &descs[..count as usize] {
            let w0 = desc.size as u32;
            let w1 = desc.position as u32;
            let w2 = (desc.flags as u32 & 3)
                | (((desc.position >> 34) & 0x1c) as u32)
                | (((desc.size >> 8) & 0x0f00_0000) as u32)
                | (((desc.position >> 4) & 0xf000_0000) as u32);
            write_u32(w0, out, pos);
            write_u32(w1, out, pos);
            write_u32(w2, out, pos);
        }
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

    validate_counts(msg, true)?;
    let handle_stream_len = handle_stream_len(msg);
    let buff_desc_len = buff_desc_bytes(msg);

    let raw_len_orig = raw_data.len();
    let data_len_aligned = (raw_len_orig + 3) & !3;
    let raw_length = data_len_aligned;

    let offset_for_pad0 = cmd_ptr
        + 8
        + (handle_stream_len as i64)
        + ((msg.ptr_buff_count as i64) * 8)
        + (buff_desc_len as i64);
    let pad0 = get_pad_size_16(offset_for_pad0) as usize;
    let pad1 = 0x10 - pad0;

    let total_data_words = (data_len_aligned + pad0 + pad1) / 4;
    let mut word1: u32 = ((total_data_words as u32) & 0x3ff) | (2 << 10);
    if msg.handle_desc_enabled != 0 {
        word1 |= 1 << 31;
    }

    let total_len = 8
        + handle_stream_len
        + (msg.ptr_buff_count as usize) * 8
        + buff_desc_len
        + pad0
        + raw_length
        + pad1
        + 8; // recvListAddr (u64)

    if out.len() < total_len {
        return Err(IpcSerializeError::OutputBufferTooSmall);
    }

    let mut pos = 0usize;

    write_u32(word0, out, &mut pos);
    write_u32(word1, out, &mut pos);

    // Handle descriptor
    write_handle_desc(msg, out, &mut pos);

    // Ptr buffs
    for i in 0..msg.ptr_buff_count as usize {
        let desc = &msg.ptr_buffs[i];
        let mut w0: u32 = (((desc.position & 0x0f_0000_0000) >> 20) as u32)
            | (((desc.position & 0x70_0000_0000) >> 30) as u32);
        w0 |= desc.index & 0x03f;
        w0 |= (desc.index & 0x1c0) << 3;
        w0 |= (desc.size as u32) << 16;

        let w1: u32 = desc.position as u32;

        write_u32(w0, out, &mut pos);
        write_u32(w1, out, &mut pos);
    }

    // Send / receive / exchange buffer descriptors
    write_buff_descs(msg, out, &mut pos);

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

    // TIPC has no pointer (X) descriptors. C# `GetStreamTipc` asserts
    // `PtrBuff.Count == 0`; reject such a message instead of dropping them.
    if msg.ptr_buff_count != 0 {
        return Err(IpcSerializeError::InvalidData);
    }
    validate_counts(msg, false)?;
    let handle_stream_len = handle_stream_len(msg);
    let buff_desc_len = buff_desc_bytes(msg);

    let raw_len_orig = raw_data.len();
    let raw_len_aligned = (raw_len_orig + 3) & !3;
    let data_len_words = raw_len_aligned / 4;

    let mut word1: u32 = (data_len_words as u32) & 0x3ff;
    if msg.handle_desc_enabled != 0 {
        word1 |= 1 << 31;
    }

    // Raw data is zero-padded to the word count advertised in the header.
    let total_len = 8 + handle_stream_len + buff_desc_len + raw_len_aligned;
    if out.len() < total_len {
        return Err(IpcSerializeError::OutputBufferTooSmall);
    }

    let mut pos = 0usize;

    write_u32(word0, out, &mut pos);
    write_u32(word1, out, &mut pos);

    write_handle_desc(msg, out, &mut pos);

    write_buff_descs(msg, out, &mut pos);

    out[pos..pos + raw_len_orig].copy_from_slice(raw_data);
    pos += raw_len_orig;
    let pad_raw = raw_len_aligned - raw_len_orig;
    out[pos..pos + pad_raw].fill(0);
    pos += pad_raw;

    Ok(pos)
}
