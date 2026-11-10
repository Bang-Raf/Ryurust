use crate::blender::blend_surface;
use crate::scaler::{deinterlace_bob, deinterlace_motion_adaptive, deinterlace_weave};
use crate::types::{VicColorMatrix, VicPixel, VicRectangle};
use std::slice;

pub const VIC_STATUS_OK: i32 = 0;
pub const VIC_STATUS_NULL_POINTER: i32 = -1;
pub const VIC_STATUS_INVALID_ARGUMENT: i32 = -2;

#[no_mangle]
pub unsafe extern "C" fn ryu_vic_blend_surface(
    dst: *mut VicPixel,
    dst_len: usize,
    dst_width: usize,
    dst_height: usize,
    src: *const VicPixel,
    src_len: usize,
    src_width: usize,
    src_height: usize,
    rect: *const VicRectangle,
    matrix: *const VicColorMatrix,
) -> i32 {
    if dst.is_null() || src.is_null() || rect.is_null() || matrix.is_null() {
        return VIC_STATUS_NULL_POINTER;
    }
    if dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0 {
        return VIC_STATUS_OK;
    }
    match (
        dst_width.checked_mul(dst_height),
        src_width.checked_mul(src_height),
    ) {
        (Some(d), Some(s)) if dst_len >= d && src_len >= s => {}
        _ => return VIC_STATUS_INVALID_ARGUMENT,
    }

    // SAFETY: the caller guarantees `dst_len` writable and `src_len` readable pixels behind
    // the (non-null) pointers, and that the two surfaces do not overlap; `rect` and `matrix`
    // are non-null and point at valid values.
    let dst_slice = slice::from_raw_parts_mut(dst, dst_len);
    let src_slice = slice::from_raw_parts(src, src_len);
    let target_rect = &*rect;
    let color_matrix = &*matrix;

    match blend_surface(
        dst_slice,
        dst_width,
        dst_height,
        src_slice,
        src_width,
        src_height,
        target_rect,
        color_matrix,
    ) {
        Ok(()) => VIC_STATUS_OK,
        Err(status) => status,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_vic_deinterlace_weave(
    data: *mut u8,
    data_len: usize,
    prev_data: *const u8,
    prev_len: usize,
    width: usize,
    field_size: usize,
    is_top_field: bool,
) -> i32 {
    if data.is_null() || prev_data.is_null() {
        return VIC_STATUS_NULL_POINTER;
    }
    if width == 0 || field_size == 0 {
        return VIC_STATUS_OK;
    }

    let data_slice = slice::from_raw_parts_mut(data, data_len);
    let prev_slice = slice::from_raw_parts(prev_data, prev_len);

    deinterlace_weave(data_slice, prev_slice, width, field_size, is_top_field);

    VIC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_vic_deinterlace_bob(
    data: *mut u8,
    data_len: usize,
    width: usize,
    field_size: usize,
    is_top_field: bool,
) -> i32 {
    if data.is_null() {
        return VIC_STATUS_NULL_POINTER;
    }
    if width == 0 || field_size == 0 {
        return VIC_STATUS_OK;
    }

    let data_slice = slice::from_raw_parts_mut(data, data_len);

    deinterlace_bob(data_slice, width, field_size, is_top_field);

    VIC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_vic_deinterlace_motion_adaptive(
    data: *mut u8,
    data_len: usize,
    prev_data: *const u8,
    prev_len: usize,
    next_data: *const u8,
    next_len: usize,
    width: usize,
    field_size: usize,
    is_top_field: bool,
) -> i32 {
    if data.is_null() || prev_data.is_null() || next_data.is_null() {
        return VIC_STATUS_NULL_POINTER;
    }
    if width == 0 || field_size == 0 {
        return VIC_STATUS_OK;
    }

    let data_slice = slice::from_raw_parts_mut(data, data_len);
    let prev_slice = slice::from_raw_parts(prev_data, prev_len);
    let next_slice = slice::from_raw_parts(next_data, next_len);

    deinterlace_motion_adaptive(
        data_slice,
        prev_slice,
        next_slice,
        width,
        field_size,
        is_top_field,
    );

    VIC_STATUS_OK
}
