use crate::tree::*;
use std::slice;

pub const RYU_INTERVAL_TREE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_interval_tree_version() -> u32 {
    RYU_INTERVAL_TREE_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_interval_tree_create() -> *mut FlatIntervalTree {
    Box::into_raw(Box::new(FlatIntervalTree::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_destroy(tree: *mut FlatIntervalTree) {
    if !tree.is_null() {
        drop(Box::from_raw(tree));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_clear(tree: *mut FlatIntervalTree) {
    if let Some(t) = tree.as_mut() {
        t.clear();
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_insert(
    tree: *mut FlatIntervalTree,
    start: u64,
    end: u64,
    value: u64,
) {
    if let Some(t) = tree.as_mut() {
        t.insert(start, end, value);
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_remove(
    tree: *mut FlatIntervalTree,
    start: u64,
    end: u64,
    value: u64,
) -> i32 {
    if let Some(t) = tree.as_mut() {
        if t.remove(start, end, value) {
            1
        } else {
            0
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_find_overlaps(
    tree: *const FlatIntervalTree,
    start: u64,
    end: u64,
    out_buf: *mut u64,
    max_out: usize,
) -> usize {
    if let Some(t) = tree.as_ref() {
        if out_buf.is_null() || max_out == 0 {
            let mut tmp = Vec::with_capacity(32);
            t.find_overlaps(start, end, &mut tmp);
            return tmp.len();
        }

        let slice = slice::from_raw_parts_mut(out_buf, max_out);
        t.find_overlaps_slice(start, end, slice)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_len(tree: *const FlatIntervalTree) -> usize {
    if let Some(t) = tree.as_ref() {
        t.len()
    } else {
        0
    }
}
