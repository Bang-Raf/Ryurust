// Blittable C-ABI FFI Layer for Ryubing Native NVDEC / VP9 Video Acceleration

use crate::idct::{idct16x16_add, idct32x32_add, idct4x4_add, idct8x8_add, iht4x4_add, iht8x8_add};
use crate::loop_filter::{
    loop_filter_frame, lpf_horizontal_16, lpf_horizontal_4, lpf_horizontal_8, lpf_vertical_16,
    lpf_vertical_4, lpf_vertical_8,
};
use crate::vp9::{Vp9DecoderPipeline, Vp9FrameConfig};

pub const NVDEC_STATUS_OK: i32 = 0;
pub const NVDEC_STATUS_NULL_POINTER: i32 = -1;
pub const NVDEC_STATUS_INVALID_ARGUMENT: i32 = -2;
pub const NVDEC_STATUS_BUFFER_TOO_SMALL: i32 = -3;
pub const NVDEC_STATUS_CORRUPT_BITSTREAM: i32 = -4;

#[no_mangle]
pub extern "C" fn ryu_nvdec_get_version() -> u32 {
    0x00010000 // v1.0.0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_idct4x4_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 4 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 16);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 4);
    idct4x4_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_idct8x8_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 8 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 64);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 8);
    idct8x8_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_idct16x16_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 16 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 256);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 16);
    idct16x16_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_idct32x32_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 32 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 1024);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 32);
    idct32x32_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_iht4x4_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    tx_type: i32,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 4 || tx_type < 0 || tx_type > 3 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 16);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 4);
    iht4x4_add(tx_type, input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_iht8x8_add(
    input: *const i32,
    dest: *mut u8,
    stride: usize,
    tx_type: i32,
    eob: i32,
) -> i32 {
    if input.is_null() || dest.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if stride < 8 || tx_type < 0 || tx_type > 3 || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 64);
    let dest_slice = std::slice::from_raw_parts_mut(dest, stride * 8);
    iht8x8_add(tx_type, input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_loop_filter_edge(
    s: *mut u8,
    buffer_len: usize,
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
    size: i32,
    is_vertical: i32,
) -> i32 {
    if s.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if pitch == 0 || buffer_len == 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let slice = std::slice::from_raw_parts_mut(s, buffer_len);

    match (size, is_vertical != 0) {
        (4, false) => lpf_horizontal_4(slice, offset, pitch, blimit, limit, thresh),
        (4, true) => lpf_vertical_4(slice, offset, pitch, blimit, limit, thresh),
        (8, false) => lpf_horizontal_8(slice, offset, pitch, blimit, limit, thresh),
        (8, true) => lpf_vertical_8(slice, offset, pitch, blimit, limit, thresh),
        (16, false) => lpf_horizontal_16(slice, offset, pitch, blimit, limit, thresh),
        (16, true) => lpf_vertical_16(slice, offset, pitch, blimit, limit, thresh),
        _ => return NVDEC_STATUS_INVALID_ARGUMENT,
    }

    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_vp9_loop_filter_frame(
    y_ptr: *mut u8,
    y_len: usize,
    u_ptr: *mut u8,
    u_len: usize,
    v_ptr: *mut u8,
    v_len: usize,
    width: usize,
    height: usize,
    y_stride: usize,
    uv_stride: usize,
    filter_level: u8,
    sharpness: u8,
) -> i32 {
    if y_ptr.is_null() || u_ptr.is_null() || v_ptr.is_null() {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if width < 8 || height < 8 || y_stride < width || uv_stride < width / 2 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }
    if y_len < height * y_stride
        || u_len < ((height + 1) / 2) * uv_stride
        || v_len < ((height + 1) / 2) * uv_stride
    {
        return NVDEC_STATUS_BUFFER_TOO_SMALL;
    }

    let y_slice = std::slice::from_raw_parts_mut(y_ptr, y_len);
    let u_slice = std::slice::from_raw_parts_mut(u_ptr, u_len);
    let v_slice = std::slice::from_raw_parts_mut(v_ptr, v_len);

    loop_filter_frame(
        y_slice,
        u_slice,
        v_slice,
        width,
        height,
        y_stride,
        uv_stride,
        filter_level,
        sharpness,
    );

    NVDEC_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_nvdec_decode_vp9_frame(
    bitstream: *const u8,
    bitstream_len: usize,
    config: *const Vp9FrameConfig,
    y_out: *mut u8,
    y_len: usize,
    u_out: *mut u8,
    u_len: usize,
    v_out: *mut u8,
    v_len: usize,
    y_stride: usize,
    uv_stride: usize,
) -> i32 {
    if bitstream.is_null()
        || config.is_null()
        || y_out.is_null()
        || u_out.is_null()
        || v_out.is_null()
    {
        return NVDEC_STATUS_NULL_POINTER;
    }
    if bitstream_len == 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let bitstream_slice = std::slice::from_raw_parts(bitstream, bitstream_len);
    let cfg = &*config;

    let y_slice = std::slice::from_raw_parts_mut(y_out, y_len);
    let u_slice = std::slice::from_raw_parts_mut(u_out, u_len);
    let v_slice = std::slice::from_raw_parts_mut(v_out, v_len);

    match Vp9DecoderPipeline::decode_frame(
        bitstream_slice,
        cfg,
        y_slice,
        u_slice,
        v_slice,
        y_stride,
        uv_stride,
    ) {
        Ok(()) => NVDEC_STATUS_OK,
        Err(err) => err,
    }
}
