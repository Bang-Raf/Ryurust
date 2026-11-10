use crate::differ::*;
use std::slice;

pub const RYU_BUFFER_DIFF_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_buffer_diff_version() -> u32 {
    RYU_BUFFER_DIFF_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn ryu_buffer_find_dirty_range(
    src: *const u8,
    dst: *const u8,
    len: usize,
    out_offset: *mut usize,
    out_length: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() {
        return -1;
    }

    if len == 0 {
        if !out_offset.is_null() {
            *out_offset = 0;
        }
        if !out_length.is_null() {
            *out_length = 0;
        }
        return 0;
    }

    let src_slice = slice::from_raw_parts(src, len);
    let dst_slice = slice::from_raw_parts(dst, len);

    match find_dirty_range(src_slice, dst_slice) {
        Some(range) => {
            if !out_offset.is_null() {
                *out_offset = range.offset;
            }
            if !out_length.is_null() {
                *out_length = range.length;
            }
            1
        }
        None => {
            if !out_offset.is_null() {
                *out_offset = 0;
            }
            if !out_length.is_null() {
                *out_length = 0;
            }
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_buffer_diff_and_copy(
    src: *const u8,
    dst: *mut u8,
    len: usize,
    out_offset: *mut usize,
    out_length: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() {
        return -1;
    }

    if len == 0 {
        if !out_offset.is_null() {
            *out_offset = 0;
        }
        if !out_length.is_null() {
            *out_length = 0;
        }
        return 0;
    }

    let src_slice = slice::from_raw_parts(src, len);
    let dst_slice = slice::from_raw_parts_mut(dst, len);

    match diff_and_copy(src_slice, dst_slice) {
        Some(range) => {
            if !out_offset.is_null() {
                *out_offset = range.offset;
            }
            if !out_length.is_null() {
                *out_length = range.length;
            }
            1
        }
        None => {
            if !out_offset.is_null() {
                *out_offset = 0;
            }
            if !out_length.is_null() {
                *out_length = 0;
            }
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_buffer_is_equal(a: *const u8, b: *const u8, len: usize) -> i32 {
    if a.is_null() || b.is_null() {
        return -1;
    }

    if len == 0 {
        return 1;
    }

    let a_slice = slice::from_raw_parts(a, len);
    let b_slice = slice::from_raw_parts(b, len);

    if is_equal_fast(a_slice, b_slice) {
        1
    } else {
        0
    }
}
