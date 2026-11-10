use crate::page_table::Radix48PageTable;
use std::slice;

pub const RYU_PAGETABLE_OK: i32 = 0;
pub const RYU_PAGETABLE_ERR_NULL_PTR: i32 = -1;

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_create() -> *mut Radix48PageTable {
    Box::into_raw(Box::new(Radix48PageTable::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_destroy(pt: *mut Radix48PageTable) {
    if !pt.is_null() {
        drop(Box::from_raw(pt));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_read(pt: *const Radix48PageTable, va: u64) -> u64 {
    if pt.is_null() {
        return 0;
    }
    (*pt).read(va)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_map(pt: *mut Radix48PageTable, va: u64, value: u64) {
    if !pt.is_null() {
        (*pt).map(va, value);
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_unmap(pt: *mut Radix48PageTable, va: u64) {
    if !pt.is_null() {
        (*pt).unmap(va);
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn ryu_pagetable_unmap_range(pt: *mut Radix48PageTable, va: u64, size: u64) {
    if !pt.is_null() {
        (*pt).unmap_range(va, size);
    }
}

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
