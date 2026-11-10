use crate::crc32::*;
use crate::lz4::*;
use crate::toc::*;
use std::slice;

pub const RYU_SHADER_CACHE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_shader_cache_version() -> u32 {
    RYU_SHADER_CACHE_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn ryu_shader_cache_validate_toc(
    data: *const u8,
    len: usize,
    expected_magic: u32,
    out_header: *mut TocHeader,
) -> i32 {
    if data.is_null() || len < 32 {
        return -1;
    }

    let slice = slice::from_raw_parts(data, len);
    match validate_header(slice, expected_magic) {
        Ok(header) => {
            if !out_header.is_null() {
                *out_header = header;
            }
            0
        }
        Err(TocError::InvalidMagic) => -2,
        Err(TocError::UnsupportedVersion) => -3,
        Err(_) => -4,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_shader_cache_parse_entries(
    data: *const u8,
    len: usize,
    out_entries: *mut OffsetAndSize,
    max_entries: usize,
) -> i32 {
    if data.is_null() || out_entries.is_null() || max_entries == 0 {
        return -1;
    }

    let slice = slice::from_raw_parts(data, len);
    let out_slice = slice::from_raw_parts_mut(out_entries, max_entries);

    match parse_toc_entries(slice, out_slice) {
        Ok(count) => count as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_shader_cache_decompress_lz4(
    src: *const u8,
    src_len: usize,
    dst: *mut u8,
    dst_cap: usize,
    out_uncompressed_len: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() || src_len == 0 || dst_cap == 0 {
        return -1;
    }

    let src_slice = slice::from_raw_parts(src, src_len);
    let dst_slice = slice::from_raw_parts_mut(dst, dst_cap);

    match decompress_lz4_block(src_slice, dst_slice) {
        Ok(decomp_len) => {
            if !out_uncompressed_len.is_null() {
                *out_uncompressed_len = decomp_len;
            }
            0
        }
        Err(_) => -2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_shader_cache_crc32(data: *const u8, len: usize) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = slice::from_raw_parts(data, len);
    compute_crc32(slice)
}
