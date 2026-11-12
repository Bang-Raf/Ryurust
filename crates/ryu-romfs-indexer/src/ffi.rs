use crate::indexer::{romfs_hash, RyuRomFs};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

pub const RYU_ROMFS_INDEXER_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_romfs_indexer_version() -> u32 {
    RYU_ROMFS_INDEXER_VERSION
}

/// Parses RomFS metadata. Returns a handle, or null if `data` is null, `len < 0x50`, or the
/// metadata is invalid. The handle does not borrow `data`.
///
/// # Safety
/// `data` must be null or valid for reads of `len` bytes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_create(data: *const u8, len: usize) -> *mut RyuRomFs {
    if data.is_null() || len < 0x50 {
        return std::ptr::null_mut();
    }
    // SAFETY: `data` is non-null and valid for `len` reads, per the caller contract.
    let slice = unsafe { slice::from_raw_parts(data, len) };
    match RyuRomFs::from_metadata_bytes(slice) {
        Some(r) => Box::into_raw(Box::new(r)),
        None => std::ptr::null_mut(),
    }
}

/// Frees a handle. Null is a no-op.
///
/// # Safety
/// `romfs` must be null or a handle from `ryu_romfs_create` that has not been destroyed yet;
/// it must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_destroy(romfs: *mut RyuRomFs) {
    if !romfs.is_null() {
        // SAFETY: non-null `romfs` came from `Box::into_raw` in `ryu_romfs_create` and the
        // caller guarantees it is not freed twice.
        unsafe {
            drop(Box::from_raw(romfs));
        }
    }
}

/// Looks up `path`. Returns `1` and fills the non-null outputs if found, `0` if not found,
/// `-1` if `romfs` or `path` is null or `path` is not UTF-8.
///
/// # Safety
/// `romfs` must be null or a live handle from `ryu_romfs_create`. `path` must be null or a
/// NUL-terminated string. Each output pointer must be null or valid for one aligned `u64` write.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_find_file(
    romfs: *const RyuRomFs,
    path: *const c_char,
    out_offset: *mut u64,
    out_size: *mut u64,
) -> i32 {
    if romfs.is_null() || path.is_null() {
        return -1;
    }
    // SAFETY: non-null live handle, per the caller contract; only read here.
    let r = unsafe { &*romfs };
    // SAFETY: `path` is non-null and NUL-terminated, per the caller contract.
    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Some(info) = r.find_file(path_str) {
        if !out_offset.is_null() {
            // SAFETY: non-null, and valid for a `u64` write per the caller contract.
            unsafe { *out_offset = info.offset };
        }
        if !out_size.is_null() {
            // SAFETY: non-null, and valid for a `u64` write per the caller contract.
            unsafe { *out_size = info.size };
        }
        1 // Found
    } else {
        0 // Not found
    }
}

/// RomFS name hash of `name[..name_len]` under `parent`. A null `name` (with any `name_len`)
/// or `name_len == 0` hashes the empty name.
///
/// # Safety
/// `name` must be null or valid for reads of `name_len` bytes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_hash(parent: u32, name: *const u8, name_len: usize) -> u32 {
    if name.is_null() || name_len == 0 {
        return romfs_hash(parent, &[]);
    }
    // SAFETY: `name` is non-null and valid for `name_len` reads, per the caller contract.
    let slice = unsafe { slice::from_raw_parts(name, name_len) };
    romfs_hash(parent, slice)
}

/// Number of files, or `0` for a null handle.
///
/// # Safety
/// `romfs` must be null or a live handle from `ryu_romfs_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_get_file_count(romfs: *const RyuRomFs) -> usize {
    if romfs.is_null() {
        return 0;
    }
    // SAFETY: non-null live handle, per the caller contract; only read here.
    let r = unsafe { &*romfs };
    r.file_count()
}

/// Number of directories, or `0` for a null handle.
///
/// # Safety
/// `romfs` must be null or a live handle from `ryu_romfs_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_romfs_get_dir_count(romfs: *const RyuRomFs) -> usize {
    if romfs.is_null() {
        return 0;
    }
    // SAFETY: non-null live handle, per the caller contract; only read here.
    let r = unsafe { &*romfs };
    r.dir_count()
}
