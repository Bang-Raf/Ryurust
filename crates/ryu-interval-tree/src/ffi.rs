use crate::tree::*;
use std::slice;

pub const RYU_INTERVAL_TREE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_interval_tree_version() -> u32 {
    RYU_INTERVAL_TREE_VERSION
}

/// Allocates an empty tree; free it with `ryu_interval_tree_destroy`.
#[no_mangle]
pub extern "C" fn ryu_interval_tree_create() -> *mut FlatIntervalTree {
    Box::into_raw(Box::new(FlatIntervalTree::new()))
}

/// Frees a tree. Null is ignored.
///
/// # Safety
/// `tree` must be null or a handle from `ryu_interval_tree_create` that has not
/// been destroyed yet; it must not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_destroy(tree: *mut FlatIntervalTree) {
    if !tree.is_null() {
        // SAFETY: non-null and, per the contract, a live `Box` from `_create`.
        drop(Box::from_raw(tree));
    }
}

/// Removes every interval. Null `tree` is a no-op.
///
/// # Safety
/// `tree` must be null or a live handle from `ryu_interval_tree_create`, not
/// used concurrently from another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_clear(tree: *mut FlatIntervalTree) {
    // SAFETY: `as_mut` handles null; otherwise the handle is live and unaliased.
    if let Some(t) = tree.as_mut() {
        t.clear();
    }
}

/// Inserts `[start, end)` with `value`. Null `tree` is a no-op.
///
/// # Safety
/// `tree` must be null or a live handle from `ryu_interval_tree_create`, not
/// used concurrently from another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_insert(
    tree: *mut FlatIntervalTree,
    start: u64,
    end: u64,
    value: u64,
) {
    // SAFETY: `as_mut` handles null; otherwise the handle is live and unaliased.
    if let Some(t) = tree.as_mut() {
        t.insert(start, end, value);
    }
}

/// Removes one copy of the exact `(start, end, value)` entry. Returns 1 if one
/// was removed, 0 if none matched, -1 if `tree` is null.
///
/// # Safety
/// `tree` must be null or a live handle from `ryu_interval_tree_create`, not
/// used concurrently from another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_remove(
    tree: *mut FlatIntervalTree,
    start: u64,
    end: u64,
    value: u64,
) -> i32 {
    // SAFETY: `as_mut` handles null; otherwise the handle is live and unaliased.
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

/// Writes up to `max_out` values of intervals overlapping `[start, end)` to
/// `out_buf` (ascending start order) and returns the total overlap count, which
/// may exceed `max_out`. Null or zero-sized `out_buf` only counts. Null `tree`
/// returns 0.
///
/// # Safety
/// `tree` must be null or a live handle from `ryu_interval_tree_create`. A
/// non-null `out_buf` must be valid for writes of `max_out` `u64`s and must not
/// overlap the tree.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_find_overlaps(
    tree: *const FlatIntervalTree,
    start: u64,
    end: u64,
    out_buf: *mut u64,
    max_out: usize,
) -> usize {
    // SAFETY: `as_ref` handles null; otherwise the handle is live.
    if let Some(t) = tree.as_ref() {
        if out_buf.is_null() || max_out == 0 {
            let mut tmp = Vec::with_capacity(32);
            t.find_overlaps(start, end, &mut tmp);
            return tmp.len();
        }

        // SAFETY: `out_buf` is non-null and `max_out > 0` here; the caller
        // guarantees it is valid for `max_out` writes and unaliased.
        let slice = slice::from_raw_parts_mut(out_buf, max_out);
        t.find_overlaps_slice(start, end, slice)
    } else {
        0
    }
}

/// Number of stored intervals; 0 for a null `tree`.
///
/// # Safety
/// `tree` must be null or a live handle from `ryu_interval_tree_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_interval_tree_len(tree: *const FlatIntervalTree) -> usize {
    // SAFETY: `as_ref` handles null; otherwise the handle is live.
    if let Some(t) = tree.as_ref() {
        t.len()
    } else {
        0
    }
}
