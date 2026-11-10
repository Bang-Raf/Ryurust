//! C-ABI FFI exports for .NET 9 P/Invoke interop.

use crate::transcode::*;

pub const RYU_PIXEL_SUCCESS: i32 = 0;
pub const RYU_PIXEL_ERR_NULL_PTR: i32 = -1;
pub const RYU_PIXEL_ERR_BUFFER_TOO_SMALL: i32 = -2;
pub const RYU_PIXEL_ERR_INVALID_PARAM: i32 = -3;

#[no_mangle]
pub extern "C" fn ryu_pixel_converter_version() -> u32 {
    0x00010000 // 1.0.0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_r4g4_to_r4g4b4a4(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    if width == 0 {
        return RYU_PIXEL_SUCCESS;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_r4g4_to_r4g4b4a4(src, dst, width) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_r5g6b5_to_r8g8b8a8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    if width == 0 {
        return RYU_PIXEL_SUCCESS;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_r5g6b5_to_r8g8b8a8(src, dst, width) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_r5g6b5_to_rgba8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
) -> i32 {
    ryu_pixel_convert_r5g6b5_to_r8g8b8a8(src_ptr, src_len, dst_ptr, dst_len, width)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_r5g5b5_to_r8g8b8a8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    force_alpha: i32,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    if width == 0 {
        return RYU_PIXEL_SUCCESS;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_r5g5b5_to_r8g8b8a8(src, dst, width, force_alpha != 0) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_a1b5g5r5_to_r8g8b8a8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    if width == 0 {
        return RYU_PIXEL_SUCCESS;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_a1b5g5r5_to_r8g8b8a8(src, dst, width) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_r4g4b4a4_to_r8g8b8a8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    if width == 0 {
        return RYU_PIXEL_SUCCESS;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_r4g4b4a4_to_r8g8b8a8(src, dst, width) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pixel_convert_bgra8_to_rgba8(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_PIXEL_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match convert_bgra8_to_rgba8(src, dst) {
        Ok(()) => RYU_PIXEL_SUCCESS,
        Err(_) => RYU_PIXEL_ERR_BUFFER_TOO_SMALL,
    }
}
