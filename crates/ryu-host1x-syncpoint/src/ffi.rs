//! C-ABI Foreign Function Interface for ryu-host1x-syncpoint.

use crate::sequencer::SyncptSequencer;
use std::slice;

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_create() -> *mut SyncptSequencer {
    Box::into_raw(Box::new(SyncptSequencer::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_destroy(handle: *mut SyncptSequencer) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_increment(handle: *mut SyncptSequencer, syncpt_id: u32) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.increment(syncpt_id);
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_increment_when_done(
    handle: *mut SyncptSequencer,
    class_id: u32,
    syncpt_id: u32,
    out_handle: *mut u32,
) -> i32 {
    if handle.is_null() || out_handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    let h = seq.increment_when_done(class_id, syncpt_id);
    *out_handle = h;
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_signal_done(
    handle: *mut SyncptSequencer,
    op_handle: u32,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.signal_done(op_handle);
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_drain_flushed(
    handle: *mut SyncptSequencer,
    dst: *mut u32,
    capacity: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    if dst.is_null() || capacity == 0 {
        return 0;
    }
    let seq = &*handle;
    let dst_slice = slice::from_raw_parts_mut(dst, capacity);
    seq.drain_flushed(dst_slice) as i32
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_reset(handle: *mut SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.reset();
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_pending_count(handle: *const SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.pending_count() as i32
}

#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_flushed_count(handle: *const SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.flushed_count() as i32
}
