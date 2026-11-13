use crate::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcParseError {
    BufferTooSmall,
    InvalidHeader,
    CountOutOfRange,
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

pub fn parse_ipc_message(data: &[u8], cmd_ptr: i64) -> Result<ParsedIpcMessage, IpcParseError> {
    if data.len() < 8 {
        return Err(IpcParseError::BufferTooSmall);
    }

    let mut pos = 0usize;

    // Helper reader
    let read_u32 = |data: &[u8], pos: &mut usize| -> Result<u32, IpcParseError> {
        if *pos + 4 > data.len() {
            return Err(IpcParseError::BufferTooSmall);
        }
        let val = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
        *pos += 4;
        Ok(val)
    };

    let read_u64 = |data: &[u8], pos: &mut usize| -> Result<u64, IpcParseError> {
        if *pos + 8 > data.len() {
            return Err(IpcParseError::BufferTooSmall);
        }
        let val = u64::from_le_bytes([
            data[*pos],
            data[*pos + 1],
            data[*pos + 2],
            data[*pos + 3],
            data[*pos + 4],
            data[*pos + 5],
            data[*pos + 6],
            data[*pos + 7],
        ]);
        *pos += 8;
        Ok(val)
    };

    let read_i32 = |data: &[u8], pos: &mut usize| -> Result<i32, IpcParseError> {
        read_u32(data, pos).map(|v| v as i32)
    };

    let word0 = read_u32(data, &mut pos)?;
    let word1 = read_u32(data, &mut pos)?;

    let msg_type = word0 & 0xffff;
    let ptr_buff_count = ((word0 >> 16) & 0xf) as usize;
    let send_buff_count = ((word0 >> 20) & 0xf) as usize;
    let recv_buff_count = ((word0 >> 24) & 0xf) as usize;
    let xchg_buff_count = ((word0 >> 28) & 0xf) as usize;

    let mut raw_data_size = ((word1 & 0x3ff) * 4) as usize;
    let recv_list_flags = ((word1 >> 10) & 0xf) as i32;
    let hnd_desc_enable = ((word1 >> 31) & 0x1) != 0;

    let mut msg = ParsedIpcMessage {
        msg_type,
        handle_desc_enabled: if hnd_desc_enable { 1 } else { 0 },
        ..Default::default()
    };

    if hnd_desc_enable {
        let word = read_u32(data, &mut pos)?;
        let has_pid = (word & 1) != 0;
        msg.handle_desc.has_pid = if has_pid { 1 } else { 0 };
        if has_pid {
            msg.handle_desc.pid = read_u64(data, &mut pos)?;
        }

        let to_copy_size = ((word >> 1) & 0xf) as usize;
        msg.handle_desc.copy_count = to_copy_size as u32;
        for i in 0..to_copy_size {
            let handle = read_i32(data, &mut pos)?;
            if let Some(slot) = msg.handle_desc.copy_handles.get_mut(i) {
                *slot = handle;
            }
        }

        let to_move_size = ((word >> 5) & 0xf) as usize;
        msg.handle_desc.move_count = to_move_size as u32;
        for i in 0..to_move_size {
            let handle = read_i32(data, &mut pos)?;
            if let Some(slot) = msg.handle_desc.move_handles.get_mut(i) {
                *slot = handle;
            }
        }
    }

    // PtrBuffs
    msg.ptr_buff_count = ptr_buff_count as u32;
    for i in 0..ptr_buff_count {
        let w0 = read_u32(data, &mut pos)? as u64;
        let w1 = read_u32(data, &mut pos)? as u64;

        let mut position = w1;
        position |= (w0 << 20) & 0x0f_0000_0000;
        position |= (w0 << 30) & 0x70_0000_0000;

        let mut index = (w0 as u32) & 0x03f;
        index |= ((w0 as u32) >> 3) & 0x1c0;

        let size = (w0 >> 16) as u16 as u64;

        if let Some(slot) = msg.ptr_buffs.get_mut(i) {
            *slot = IpcPtrBufferDesc {
                position,
                size,
                index,
                _padding: 0,
            };
        }
    }

    // Helper for IpcBuffDesc
    let read_buff = |data: &[u8],
                     pos: &mut usize,
                     count: usize,
                     out: &mut [IpcBufferDesc; MAX_BUFFERS]|
     -> Result<(), IpcParseError> {
        for i in 0..count {
            let w0 = read_u32(data, pos)? as u64;
            let w1 = read_u32(data, pos)? as u64;
            let w2 = read_u32(data, pos)? as u64;

            let mut position = w1;
            position |= (w2 << 4) & 0x0f_0000_0000;
            position |= (w2 << 34) & 0x70_0000_0000;

            let mut size = w0;
            size |= (w2 << 8) & 0x0f_0000_0000;

            let flags = (w2 & 3) as u8;

            // Entries past MAX_BUFFERS are still consumed from `data` but dropped.
            if let Some(slot) = out.get_mut(i) {
                *slot = IpcBufferDesc {
                    position,
                    size,
                    flags,
                    _padding: [0; 7],
                };
            }
        }
        Ok(())
    };

    msg.send_buff_count = send_buff_count as u32;
    read_buff(data, &mut pos, send_buff_count, &mut msg.send_buffs)?;

    msg.recv_buff_count = recv_buff_count as u32;
    read_buff(data, &mut pos, recv_buff_count, &mut msg.recv_buffs)?;

    msg.xchg_buff_count = xchg_buff_count as u32;
    read_buff(data, &mut pos, xchg_buff_count, &mut msg.xchg_buffs)?;

    let recv_list_pos = pos + raw_data_size;

    // CMIF padding requirements: Only CMIF has the padding requirements (Type < TipcCloseSession = 0xF)
    if msg_type < IpcMessageType::TipcCloseSession as u32 && raw_data_size > 0 {
        let pad0 = get_pad_size_16((pos as i64) + cmd_ptr) as usize;
        raw_data_size = raw_data_size.saturating_sub(pad0);
        pos = (pos + pad0).min(recv_list_pos);
    }

    let mut recv_list_count = recv_list_flags - 2;
    if recv_list_count == 0 {
        recv_list_count = 1;
    } else if recv_list_count < 0 {
        recv_list_count = 0;
    }

    if pos + raw_data_size > data.len() {
        return Err(IpcParseError::BufferTooSmall);
    }

    msg.raw_data_offset = pos as u32;
    msg.raw_data_length = raw_data_size as u32;

    pos = recv_list_pos;

    msg.recv_list_count = recv_list_count as u32;
    for i in 0..recv_list_count as usize {
        let packed_value = read_u64(data, &mut pos)?;
        let position = packed_value & 0xffff_ffff_ffff;
        let size = (packed_value >> 48) as u16 as u64;

        if let Some(slot) = msg.recv_list_buffs.get_mut(i) {
            *slot = IpcRecvListBufferDesc { position, size };
        }
    }

    Ok(msg)
}
