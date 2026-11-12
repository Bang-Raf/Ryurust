//! C-ABI Foreign Function Interface for ryu-host1x-syncpoint.

use crate::sequencer::SyncptSequencer;
use std::slice;

pub const RYU_HOST1X_SYNCPOINT_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_host1x_syncpoint_version() -> u32 {
    RYU_HOST1X_SYNCPOINT_VERSION
}

/// # Safety
/// No pointer preconditions. The returned handle must be released exactly once with
/// `ryu_syncpt_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_create() -> *mut SyncptSequencer {
    Box::into_raw(Box::new(SyncptSequencer::new()))
}

/// # Safety
/// `handle` must be null or a handle from `ryu_syncpt_create` that has not been
/// destroyed yet; it must not be used afterwards or concurrently with this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_destroy(handle: *mut SyncptSequencer) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_increment(handle: *mut SyncptSequencer, syncpt_id: u32) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.increment(syncpt_id);
    0
}

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`. `out_handle` must be
/// null or writable for one `u32`.
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

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`.
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

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`. `dst` must be null or
/// writable for `capacity` elements.
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

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_reset(handle: *mut SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.reset();
    0
}

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_pending_count(handle: *const SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.pending_count() as i32
}

/// # Safety
/// `handle` must be null or a live handle from `ryu_syncpt_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_syncpt_flushed_count(handle: *const SyncptSequencer) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let seq = &*handle;
    seq.flushed_count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_host1x_syncpoint_version(), RYU_HOST1X_SYNCPOINT_VERSION);
        assert_eq!(RYU_HOST1X_SYNCPOINT_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let mut out = 0u32;
        let mut dst = [0u32; 4];
        // SAFETY: null handles are rejected before use; `seq` is live until its
        // destroy and the other pointers refer to live locals.
        unsafe {
            ryu_syncpt_destroy(ptr::null_mut());
            assert_eq!(ryu_syncpt_increment(ptr::null_mut(), 1), -1);
            assert_eq!(
                ryu_syncpt_increment_when_done(ptr::null_mut(), 0, 1, &mut out),
                -1
            );
            assert_eq!(ryu_syncpt_signal_done(ptr::null_mut(), 0), -1);
            assert_eq!(
                ryu_syncpt_drain_flushed(ptr::null_mut(), dst.as_mut_ptr(), 4),
                -1
            );
            assert_eq!(ryu_syncpt_reset(ptr::null_mut()), -1);
            assert_eq!(ryu_syncpt_pending_count(ptr::null()), -1);
            assert_eq!(ryu_syncpt_flushed_count(ptr::null()), -1);

            let seq = ryu_syncpt_create();
            assert!(!seq.is_null());
            assert_eq!(
                ryu_syncpt_increment_when_done(seq, 0, 1, ptr::null_mut()),
                -1
            );
            assert_eq!(ryu_syncpt_pending_count(seq), 0);
            assert_eq!(ryu_syncpt_increment(seq, 1), 0);
            // A null destination drains nothing and leaves the entry queued.
            assert_eq!(ryu_syncpt_drain_flushed(seq, ptr::null_mut(), 4), 0);
            assert_eq!(ryu_syncpt_flushed_count(seq), 1);
            ryu_syncpt_destroy(seq);
        }
    }
}
