use crate::differ::*;
use std::slice;

pub const RYU_BUFFER_DIFF_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_buffer_diff_version() -> u32 {
    RYU_BUFFER_DIFF_VERSION
}

/// # Safety
/// `src` and `dst` must each be readable for `len` bytes. `out_offset` and
/// `out_length` may be null, otherwise each must be writable for one `usize`.
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

/// # Safety
/// `src` must be readable and `dst` writable for `len` bytes, and the two ranges
/// must not overlap. `out_offset` and `out_length` may be null, otherwise each must
/// be writable for one `usize`.
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

/// # Safety
/// `a` and `b` must each be readable for `len` bytes.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_buffer_diff_version(), RYU_BUFFER_DIFF_VERSION);
        assert_eq!(RYU_BUFFER_DIFF_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let src = [1u8; 64];
        let mut dst = [2u8; 64];
        let mut offset = 7usize;
        let mut length = 7usize;
        // SAFETY: null buffers are rejected before use; every non-null pointer
        // refers to a live local of the stated length.
        unsafe {
            assert_eq!(
                ryu_buffer_find_dirty_range(
                    ptr::null(),
                    dst.as_ptr(),
                    64,
                    &mut offset,
                    &mut length
                ),
                -1
            );
            assert_eq!(
                ryu_buffer_find_dirty_range(
                    src.as_ptr(),
                    ptr::null(),
                    64,
                    &mut offset,
                    &mut length
                ),
                -1
            );
            assert_eq!(
                ryu_buffer_diff_and_copy(
                    ptr::null(),
                    dst.as_mut_ptr(),
                    64,
                    &mut offset,
                    &mut length
                ),
                -1
            );
            assert_eq!(
                ryu_buffer_diff_and_copy(
                    src.as_ptr(),
                    ptr::null_mut(),
                    64,
                    &mut offset,
                    &mut length
                ),
                -1
            );
            assert_eq!((offset, length), (7, 7));
            assert_eq!(ryu_buffer_is_equal(ptr::null(), dst.as_ptr(), 64), -1);
            assert_eq!(ryu_buffer_is_equal(src.as_ptr(), ptr::null(), 64), -1);
            assert_eq!(ryu_buffer_is_equal(ptr::null(), ptr::null(), 0), -1);

            // The out-pointers are optional.
            assert_eq!(
                ryu_buffer_find_dirty_range(
                    src.as_ptr(),
                    dst.as_ptr(),
                    64,
                    ptr::null_mut(),
                    ptr::null_mut()
                ),
                1
            );
            assert_eq!(
                ryu_buffer_diff_and_copy(
                    src.as_ptr(),
                    dst.as_mut_ptr(),
                    64,
                    ptr::null_mut(),
                    ptr::null_mut()
                ),
                1
            );
            assert_eq!(src, dst);
        }
    }
}
