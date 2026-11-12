//! C-ABI Foreign Function Interface for Tegra Block-Linear Swizzling.

use crate::swizzle::{
    convert_block_linear_to_linear, convert_block_linear_to_linear_parallel,
    convert_linear_to_block_linear, convert_linear_to_block_linear_parallel,
    query_block_linear_size, SwizzleError,
};

pub const SWIZZLE_STATUS_SUCCESS: i32 = 0;
pub const SWIZZLE_STATUS_NULL_POINTER: i32 = -1;
pub const SWIZZLE_STATUS_BUFFER_TOO_SMALL: i32 = -2;
pub const SWIZZLE_STATUS_INVALID_ARGUMENT: i32 = -3;

pub const RYU_TEGRA_SWIZZLE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_tegra_swizzle_version() -> u32 {
    RYU_TEGRA_SWIZZLE_VERSION
}

fn map_error(err: SwizzleError) -> i32 {
    match err {
        SwizzleError::BufferTooSmall => SWIZZLE_STATUS_BUFFER_TOO_SMALL,
        SwizzleError::InvalidDimensions
        | SwizzleError::InvalidBytesPerPixel
        | SwizzleError::InvalidGobBlocks => SWIZZLE_STATUS_INVALID_ARGUMENT,
    }
}

/// # Safety
/// This function takes no pointers and has no requirements; it is `unsafe` only because it
/// was originally exported that way.
#[no_mangle]
pub unsafe extern "C" fn ryu_swizzle_query_block_linear_size(
    width: i32,
    height: i32,
    depth: i32,
    bytes_per_pixel: i32,
    gob_blocks_in_y: i32,
    gob_blocks_in_z: i32,
) -> usize {
    if width <= 0
        || height <= 0
        || depth <= 0
        || bytes_per_pixel <= 0
        || gob_blocks_in_y <= 0
        || gob_blocks_in_z <= 0
    {
        return 0;
    }

    query_block_linear_size(
        width as usize,
        height as usize,
        depth as usize,
        bytes_per_pixel as usize,
        gob_blocks_in_y as usize,
        gob_blocks_in_z as usize,
    )
    .unwrap_or(0)
}

/// # Safety
/// `dst` must be valid for writes of `dst_len` bytes and `src` for reads of `src_len` bytes,
/// without overlapping.
#[no_mangle]
pub unsafe extern "C" fn ryu_swizzle_block_linear_to_pitch(
    dst: *mut u8,
    dst_len: usize,
    src: *const u8,
    src_len: usize,
    width: i32,
    height: i32,
    stride: i32,
    bytes_per_pixel: i32,
    gob_blocks_in_y: i32,
) -> i32 {
    if dst.is_null() || src.is_null() {
        return SWIZZLE_STATUS_NULL_POINTER;
    }
    if width <= 0 || height <= 0 || stride <= 0 || bytes_per_pixel <= 0 || gob_blocks_in_y <= 0 {
        return SWIZZLE_STATUS_INVALID_ARGUMENT;
    }

    let dst_slice = std::slice::from_raw_parts_mut(dst, dst_len);
    let src_slice = std::slice::from_raw_parts(src, src_len);

    match convert_block_linear_to_linear(
        dst_slice,
        src_slice,
        width as usize,
        height as usize,
        stride as usize,
        bytes_per_pixel as usize,
        gob_blocks_in_y as usize,
    ) {
        Ok(()) => SWIZZLE_STATUS_SUCCESS,
        Err(err) => map_error(err),
    }
}

/// # Safety
/// `dst` must be valid for writes of `dst_len` bytes and `src` for reads of `src_len` bytes,
/// without overlapping.
#[no_mangle]
pub unsafe extern "C" fn ryu_swizzle_block_linear_to_pitch_parallel(
    dst: *mut u8,
    dst_len: usize,
    src: *const u8,
    src_len: usize,
    width: i32,
    height: i32,
    stride: i32,
    bytes_per_pixel: i32,
    gob_blocks_in_y: i32,
) -> i32 {
    if dst.is_null() || src.is_null() {
        return SWIZZLE_STATUS_NULL_POINTER;
    }
    if width <= 0 || height <= 0 || stride <= 0 || bytes_per_pixel <= 0 || gob_blocks_in_y <= 0 {
        return SWIZZLE_STATUS_INVALID_ARGUMENT;
    }

    let dst_slice = std::slice::from_raw_parts_mut(dst, dst_len);
    let src_slice = std::slice::from_raw_parts(src, src_len);

    match convert_block_linear_to_linear_parallel(
        dst_slice,
        src_slice,
        width as usize,
        height as usize,
        stride as usize,
        bytes_per_pixel as usize,
        gob_blocks_in_y as usize,
    ) {
        Ok(()) => SWIZZLE_STATUS_SUCCESS,
        Err(err) => map_error(err),
    }
}

