//! C-ABI FFI exports for .NET 9 P/Invoke interop.

use crate::bc1::decode_bc1_surface;
use crate::bc2::decode_bc2_surface;
use crate::bc3::decode_bc3_surface;
use crate::bc4::decode_bc4_surface;
use crate::bc5::decode_bc5_surface;
use crate::bc6h::decode_bc6h_surface;
use crate::bc7::decode_bc7_surface;

pub const RYU_BCN_SUCCESS: i32 = 0;
pub const RYU_BCN_ERR_NULL_PTR: i32 = -1;
pub const RYU_BCN_ERR_BUFFER_TOO_SMALL: i32 = -2;
pub const RYU_BCN_ERR_INVALID_DIM: i32 = -3;

#[no_mangle]
pub extern "C" fn ryu_texture_bcn_version() -> u32 {
    0x00010000 // 1.0.0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc1(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc1_surface(src, dst, width, height) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc2(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc2_surface(src, dst, width, height) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc3(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc3_surface(src, dst, width, height) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc4(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
    signed: i32,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc4_surface(src, dst, width, height, signed != 0) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc5(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
    signed: i32,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc5_surface(src, dst, width, height, signed != 0) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc6h(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
    signed: i32,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc6h_surface(src, dst, width, height, signed != 0) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_texture_decode_bc7(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
    width: usize,
    height: usize,
) -> i32 {
    if src_ptr.is_null() || dst_ptr.is_null() {
        return RYU_BCN_ERR_NULL_PTR;
    }
    let src = std::slice::from_raw_parts(src_ptr, src_len);
    let dst = std::slice::from_raw_parts_mut(dst_ptr, dst_len);
    match decode_bc7_surface(src, dst, width, height) {
        Ok(()) => RYU_BCN_SUCCESS,
        Err(_) => RYU_BCN_ERR_BUFFER_TOO_SMALL,
    }
}
