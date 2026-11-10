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

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_destroy(tracker: *mut MaxwellStateTracker) {
    if !tracker.is_null() {
        unsafe {
            drop(Box::from_raw(tracker));
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_map_register(
    tracker: *mut MaxwellStateTracker,
    reg_offset: u32,
    group_index: u8,
) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.map_register(reg_offset as usize, group_index);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_map_range(
    tracker: *mut MaxwellStateTracker,
    start_offset: u32,
    size_bytes: u32,
    group_index: u8,
) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.map_register_range(start_offset as usize, size_bytes as usize, group_index);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_set_dirty(
    tracker: *mut MaxwellStateTracker,
    reg_offset: u32,
) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.set_dirty(reg_offset as usize);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_set_dirty_batch(
    tracker: *mut MaxwellStateTracker,
    offsets: *const u32,
    count: usize,
) {
    if !tracker.is_null() && !offsets.is_null() && count > 0 {
        let t = unsafe { &mut *tracker };
        let s = unsafe { slice::from_raw_parts(offsets, count) };
        t.set_dirty_batch(s);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_force_dirty(
    tracker: *mut MaxwellStateTracker,
    group_index: u8,
) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.force_dirty(group_index);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_set_all_dirty(tracker: *mut MaxwellStateTracker) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.set_all_dirty();
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_get_dirty_mask(tracker: *const MaxwellStateTracker) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    let t = unsafe { &*tracker };
    t.get_dirty_mask()
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_scan_changes(
    tracker: *mut MaxwellStateTracker,
    current: *const u8,
    shadow: *const u8,
    len: usize,
) -> u64 {
    if tracker.is_null() || current.is_null() || shadow.is_null() || len == 0 {
        return 0;
    }
    let t = unsafe { &mut *tracker };
    let curr_slice = unsafe { slice::from_raw_parts(current, len) };
    let shad_slice = unsafe { slice::from_raw_parts(shadow, len) };
    t.scan_changes(curr_slice, shad_slice)
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_collect_dirty(
    tracker: *mut MaxwellStateTracker,
    check_mask: u64,
    out_groups: *mut u32,
    max_groups: usize,
) -> usize {
    if tracker.is_null() || out_groups.is_null() || max_groups == 0 {
        return 0;
    }
    let t = unsafe { &mut *tracker };
    let out_slice = unsafe { slice::from_raw_parts_mut(out_groups, max_groups) };
    t.collect_dirty_groups(check_mask, out_slice)
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_tracker_clear_dirty(tracker: *mut MaxwellStateTracker, mask: u64) {
    if !tracker.is_null() {
        let t = unsafe { &mut *tracker };
        t.clear_dirty(mask);
    }
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_pipeline_hash64(data: *const u8, len: usize) -> u64 {
    if data.is_null() || len == 0 {
        return 0;
    }
    let s = unsafe { slice::from_raw_parts(data, len) };
    pipeline_state_hash64(s)
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_pipeline_hash32(data: *const u8, len: usize) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }
    let s = unsafe { slice::from_raw_parts(data, len) };
    pipeline_state_hash32(s)
}

#[no_mangle]
pub extern "C" fn ryu_maxwell_pipeline_equals(a: *const u8, b: *const u8, len: usize) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    if len == 0 {
        return 1;
    }
    let sa = unsafe { slice::from_raw_parts(a, len) };
    let sb = unsafe { slice::from_raw_parts(b, len) };
    if pipeline_state_equals(sa, sb) {
        1
    } else {
        0
    }
}
