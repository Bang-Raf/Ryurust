use crate::crc32::*;
use crate::lz4::*;
use crate::toc::*;
use std::slice;

pub const RYU_SHADER_CACHE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_shader_cache_version() -> u32 {
    RYU_SHADER_CACHE_VERSION
}

/// # Safety
/// `data` must be null or readable for `len` bytes. `out_header` may be null,
/// otherwise it must be writable for one `TocHeader`.
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

/// # Safety
/// `data` must be null or readable for `len` bytes. `out_entries` must be null or
/// writable for `max_entries` elements and must not overlap `data`.
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

    // Negative results match `ryu_shader_cache_validate_toc`'s header error codes.
    match parse_toc_entries(slice, out_slice) {
        Ok(count) => count as i32,
        Err(TocError::InvalidMagic) => -2,
        Err(TocError::UnsupportedVersion) => -3,
        Err(_) => -1,
    }
}

/// # Safety
/// `src` must be null or readable for `src_len` bytes and `dst` null or writable for
/// `dst_cap` bytes, without overlap. `out_uncompressed_len` may be null, otherwise it
/// must be writable for one `usize`.
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

/// Returns 0 for a null or empty buffer (there is no separate error channel).
///
/// # Safety
/// `data` must be null or readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_shader_cache_crc32(data: *const u8, len: usize) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = slice::from_raw_parts(data, len);
    compute_crc32(slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_shader_cache_version(), RYU_SHADER_CACHE_VERSION);
        assert_eq!(RYU_SHADER_CACHE_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let data = [0u8; 64];
        let mut out = [0u8; 64];
        let mut entries = [OffsetAndSize::default(); 4];
        let mut len = 7usize;
        // SAFETY: null buffers are rejected before use; every non-null pointer
        // refers to a live local of the stated length.
        unsafe {
            assert_eq!(
                ryu_shader_cache_validate_toc(ptr::null(), 64, 0, ptr::null_mut()),
                -1
            );
            // A null `out_header` is optional: validation still runs (bad magic here).
            assert_eq!(
                ryu_shader_cache_validate_toc(data.as_ptr(), 64, 0x1234_5678, ptr::null_mut()),
                -2
            );
            assert_eq!(
                ryu_shader_cache_parse_entries(ptr::null(), 64, entries.as_mut_ptr(), 4),
                -1
            );
            assert_eq!(
                ryu_shader_cache_parse_entries(data.as_ptr(), 64, ptr::null_mut(), 4),
                -1
            );
            assert_eq!(
                ryu_shader_cache_decompress_lz4(ptr::null(), 64, out.as_mut_ptr(), 64, &mut len),
                -1
            );
            assert_eq!(
                ryu_shader_cache_decompress_lz4(data.as_ptr(), 64, ptr::null_mut(), 64, &mut len),
                -1
            );
            assert_eq!(len, 7);
            assert_eq!(ryu_shader_cache_crc32(ptr::null(), 64), 0);
        }
    }
}
