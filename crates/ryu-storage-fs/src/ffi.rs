use crate::aes_ctr::decrypt_nca_ctr;
use crate::lz4_codec::{compress_lz4_block, decompress_lz4_block};
use crate::pfs0::{find_pfs0_file, get_pfs0_entry, parse_pfs0_header};
use crate::zstd_codec::{compress_zstd_block, decompress_zstd_block};
use std::slice;

pub const RYU_STORAGE_OK: i32 = 0;
pub const RYU_STORAGE_ERR_NULL_PTR: i32 = -1;
pub const RYU_STORAGE_ERR_CAPACITY: i32 = -2;
pub const RYU_STORAGE_ERR_CORRUPT_HEADER: i32 = -3;
pub const RYU_STORAGE_ERR_DECOMPRESS: i32 = -4;
pub const RYU_STORAGE_ERR_NOT_FOUND: i32 = -5;

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_nca_decrypt_stream(
    src: *const u8,
    dst: *mut u8,
    len: usize,
    key: *const u8,
    counter: *const u8,
) -> i32 {
    ryu_storage_nca_decrypt_ctr_offset(src, dst, len, key, counter, 0)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_nca_decrypt_ctr_offset(
    src: *const u8,
    dst: *mut u8,
    len: usize,
    key: *const u8,
    base_counter: *const u8,
    offset_bytes: u64,
) -> i32 {
    if src.is_null() || dst.is_null() || key.is_null() || base_counter.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }
    if len == 0 {
        return RYU_STORAGE_OK;
    }

    let src_slice = slice::from_raw_parts(src, len);
    let dst_slice = slice::from_raw_parts_mut(dst, len);
    let key_arr = *(key as *const [u8; 16]);
    let counter_arr = *(base_counter as *const [u8; 16]);

    match decrypt_nca_ctr(src_slice, dst_slice, &key_arr, &counter_arr, offset_bytes) {
        Ok(_) => RYU_STORAGE_OK,
        Err(_) => RYU_STORAGE_ERR_DECOMPRESS,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_lz4_decompress_block(
    src: *const u8,
    src_len: usize,
    dst: *mut u8,
    dst_capacity: usize,
    out_decompressed_bytes: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() || out_decompressed_bytes.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }
    if src_len == 0 {
        *out_decompressed_bytes = 0;
        return RYU_STORAGE_OK;
    }

    let src_slice = slice::from_raw_parts(src, src_len);
    let dst_slice = slice::from_raw_parts_mut(dst, dst_capacity);

    match decompress_lz4_block(src_slice, dst_slice) {
        Ok(decompressed_len) => {
            *out_decompressed_bytes = decompressed_len;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_DECOMPRESS,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_lz4_compress_block(
    src: *const u8,
    src_len: usize,
    dst: *mut u8,
    dst_capacity: usize,
    out_compressed_bytes: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() || out_compressed_bytes.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }
    if src_len == 0 {
        *out_compressed_bytes = 0;
        return RYU_STORAGE_OK;
    }

    let src_slice = slice::from_raw_parts(src, src_len);
    let dst_slice = slice::from_raw_parts_mut(dst, dst_capacity);

    match compress_lz4_block(src_slice, dst_slice) {
        Ok(compressed_len) => {
            *out_compressed_bytes = compressed_len;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_CAPACITY,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_zstd_decompress_block(
    src: *const u8,
    src_len: usize,
    dst: *mut u8,
    dst_capacity: usize,
    out_decompressed_bytes: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() || out_decompressed_bytes.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }
    if src_len == 0 {
        *out_decompressed_bytes = 0;
        return RYU_STORAGE_OK;
    }

    let src_slice = slice::from_raw_parts(src, src_len);
    let dst_slice = slice::from_raw_parts_mut(dst, dst_capacity);

    match decompress_zstd_block(src_slice, dst_slice) {
        Ok(decompressed_len) => {
            *out_decompressed_bytes = decompressed_len;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_DECOMPRESS,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_zstd_compress_block(
    src: *const u8,
    src_len: usize,
    dst: *mut u8,
    dst_capacity: usize,
    compression_level: i32,
    out_compressed_bytes: *mut usize,
) -> i32 {
    if src.is_null() || dst.is_null() || out_compressed_bytes.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }
    if src_len == 0 {
        *out_compressed_bytes = 0;
        return RYU_STORAGE_OK;
    }

    let src_slice = slice::from_raw_parts(src, src_len);
    let dst_slice = slice::from_raw_parts_mut(dst, dst_capacity);

    match compress_zstd_block(src_slice, dst_slice, compression_level) {
        Ok(compressed_len) => {
            *out_compressed_bytes = compressed_len;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_CAPACITY,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_pfs0_verify_header(
    header_ptr: *const u8,
    header_len: usize,
    out_num_files: *mut u32,
    out_string_table_size: *mut u32,
) -> i32 {
    if header_ptr.is_null() || out_num_files.is_null() || out_string_table_size.is_null() {
        return RYU_STORAGE_ERR_NULL_PTR;
    }

    let data = slice::from_raw_parts(header_ptr, header_len);
    match parse_pfs0_header(data) {
        Ok(info) => {
            *out_num_files = info.num_files;
            *out_string_table_size = info.string_table_size;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_CORRUPT_HEADER,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_pfs0_find_file(
    header_ptr: *const u8,
    header_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    out_offset: *mut u64,
    out_size: *mut u64,
) -> i32 {
    if header_ptr.is_null() || file_name_ptr.is_null() || out_offset.is_null() || out_size.is_null()
    {
        return RYU_STORAGE_ERR_NULL_PTR;
    }

    let data = slice::from_raw_parts(header_ptr, header_len);
    let name_bytes = slice::from_raw_parts(file_name_ptr, file_name_len);
    let name_str = match std::str::from_utf8(name_bytes) {
        Ok(s) => s,
        Err(_) => return RYU_STORAGE_ERR_NOT_FOUND,
    };

    match find_pfs0_file(data, name_str) {
        Ok(entry) => {
            *out_offset = entry.offset;
            *out_size = entry.size;
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_NOT_FOUND,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_storage_pfs0_get_entry(
    header_ptr: *const u8,
    header_len: usize,
    index: u32,
    out_offset: *mut u64,
    out_size: *mut u64,
    name_buf: *mut u8,
    name_buf_capacity: usize,
    out_name_len: *mut usize,
) -> i32 {
    if header_ptr.is_null()
        || out_offset.is_null()
        || out_size.is_null()
        || name_buf.is_null()
        || out_name_len.is_null()
    {
        return RYU_STORAGE_ERR_NULL_PTR;
    }

    let data = slice::from_raw_parts(header_ptr, header_len);
    match get_pfs0_entry(data, index) {
        Ok((entry, name)) => {
            let name_bytes = name.as_bytes();
            if name_bytes.len() > name_buf_capacity {
                return RYU_STORAGE_ERR_CAPACITY;
            }
            let name_slice = slice::from_raw_parts_mut(name_buf, name_buf_capacity);
            name_slice[..name_bytes.len()].copy_from_slice(name_bytes);
            *out_offset = entry.offset;
            *out_size = entry.size;
            *out_name_len = name_bytes.len();
            RYU_STORAGE_OK
        }
        Err(_) => RYU_STORAGE_ERR_NOT_FOUND,
    }
}
