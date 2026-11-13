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

/// `a * b` as a slice length: `None` if it overflows or exceeds `isize::MAX`
/// (the limit for `slice::from_raw_parts`).
fn plane_len(a: usize, b: usize) -> Option<usize> {
    a.checked_mul(b).filter(|&n| n <= isize::MAX as usize)
}

pub const RYU_VIDEO_NVDEC_VERSION: u32 = 0x0001_0000; // v1.0.0

/// Kept for compatibility with existing callers; same value as `ryu_video_nvdec_version`.
#[no_mangle]
pub extern "C" fn ryu_nvdec_get_version() -> u32 {
    RYU_VIDEO_NVDEC_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_video_nvdec_version() -> u32 {
    RYU_VIDEO_NVDEC_VERSION
}

/// # Safety
/// `input` must be valid for reads of 16 coefficients and `dest` for reads and writes of
/// `stride * 4` bytes, without overlapping.
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
    let Some(dest_len) = plane_len(stride, 4) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    idct4x4_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `input` must be valid for reads of 64 coefficients and `dest` for reads and writes of
/// `stride * 8` bytes, without overlapping.
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
    let Some(dest_len) = plane_len(stride, 8) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    idct8x8_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `input` must be valid for reads of 256 coefficients and `dest` for reads and writes of
/// `stride * 16` bytes, without overlapping.
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
    let Some(dest_len) = plane_len(stride, 16) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    idct16x16_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `input` must be valid for reads of 1024 coefficients and `dest` for reads and writes of
/// `stride * 32` bytes, without overlapping.
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
    let Some(dest_len) = plane_len(stride, 32) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    idct32x32_add(input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `input` must be valid for reads of 16 coefficients and `dest` for reads and writes of
/// `stride * 4` bytes, without overlapping.
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
    if stride < 4 || !(0..=3).contains(&tx_type) || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 16);
    let Some(dest_len) = plane_len(stride, 4) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    iht4x4_add(tx_type, input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `input` must be valid for reads of 64 coefficients and `dest` for reads and writes of
/// `stride * 8` bytes, without overlapping.
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
    if stride < 8 || !(0..=3).contains(&tx_type) || eob < 0 {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    }

    let input_slice = std::slice::from_raw_parts(input, 64);
    let Some(dest_len) = plane_len(stride, 8) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    let dest_slice = std::slice::from_raw_parts_mut(dest, dest_len);
    iht8x8_add(tx_type, input_slice, dest_slice, stride, eob);
    NVDEC_STATUS_OK
}

/// # Safety
/// `s` must be valid for reads and writes of `buffer_len` bytes.
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

/// # Safety
/// The Y, U and V planes must be valid for reads and writes of `y_len`, `u_len` and `v_len`
/// bytes respectively and must not overlap each other.
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
    let (Some(y_need), Some(uv_need)) = (
        plane_len(height, y_stride),
        plane_len(height.div_ceil(2), uv_stride),
    ) else {
        return NVDEC_STATUS_INVALID_ARGUMENT;
    };
    if y_len < y_need || u_len < uv_need || v_len < uv_need {
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

/// # Safety
/// `bitstream` must be valid for reads of `bitstream_len` bytes and `config` must point to a
/// valid `Vp9FrameConfig`. The Y, U and V output planes must be valid for reads and writes of
/// `y_len`, `u_len` and `v_len` bytes respectively and must not overlap each other or the inputs.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_exports_return_constant() {
        assert_eq!(ryu_video_nvdec_version(), RYU_VIDEO_NVDEC_VERSION);
        assert_eq!(ryu_nvdec_get_version(), RYU_VIDEO_NVDEC_VERSION);
        assert_eq!(RYU_VIDEO_NVDEC_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        const E: i32 = NVDEC_STATUS_NULL_POINTER;
        // SAFETY: every call passes null for the required pointers, which each function
        // rejects before touching memory.
        unsafe {
            assert_eq!(ryu_nvdec_vp9_idct4x4_add(null(), null_mut(), 4, 1), E);
            assert_eq!(ryu_nvdec_vp9_idct8x8_add(null(), null_mut(), 8, 1), E);
            assert_eq!(ryu_nvdec_vp9_idct16x16_add(null(), null_mut(), 16, 1), E);
            assert_eq!(ryu_nvdec_vp9_idct32x32_add(null(), null_mut(), 32, 1), E);
            assert_eq!(ryu_nvdec_vp9_iht4x4_add(null(), null_mut(), 4, 0, 1), E);
            assert_eq!(ryu_nvdec_vp9_iht8x8_add(null(), null_mut(), 8, 0, 1), E);
            assert_eq!(
                ryu_nvdec_vp9_loop_filter_edge(null_mut(), 64, 32, 8, 1, 1, 1, 4, 0),
                E
            );
            assert_eq!(
                ryu_nvdec_vp9_loop_filter_frame(
                    null_mut(),
                    64,
                    null_mut(),
                    16,
                    null_mut(),
                    16,
                    8,
                    8,
                    8,
                    4,
                    1,
                    0
                ),
                E
            );
            assert_eq!(
                ryu_nvdec_decode_vp9_frame(
                    null(),
                    16,
                    null(),
                    null_mut(),
                    64,
                    null_mut(),
                    16,
                    null_mut(),
                    16,
                    8,
                    4
                ),
                E
            );
        }
    }

    #[test]
    fn overflowing_plane_sizes_are_rejected() {
        let coeffs = [0i32; 1024];
        let mut buf = [0u8; 64];
        let huge = usize::MAX / 2;
        let cfg = Vp9FrameConfig {
            width: 16,
            height: 16,
            is_key_frame: 1,
            intra_only: 0,
            filter_level: 0,
            sharpness: 0,
            base_qindex: 0,
            transform_mode: 0,
            lossless: 0,
        };
        let bits = [0u8; 4];
        // SAFETY: every call is rejected before any pointer is turned into a slice.
        unsafe {
            let d = buf.as_mut_ptr();
            assert_eq!(
                ryu_nvdec_vp9_idct4x4_add(coeffs.as_ptr(), d, huge, 1),
                NVDEC_STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                ryu_nvdec_vp9_idct32x32_add(coeffs.as_ptr(), d, huge, 1),
                NVDEC_STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                ryu_nvdec_vp9_loop_filter_frame(d, 64, d, 64, d, 64, 8, huge, 8, 4, 1, 0),
                NVDEC_STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                ryu_nvdec_decode_vp9_frame(bits.as_ptr(), 4, &cfg, d, 64, d, 64, d, 64, huge, huge),
                NVDEC_STATUS_INVALID_ARGUMENT
            );
            // Stride narrower than the frame: rows would overlap.
            assert_eq!(
                ryu_nvdec_decode_vp9_frame(bits.as_ptr(), 4, &cfg, d, 64, d, 64, d, 64, 8, 8),
                NVDEC_STATUS_INVALID_ARGUMENT
            );
        }
    }
}
