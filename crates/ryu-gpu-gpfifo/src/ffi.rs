use crate::parser::{parse_pushbuffer_chunk, GpfifoParsedMethod, GpfifoState};
use std::slice;

pub const RYU_GPFIFO_OK: i32 = 0;
pub const RYU_GPFIFO_ERR_NULL_PTR: i32 = -1;

#[no_mangle]
pub unsafe extern "C" fn ryu_gpfifo_parse_chunk(
    cmd_buffer_ptr: *const i32,
    cmd_buffer_len: usize,
    state_ptr: *mut GpfifoState,
    out_methods_ptr: *mut GpfifoParsedMethod,
    out_methods_capacity: usize,
    out_methods_count: *mut usize,
) -> i32 {
    if cmd_buffer_ptr.is_null() || state_ptr.is_null() || out_methods_count.is_null() {
        return RYU_GPFIFO_ERR_NULL_PTR;
    }
    if cmd_buffer_len == 0 {
        *out_methods_count = 0;
        return RYU_GPFIFO_OK;
    }

    let cmd_buffer = slice::from_raw_parts(cmd_buffer_ptr, cmd_buffer_len);
    let state = &mut *state_ptr;
    let out_methods = if !out_methods_ptr.is_null() && out_methods_capacity > 0 {
        slice::from_raw_parts_mut(out_methods_ptr, out_methods_capacity)
    } else {
        &mut []
    };

    let parsed = parse_pushbuffer_chunk(cmd_buffer, state, out_methods);
    *out_methods_count = parsed;

    RYU_GPFIFO_OK
}
