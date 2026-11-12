use crate::page_table::Radix48PageTable;
use std::slice;

pub const RYU_PAGETABLE_OK: i32 = 0;
pub const RYU_PAGETABLE_ERR_NULL_PTR: i32 = -1;

pub const RYU_MEMORY_PAGETABLE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_memory_pagetable_version() -> u32 {
    RYU_MEMORY_PAGETABLE_VERSION
}

/// # Safety
/// Always sound to call; release the returned handle with `ryu_pagetable_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_create() -> *mut Radix48PageTable {
    Box::into_raw(Box::new(Radix48PageTable::new()))
}

/// # Safety
/// `pt` must be null or a handle from `ryu_pagetable_create` that has not been
/// destroyed yet; it must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_destroy(pt: *mut Radix48PageTable) {
    if !pt.is_null() {
        drop(Box::from_raw(pt));
    }
}

/// Returns 0 (the unmapped value) when `pt` is null.
///
/// # Safety
/// `pt` must be null or a live handle from `ryu_pagetable_create`, with no concurrent
/// mutating call (`map`/`unmap`/`destroy`) on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_read(pt: *const Radix48PageTable, va: u64) -> u64 {
    if pt.is_null() {
        return 0;
    }
    (*pt).read(va)
}

/// # Safety
/// `pt` must be null (no-op) or a live handle from `ryu_pagetable_create`, not accessed
/// concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_map(pt: *mut Radix48PageTable, va: u64, value: u64) {
    if !pt.is_null() {
        (*pt).map(va, value);
    }
}

/// # Safety
/// `pt` must be null (no-op) or a live handle from `ryu_pagetable_create`, not accessed
/// concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_unmap(pt: *mut Radix48PageTable, va: u64) {
    if !pt.is_null() {
        (*pt).unmap(va);
    }
}

/// # Safety
/// `pt` must be null (no-op) or a live handle from `ryu_pagetable_create`, not accessed
/// concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_map_range(
    pt: *mut Radix48PageTable,
    va: u64,
    size: u64,
    host_ptr_base: u64,
) {
    if !pt.is_null() {
        (*pt).map_range(va, size, host_ptr_base);
    }
}

/// # Safety
/// `pt` must be null (no-op) or a live handle from `ryu_pagetable_create`, not accessed
/// concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_unmap_range(pt: *mut Radix48PageTable, va: u64, size: u64) {
    if !pt.is_null() {
        (*pt).unmap_range(va, size);
    }
}

/// # Safety
/// `pt` must be null or a live handle with no concurrent mutating call. `vas` must be
/// valid for reads and `out_values` for writes of `count` `u64`s, and the two buffers
/// must not overlap.
#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_read_batch(
    pt: *const Radix48PageTable,
    vas: *const u64,
    count: usize,
    out_values: *mut u64,
) -> i32 {
    if pt.is_null() || vas.is_null() || out_values.is_null() {
        return RYU_PAGETABLE_ERR_NULL_PTR;
    }
    if count == 0 {
        return RYU_PAGETABLE_OK;
    }

    let va_slice = slice::from_raw_parts(vas, count);
    let out_slice = slice::from_raw_parts_mut(out_values, count);

    for i in 0..count {
        out_slice[i] = (*pt).read(va_slice[i]);
    }

    RYU_PAGETABLE_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_memory_pagetable_version(), RYU_MEMORY_PAGETABLE_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        unsafe {
            ryu_pagetable_destroy(ptr::null_mut());
            assert_eq!(ryu_pagetable_read(ptr::null(), 0x1000), 0);
            ryu_pagetable_map(ptr::null_mut(), 0x1000, 1);
            ryu_pagetable_unmap(ptr::null_mut(), 0x1000);
            ryu_pagetable_map_range(ptr::null_mut(), 0x1000, 0x2000, 1);
            ryu_pagetable_unmap_range(ptr::null_mut(), 0x1000, 0x2000);

            let pt = ryu_pagetable_create();
            let vas = [0x1000u64; 2];
            let mut out = [0u64; 2];
            assert_eq!(
                ryu_pagetable_read_batch(ptr::null(), vas.as_ptr(), 2, out.as_mut_ptr()),
                RYU_PAGETABLE_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_pagetable_read_batch(pt, ptr::null(), 2, out.as_mut_ptr()),
                RYU_PAGETABLE_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_pagetable_read_batch(pt, vas.as_ptr(), 2, ptr::null_mut()),
                RYU_PAGETABLE_ERR_NULL_PTR
            );
            ryu_pagetable_destroy(pt);
        }
    }
}
