use crate::parser::parse_ipc_message;
use crate::serializer::{serialize_cmif, serialize_tipc};
use crate::types::*;
use std::slice;

pub const RYU_IPC_VERSION: u32 = 0x0001_0000;

/// Kept for compatibility with existing bindings; same value as `ryu_ipc_parser_version`.
#[no_mangle]
pub extern "C" fn ryu_ipc_version() -> u32 {
    RYU_IPC_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_ipc_parser_version() -> u32 {
    RYU_IPC_VERSION
}

/// # Safety
/// `data` must be valid for reads of `data_len` bytes and `out` for writing one
/// `ParsedIpcMessage`. Either being null returns -1.
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

/// # Safety
/// `msg` must point to a valid `ParsedIpcMessage`, `raw_data` must be null or valid for
/// reads of `raw_len` bytes, `out_buf` valid for writes of `out_cap` bytes (not
/// overlapping the inputs) and `out_len` for writing one `usize`.
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

/// # Safety
/// `msg` must point to a valid `ParsedIpcMessage`, `raw_data` must be null or valid for
/// reads of `raw_len` bytes, `out_buf` valid for writes of `out_cap` bytes (not
/// overlapping the inputs) and `out_len` for writing one `usize`.
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

/// # Safety
/// `msg` must point to a valid `ParsedIpcMessage`; `out_pos` and `out_size` must each be
/// valid for writing one `u64`. Any of them being null returns -1.
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

/// # Safety
/// `msg` must point to a valid `ParsedIpcMessage`; `out_pos` and `out_size` must each be
/// valid for writing one `u64`. Any of them being null returns -1.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_ipc_parser_version(), RYU_IPC_VERSION);
        assert_eq!(ryu_ipc_version(), RYU_IPC_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let data = [0u8; 16];
        let mut msg = ParsedIpcMessage::default();
        let mut out = [0u8; 64];
        let mut out_len = 0usize;
        let (mut pos, mut size) = (0u64, 0u64);
        unsafe {
            assert_eq!(ryu_ipc_parse(ptr::null(), 16, 0, &mut msg), -1);
            assert_eq!(ryu_ipc_parse(data.as_ptr(), 16, 0, ptr::null_mut()), -1);

            assert_eq!(
                ryu_ipc_serialize_cmif(
                    ptr::null(),
                    data.as_ptr(),
                    16,
                    0,
                    0,
                    out.as_mut_ptr(),
                    out.len(),
                    &mut out_len
                ),
                -1
            );
            assert_eq!(
                ryu_ipc_serialize_cmif(
                    &msg,
                    data.as_ptr(),
                    16,
                    0,
                    0,
                    ptr::null_mut(),
                    out.len(),
                    &mut out_len
                ),
                -1
            );
            assert_eq!(
                ryu_ipc_serialize_cmif(
                    &msg,
                    data.as_ptr(),
                    16,
                    0,
                    0,
                    out.as_mut_ptr(),
                    out.len(),
                    ptr::null_mut()
                ),
                -1
            );

            assert_eq!(
                ryu_ipc_serialize_tipc(
                    ptr::null(),
                    data.as_ptr(),
                    16,
                    out.as_mut_ptr(),
                    out.len(),
                    &mut out_len
                ),
                -1
            );
            assert_eq!(
                ryu_ipc_serialize_tipc(
                    &msg,
                    data.as_ptr(),
                    16,
                    ptr::null_mut(),
                    out.len(),
                    &mut out_len
                ),
                -1
            );
            assert_eq!(
                ryu_ipc_serialize_tipc(
                    &msg,
                    data.as_ptr(),
                    16,
                    out.as_mut_ptr(),
                    out.len(),
                    ptr::null_mut()
                ),
                -1
            );

            assert_eq!(
                ryu_ipc_get_buffer_0x21(ptr::null(), 0, &mut pos, &mut size),
                -1
            );
            assert_eq!(
                ryu_ipc_get_buffer_0x21(&msg, 0, ptr::null_mut(), &mut size),
                -1
            );
            assert_eq!(
                ryu_ipc_get_buffer_0x21(&msg, 0, &mut pos, ptr::null_mut()),
                -1
            );
            assert_eq!(
                ryu_ipc_get_buffer_0x22(ptr::null(), 0, &mut pos, &mut size),
                -1
            );
            assert_eq!(
                ryu_ipc_get_buffer_0x22(&msg, 0, ptr::null_mut(), &mut size),
                -1
            );
            assert_eq!(
                ryu_ipc_get_buffer_0x22(&msg, 0, &mut pos, ptr::null_mut()),
                -1
            );
        }
    }
}
