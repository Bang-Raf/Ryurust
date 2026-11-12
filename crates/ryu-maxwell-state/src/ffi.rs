use crate::hasher::{pipeline_state_equals, pipeline_state_hash32, pipeline_state_hash64};
use crate::tracker::MaxwellStateTracker;
use std::slice;

pub const RYU_MAXWELL_STATE_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_maxwell_state_version() -> u32 {
    RYU_MAXWELL_STATE_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_create() -> *mut MaxwellStateTracker {
    Box::into_raw(Box::new(MaxwellStateTracker::new()))
}

/// # Safety
/// `tracker` must be null or a handle from `ryu_maxwell_tracker_create` that has not
/// been destroyed yet; it must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_destroy(tracker: *mut MaxwellStateTracker) {
    if !tracker.is_null() {
        // SAFETY: non-null, and the caller guarantees it came from `_create` and is
        // not used again.
        unsafe {
            drop(Box::from_raw(tracker));
        }
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_map_register(
    tracker: *mut MaxwellStateTracker,
    reg_offset: u32,
    group_index: u8,
) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.map_register(reg_offset as usize, group_index);
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_map_range(
    tracker: *mut MaxwellStateTracker,
    start_offset: u32,
    size_bytes: u32,
    group_index: u8,
) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.map_register_range(start_offset as usize, size_bytes as usize, group_index);
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_set_dirty(
    tracker: *mut MaxwellStateTracker,
    reg_offset: u32,
) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.set_dirty(reg_offset as usize);
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread. `offsets` must be null or readable for
/// `count` elements.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_set_dirty_batch(
    tracker: *mut MaxwellStateTracker,
    offsets: *const u32,
    count: usize,
) {
    if !tracker.is_null() && !offsets.is_null() && count > 0 {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        // SAFETY: non-null was checked; the caller guarantees `count` readable offsets.
        let s = unsafe { slice::from_raw_parts(offsets, count) };
        t.set_dirty_batch(s);
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_force_dirty(
    tracker: *mut MaxwellStateTracker,
    group_index: u8,
) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.force_dirty(group_index);
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_set_all_dirty(tracker: *mut MaxwellStateTracker) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.set_all_dirty();
    }
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_get_dirty_mask(
    tracker: *const MaxwellStateTracker,
) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
    let t = unsafe { &*tracker };
    t.get_dirty_mask()
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread. `current` and `shadow` must each be null
/// or readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_scan_changes(
    tracker: *mut MaxwellStateTracker,
    current: *const u8,
    shadow: *const u8,
    len: usize,
) -> u64 {
    if tracker.is_null() || current.is_null() || shadow.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
    let t = unsafe { &mut *tracker };
    // SAFETY: both non-null; the caller guarantees `len` readable bytes each.
    let curr_slice = unsafe { slice::from_raw_parts(current, len) };
    // SAFETY: as above.
    let shad_slice = unsafe { slice::from_raw_parts(shadow, len) };
    t.scan_changes(curr_slice, shad_slice)
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread. `out_groups` must be null or writable
/// for `max_groups` elements.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_collect_dirty(
    tracker: *mut MaxwellStateTracker,
    check_mask: u64,
    out_groups: *mut u32,
    max_groups: usize,
) -> usize {
    if tracker.is_null() || out_groups.is_null() || max_groups == 0 {
        return 0;
    }
    // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
    let t = unsafe { &mut *tracker };
    // SAFETY: non-null was checked; the caller guarantees `max_groups` writable entries.
    let out_slice = unsafe { slice::from_raw_parts_mut(out_groups, max_groups) };
    t.collect_dirty_groups(check_mask, out_slice)
}

/// # Safety
/// `tracker` must be null or a live handle from `ryu_maxwell_tracker_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_tracker_clear_dirty(
    tracker: *mut MaxwellStateTracker,
    mask: u64,
) {
    if !tracker.is_null() {
        // SAFETY: non-null was checked; the caller guarantees a live, unaliased tracker.
        let t = unsafe { &mut *tracker };
        t.clear_dirty(mask);
    }
}