/// # Safety
/// `dst` must be valid for writes of `dst_len` bytes and `src` for reads of `src_len` bytes,
/// without overlapping.
#[no_mangle]
pub unsafe extern "C" fn ryu_swizzle_pitch_to_block_linear(
    dst: *mut u8,
    dst_len: usize,
    src: *const u8,
    src_len: usize,
    width: i32,
    height: i32,
    stride: i32,
    bytes_per_pixel: i32,
    gob_blocks_in_y: i32,
) -> i32 {
    if dst.is_null() || src.is_null() {
        return SWIZZLE_STATUS_NULL_POINTER;
    }
    if width <= 0 || height <= 0 || stride <= 0 || bytes_per_pixel <= 0 || gob_blocks_in_y <= 0 {
        return SWIZZLE_STATUS_INVALID_ARGUMENT;
    }

    let dst_slice = std::slice::from_raw_parts_mut(dst, dst_len);
    let src_slice = std::slice::from_raw_parts(src, src_len);

    match convert_linear_to_block_linear(
        dst_slice,
        src_slice,
        width as usize,
        height as usize,
        stride as usize,
        bytes_per_pixel as usize,
        gob_blocks_in_y as usize,
    ) {
        Ok(()) => SWIZZLE_STATUS_SUCCESS,
        Err(err) => map_error(err),
    }
}

/// # Safety
/// `dst` must be valid for writes of `dst_len` bytes and `src` for reads of `src_len` bytes,
/// without overlapping.
#[no_mangle]
pub unsafe extern "C" fn ryu_swizzle_pitch_to_block_linear_parallel(
    dst: *mut u8,
    dst_len: usize,
    src: *const u8,
    src_len: usize,
    width: i32,
    height: i32,
    stride: i32,
    bytes_per_pixel: i32,
    gob_blocks_in_y: i32,
) -> i32 {
    if dst.is_null() || src.is_null() {
        return SWIZZLE_STATUS_NULL_POINTER;
    }
    if width <= 0 || height <= 0 || stride <= 0 || bytes_per_pixel <= 0 || gob_blocks_in_y <= 0 {
        return SWIZZLE_STATUS_INVALID_ARGUMENT;
    }

    let dst_slice = std::slice::from_raw_parts_mut(dst, dst_len);
    let src_slice = std::slice::from_raw_parts(src, src_len);

    match convert_linear_to_block_linear_parallel(
        dst_slice,
        src_slice,
        width as usize,
        height as usize,
        stride as usize,
        bytes_per_pixel as usize,
        gob_blocks_in_y as usize,
    ) {
        Ok(()) => SWIZZLE_STATUS_SUCCESS,
        Err(err) => map_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_tegra_swizzle_version(), RYU_TEGRA_SWIZZLE_VERSION);
        assert_eq!(RYU_TEGRA_SWIZZLE_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        const E: i32 = SWIZZLE_STATUS_NULL_POINTER;
        // SAFETY: every call passes null for both pointers, which each function rejects
        // before touching memory.
        unsafe {
            assert_eq!(
                ryu_swizzle_block_linear_to_pitch(null_mut(), 64, null(), 512, 4, 4, 16, 4, 1),
                E
            );
            assert_eq!(
                ryu_swizzle_block_linear_to_pitch_parallel(
                    null_mut(),
                    64,
                    null(),
                    512,
                    4,
                    4,
                    16,
                    4,
                    1
                ),
                E
            );
            assert_eq!(
                ryu_swizzle_pitch_to_block_linear(null_mut(), 512, null(), 64, 4, 4, 16, 4, 1),
                E
            );
            assert_eq!(
                ryu_swizzle_pitch_to_block_linear_parallel(
                    null_mut(),
                    512,
                    null(),
                    64,
                    4,
                    4,
                    16,
                    4,
                    1
                ),
                E
            );
        }
    }

    #[test]
    fn non_power_of_two_bpp_stays_in_row() {
        // 3 pixels of 12 bytes: the 4-byte row tail must not run past `width * bpp`.
        let (w, h, bpp) = (3i32, 2i32, 12i32);
        let row = (w * bpp) as usize;
        let linear: Vec<u8> = (0..row * h as usize).map(|i| i as u8).collect();
        // SAFETY: takes no pointers.
        let size = unsafe { ryu_swizzle_query_block_linear_size(w, h, 1, bpp, 1, 1) };
        let mut block = vec![0u8; size];
        let mut back = vec![0xEEu8; row * h as usize + 16];
        // SAFETY: every buffer is a distinct local at least as long as the length passed.
        unsafe {
            assert_eq!(
                ryu_swizzle_pitch_to_block_linear(
                    block.as_mut_ptr(),
                    size,
                    linear.as_ptr(),
                    linear.len(),
                    w,
                    h,
                    row as i32,
                    bpp,
                    1
                ),
                SWIZZLE_STATUS_SUCCESS
            );
            assert_eq!(
                ryu_swizzle_block_linear_to_pitch(
                    back.as_mut_ptr(),
                    linear.len(),
                    block.as_ptr(),
                    size,
                    w,
                    h,
                    row as i32,
                    bpp,
                    1
                ),
                SWIZZLE_STATUS_SUCCESS
            );
        }
        assert_eq!(&back[..linear.len()], &linear[..]);
        assert!(back[linear.len()..].iter().all(|&b| b == 0xEE));
    }
}
