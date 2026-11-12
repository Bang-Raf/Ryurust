use crate::parser::{parse_pushbuffer_chunk, GpfifoParsedMethod, GpfifoState};
use std::slice;

pub const RYU_GPFIFO_OK: i32 = 0;
pub const RYU_GPFIFO_ERR_NULL_PTR: i32 = -1;

pub const RYU_GPU_GPFIFO_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_gpu_gpfifo_version() -> u32 {
    RYU_GPU_GPFIFO_VERSION
}

/// # Safety
/// `cmd_buffer_ptr` must be readable for `cmd_buffer_len` words. `state_ptr` must
/// point to a valid `GpfifoState` (bool fields 0 or 1) not accessed elsewhere during
/// the call. `out_methods_ptr` may be null, otherwise it must be writable for
/// `out_methods_capacity` entries. `out_methods_count` must be writable.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_gpu_gpfifo_version(), RYU_GPU_GPFIFO_VERSION);
        assert_eq!(RYU_GPU_GPFIFO_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let cmd = [0i32; 2];
        let mut state = GpfifoState::default();
        let mut count = 7usize;
        // SAFETY: every non-null pointer refers to a live local of the stated length.
        unsafe {
            assert_eq!(
                ryu_gpfifo_parse_chunk(
                    ptr::null(),
                    2,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    4,
                    ptr::null_mut()
                ),
                RYU_GPFIFO_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_gpfifo_parse_chunk(ptr::null(), 2, &mut state, ptr::null_mut(), 0, &mut count),
                RYU_GPFIFO_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_gpfifo_parse_chunk(
                    cmd.as_ptr(),
                    2,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                    &mut count
                ),
                RYU_GPFIFO_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_gpfifo_parse_chunk(
                    cmd.as_ptr(),
                    2,
                    &mut state,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut()
                ),
                RYU_GPFIFO_ERR_NULL_PTR
            );
            assert_eq!(count, 7);
            // A null output array is optional, even with a non-zero capacity.
            assert_eq!(
                ryu_gpfifo_parse_chunk(cmd.as_ptr(), 2, &mut state, ptr::null_mut(), 4, &mut count),
                RYU_GPFIFO_OK
            );
        }
    }
}