/// Returns 0 for a null or empty buffer (there is no separate error channel).
///
/// # Safety
/// `data` must be null or readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_pipeline_hash64(data: *const u8, len: usize) -> u64 {
    if data.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: non-null was checked; the caller guarantees `len` readable bytes.
    let s = unsafe { slice::from_raw_parts(data, len) };
    pipeline_state_hash64(s)
}

/// Returns 0 for a null or empty buffer (there is no separate error channel).
///
/// # Safety
/// `data` must be null or readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_pipeline_hash32(data: *const u8, len: usize) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: non-null was checked; the caller guarantees `len` readable bytes.
    let s = unsafe { slice::from_raw_parts(data, len) };
    pipeline_state_hash32(s)
}

/// # Safety
/// `a` and `b` must each be null or readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_maxwell_pipeline_equals(
    a: *const u8,
    b: *const u8,
    len: usize,
) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    if len == 0 {
        return 1;
    }
    // SAFETY: both non-null; the caller guarantees `len` readable bytes each.
    let sa = unsafe { slice::from_raw_parts(a, len) };
    // SAFETY: as above.
    let sb = unsafe { slice::from_raw_parts(b, len) };
    if pipeline_state_equals(sa, sb) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_maxwell_state_version(), RYU_MAXWELL_STATE_VERSION);
        assert_eq!(RYU_MAXWELL_STATE_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let bytes = [0u8; 64];
        let offsets = [0u32; 2];
        let mut groups = [0u32; 4];
        // SAFETY: null handles/buffers are rejected before use; every non-null
        // pointer refers to a live local (or the live tracker) of the stated length.
        unsafe {
            let null = ptr::null_mut::<MaxwellStateTracker>();
            ryu_maxwell_tracker_destroy(null);
            ryu_maxwell_tracker_map_register(null, 0x10, 1);
            ryu_maxwell_tracker_map_range(null, 0x10, 8, 1);
            ryu_maxwell_tracker_set_dirty(null, 0x10);
            ryu_maxwell_tracker_set_dirty_batch(null, offsets.as_ptr(), 2);
            ryu_maxwell_tracker_force_dirty(null, 1);
            ryu_maxwell_tracker_set_all_dirty(null);
            ryu_maxwell_tracker_clear_dirty(null, u64::MAX);
            assert_eq!(ryu_maxwell_tracker_get_dirty_mask(null), 0);
            assert_eq!(
                ryu_maxwell_tracker_scan_changes(null, bytes.as_ptr(), bytes.as_ptr(), 64),
                0
            );
            assert_eq!(
                ryu_maxwell_tracker_collect_dirty(null, u64::MAX, groups.as_mut_ptr(), 4),
                0
            );

            let t = ryu_maxwell_tracker_create();
            ryu_maxwell_tracker_map_register(t, 0x10, 1);
            ryu_maxwell_tracker_set_dirty_batch(t, ptr::null(), 2);
            assert_eq!(ryu_maxwell_tracker_get_dirty_mask(t), 0);
            assert_eq!(
                ryu_maxwell_tracker_scan_changes(t, ptr::null(), bytes.as_ptr(), 64),
                0
            );
            assert_eq!(
                ryu_maxwell_tracker_scan_changes(t, bytes.as_ptr(), ptr::null(), 64),
                0
            );
            ryu_maxwell_tracker_force_dirty(t, 1);
            assert_eq!(
                ryu_maxwell_tracker_collect_dirty(t, u64::MAX, ptr::null_mut(), 4),
                0
            );
            assert_eq!(ryu_maxwell_tracker_get_dirty_mask(t), 1 << 1);
            ryu_maxwell_tracker_destroy(t);

            assert_eq!(ryu_maxwell_pipeline_hash64(ptr::null(), 64), 0);
            assert_eq!(ryu_maxwell_pipeline_hash32(ptr::null(), 64), 0);
            assert_eq!(
                ryu_maxwell_pipeline_equals(ptr::null(), bytes.as_ptr(), 64),
                0
            );
            assert_eq!(
                ryu_maxwell_pipeline_equals(bytes.as_ptr(), ptr::null(), 64),
                0
            );
            assert_eq!(ryu_maxwell_pipeline_equals(ptr::null(), ptr::null(), 0), 0);
        }
    }
}
