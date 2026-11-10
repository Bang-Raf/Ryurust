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

fn map_error(err: SwizzleError) -> i32 {
    match err {
        SwizzleError::BufferTooSmall => SWIZZLE_STATUS_BUFFER_TOO_SMALL,
        SwizzleError::InvalidDimensions
        | SwizzleError::InvalidBytesPerPixel
        | SwizzleError::InvalidGobBlocks => SWIZZLE_STATUS_INVALID_ARGUMENT,
    }
}

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
