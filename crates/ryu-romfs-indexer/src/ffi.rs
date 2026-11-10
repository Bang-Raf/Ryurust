use crate::indexer::{romfs_hash, RyuRomFs};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

pub const RYU_ROMFS_INDEXER_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_romfs_indexer_version() -> u32 {
    RYU_ROMFS_INDEXER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_romfs_create(data: *const u8, len: usize) -> *mut RyuRomFs {
    if data.is_null() || len < 0x50 {
        return std::ptr::null_mut();
    }
    let slice = unsafe { slice::from_raw_parts(data, len) };
    match RyuRomFs::from_metadata_bytes(slice) {
        Some(r) => Box::into_raw(Box::new(r)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ryu_romfs_destroy(romfs: *mut RyuRomFs) {
    if !romfs.is_null() {
        unsafe {
            drop(Box::from_raw(romfs));
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_romfs_find_file(
    romfs: *const RyuRomFs,
    path: *const c_char,
    out_offset: *mut u64,
    out_size: *mut u64,
) -> i32 {
    if romfs.is_null() || path.is_null() {
        return -1;
    }
    let r = unsafe { &*romfs };
    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Some(info) = r.find_file(path_str) {
        if !out_offset.is_null() {
            unsafe { *out_offset = info.offset };
        }
        if !out_size.is_null() {
            unsafe { *out_size = info.size };
        }
        1 // Found
    } else {
        0 // Not found
    }
}

#[no_mangle]
pub extern "C" fn ryu_romfs_hash(parent: u32, name: *const u8, name_len: usize) -> u32 {
    if name.is_null() || name_len == 0 {
        return romfs_hash(parent, &[]);
    }
    let slice = unsafe { slice::from_raw_parts(name, name_len) };
    romfs_hash(parent, slice)
}

#[no_mangle]
pub extern "C" fn ryu_romfs_get_file_count(romfs: *const RyuRomFs) -> usize {
    if romfs.is_null() {
        return 0;
    }
    let r = unsafe { &*romfs };
    r.file_count()
}

#[no_mangle]
pub extern "C" fn ryu_romfs_get_dir_count(romfs: *const RyuRomFs) -> usize {
    if romfs.is_null() {
        return 0;
    }
    let r = unsafe { &*romfs };
    r.dir_count()
}
