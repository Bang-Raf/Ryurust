use crate::parser::parse_ipc_message;
use crate::serializer::{serialize_cmif, serialize_tipc};
use crate::types::*;
use std::slice;

pub const RYU_IPC_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_ipc_version() -> u32 {
    RYU_IPC_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn ryu_ipc_parse(
    data: *const u8,
    data_len: usize,
    cmd_ptr: i64,
    out: *mut ParsedIpcMessage,
) -> i32 {
    if data.is_null() || out.is_null() || data_len < 8 {
        return -1;
    }

    let input_slice = slice::from_raw_parts(data, data_len);
    match parse_ipc_message(input_slice, cmd_ptr) {
        Ok(parsed) => {
            *out = parsed;
            0
        }
        Err(_) => -2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_ipc_serialize_cmif(
    msg: *const ParsedIpcMessage,
    raw_data: *const u8,
    raw_len: usize,
    cmd_ptr: i64,
    recv_list_addr: u64,
    out_buf: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> i32 {
    if msg.is_null() || out_buf.is_null() || out_len.is_null() {
        return -1;
    }

    let raw_slice = if raw_data.is_null() || raw_len == 0 {
        &[]
    } else {
        slice::from_raw_parts(raw_data, raw_len)
    };

    let out_slice = slice::from_raw_parts_mut(out_buf, out_cap);

    match serialize_cmif(&*msg, raw_slice, cmd_ptr, recv_list_addr, out_slice) {
        Ok(written) => {
            *out_len = written;
            0
        }
        Err(_) => -2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_ipc_serialize_tipc(
    msg: *const ParsedIpcMessage,
    raw_data: *const u8,
    raw_len: usize,
    out_buf: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> i32 {
    if msg.is_null() || out_buf.is_null() || out_len.is_null() {
        return -1;
    }

    let raw_slice = if raw_data.is_null() || raw_len == 0 {
        &[]
    } else {
        slice::from_raw_parts(raw_data, raw_len)
    };

    let out_slice = slice::from_raw_parts_mut(out_buf, out_cap);

    match serialize_tipc(&*msg, raw_slice, out_slice) {
        Ok(written) => {
            *out_len = written;
            0
        }
        Err(_) => -2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_ipc_get_buffer_0x21(
    msg: *const ParsedIpcMessage,
    index: u32,
    out_pos: *mut u64,
    out_size: *mut u64,
) -> i32 {
    if msg.is_null() || out_pos.is_null() || out_size.is_null() {
        return -1;
    }

    let m = &*msg;
    let idx = index as usize;

    if (m.ptr_buff_count as usize) > idx && m.ptr_buffs[idx].position != 0 {
        *out_pos = m.ptr_buffs[idx].position;
        *out_size = m.ptr_buffs[idx].size;
        return 0;
    }

    if (m.send_buff_count as usize) > idx {
        *out_pos = m.send_buffs[idx].position;
        *out_size = m.send_buffs[idx].size;
        return 0;
    }

    *out_pos = 0;
    *out_size = 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_ipc_get_buffer_0x22(
    msg: *const ParsedIpcMessage,
    index: u32,
    out_pos: *mut u64,
    out_size: *mut u64,
) -> i32 {
    if msg.is_null() || out_pos.is_null() || out_size.is_null() {
        return -1;
    }

    let m = &*msg;
    let idx = index as usize;

    if (m.recv_list_count as usize) > idx && m.recv_list_buffs[idx].position != 0 {
        *out_pos = m.recv_list_buffs[idx].position;
        *out_size = m.recv_list_buffs[idx].size;
        return 0;
    }

    if (m.recv_buff_count as usize) > idx {
        *out_pos = m.recv_buffs[idx].position;
        *out_size = m.recv_buffs[idx].size;
        return 0;
    }

    *out_pos = 0;
    *out_size = 0;
    0
}
