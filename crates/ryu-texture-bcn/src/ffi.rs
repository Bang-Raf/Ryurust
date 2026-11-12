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

pub const RYU_TEXTURE_BCN_VERSION: u32 = 0x0001_0000; // 1.0.0

#[no_mangle]
pub extern "C" fn ryu_texture_bcn_version() -> u32 {
    RYU_TEXTURE_BCN_VERSION
}

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

/// # Safety
/// `src_ptr` must be valid for reads of `src_len` bytes and `dst_ptr` for writes of `dst_len`
/// bytes, without overlapping.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_texture_bcn_version(), RYU_TEXTURE_BCN_VERSION);
        assert_eq!(RYU_TEXTURE_BCN_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        const E: i32 = RYU_BCN_ERR_NULL_PTR;
        // SAFETY: every call passes null for both pointers, which each function rejects
        // before touching memory.
        unsafe {
            assert_eq!(ryu_texture_decode_bc1(null(), 8, null_mut(), 64, 4, 4), E);
            assert_eq!(ryu_texture_decode_bc2(null(), 16, null_mut(), 64, 4, 4), E);
            assert_eq!(ryu_texture_decode_bc3(null(), 16, null_mut(), 64, 4, 4), E);
            assert_eq!(
                ryu_texture_decode_bc4(null(), 8, null_mut(), 16, 4, 4, 0),
                E
            );
            assert_eq!(
                ryu_texture_decode_bc5(null(), 16, null_mut(), 32, 4, 4, 0),
                E
            );
            assert_eq!(
                ryu_texture_decode_bc6h(null(), 16, null_mut(), 128, 4, 4, 0),
                E
            );
            assert_eq!(ryu_texture_decode_bc7(null(), 16, null_mut(), 64, 4, 4), E);
        }
    }

    #[test]
    fn single_null_pointer_is_rejected() {
        let src = [0u8; 8];
        let mut dst = [0u8; 64];
        // SAFETY: the non-null pointer in each call is valid for the length passed with it.
        unsafe {
            assert_eq!(
                ryu_texture_decode_bc1(src.as_ptr(), 8, null_mut(), 64, 4, 4),
                RYU_BCN_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_texture_decode_bc1(null(), 8, dst.as_mut_ptr(), 64, 4, 4),
                RYU_BCN_ERR_NULL_PTR
            );
        }
    }
}
