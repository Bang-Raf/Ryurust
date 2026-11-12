//! C-ABI Foreign Function Interface for ryu-gal-queue.

use crate::command_queue::GalSpscQueue;
use crate::span_pool::GalSpanPool;
use std::slice;

pub const RYU_GAL_QUEUE_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_gal_queue_version() -> u32 {
    RYU_GAL_QUEUE_VERSION
}

// ----------------------------------------------------------------------------
// GalSpanPool FFI
// ----------------------------------------------------------------------------

/// # Safety
/// No pointer preconditions. The returned handle must be released exactly once with
/// `ryu_span_pool_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_create(size: usize) -> *mut GalSpanPool {
    if size == 0 {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(GalSpanPool::new(size)))
}

/// # Safety
/// `pool` must be null or a handle from `ryu_span_pool_create` that has not been
/// destroyed yet; it and pointers obtained from it must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_destroy(pool: *mut GalSpanPool) {
    if !pool.is_null() {
        drop(Box::from_raw(pool));
    }
}

/// # Safety
/// `pool` must be null or a live handle from `ryu_span_pool_create`. `data` must be readable for
/// `size` bytes. Only one thread may act as producer.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_insert(
    pool: *mut GalSpanPool,
    data: *const u8,
    size: usize,
) -> isize {
    if pool.is_null() {
        return -1;
    }
    if size == 0 {
        return 0;
    }
    if data.is_null() {
        return -1;
    }
    let p = &*pool;
    let data_slice = slice::from_raw_parts(data, size);
    p.try_insert(data_slice)
}

/// # Safety
/// `pool` must be null or a live handle from `ryu_span_pool_create`. Only one thread may act as
/// consumer; the returned pointer is valid for `size` bytes until the pool is destroyed.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_get(pool: *mut GalSpanPool, size: usize) -> *const u8 {
    if pool.is_null() {
        return std::ptr::null();
    }
    let p = &*pool;
    p.get_consumer_ptr(size)
}

/// # Safety
/// `pool` must be null or a live handle from `ryu_span_pool_create`. `size` must be the size of the
/// span last returned by `ryu_span_pool_get`; consumer thread only.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_advance(pool: *mut GalSpanPool, size: usize) -> i32 {
    if pool.is_null() {
        return -1;
    }
    let p = &*pool;
    p.advance_consumer(size);
    0
}

/// # Safety
/// `pool` must be null or a live handle from `ryu_span_pool_create`. No producer or consumer may be
/// using the pool concurrently.
#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_reset(pool: *mut GalSpanPool) -> i32 {
    if pool.is_null() {
        return -1;
    }
    let p = &*pool;
    p.reset();
    0
}

// ----------------------------------------------------------------------------
// GalSpscQueue FFI
// ----------------------------------------------------------------------------

/// Returns null if either argument is 0 or `capacity * element_size` overflows.
///
/// # Safety
/// No pointer preconditions. The returned handle must be released exactly once with
/// `ryu_spsc_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_create(
    capacity: usize,
    element_size: usize,
) -> *mut GalSpscQueue {
    if capacity == 0 || element_size == 0 {
        return std::ptr::null_mut();
    }
    // An overflowing byte size would allocate a buffer smaller than the slots
    // handed out by `get_slot_ptr`.
    if capacity.checked_mul(element_size).is_none() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(GalSpscQueue::new(capacity, element_size)))
}

/// # Safety
/// `queue` must be null or a handle from `ryu_spsc_create` that has not been destroyed
/// yet; it and slot pointers obtained from it must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_destroy(queue: *mut GalSpscQueue) {
    if !queue.is_null() {
        drop(Box::from_raw(queue));
    }
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`. Producer thread only.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_try_reserve(queue: *mut GalSpscQueue) -> isize {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.try_reserve()
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`. Producer thread only,
/// after a successful `ryu_spsc_try_reserve`.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_commit(queue: *mut GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.commit();
    0
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`. Consumer thread only.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_try_consume(queue: *mut GalSpscQueue) -> isize {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.try_consume()
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`. Consumer thread only,
/// after a successful `ryu_spsc_try_consume`.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_release(queue: *mut GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.release();
    0
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`. The returned pointer is
/// valid for `element_size` bytes and may only be accessed by the side that currently
/// owns `slot` (reserved or consumed).
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_get_slot_ptr(queue: *mut GalSpscQueue, slot: usize) -> *mut u8 {
    if queue.is_null() {
        return std::ptr::null_mut();
    }
    let q = &*queue;
    q.get_slot_ptr(slot)
}

/// # Safety
/// `queue` must be null or a live handle from `ryu_spsc_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_pending_count(queue: *const GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.pending_count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_gal_queue_version(), RYU_GAL_QUEUE_VERSION);
        assert_eq!(RYU_GAL_QUEUE_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let data = [1u8; 8];
        // SAFETY: null handles are rejected before use; `pool` is live until its
        // destroy and `data` is a live local of the stated length.
        unsafe {
            ryu_span_pool_destroy(ptr::null_mut());
            assert_eq!(ryu_span_pool_insert(ptr::null_mut(), data.as_ptr(), 8), -1);
            assert_eq!(ryu_span_pool_insert(ptr::null_mut(), ptr::null(), 0), -1);
            assert!(ryu_span_pool_get(ptr::null_mut(), 8).is_null());
            assert_eq!(ryu_span_pool_advance(ptr::null_mut(), 8), -1);
            assert_eq!(ryu_span_pool_reset(ptr::null_mut()), -1);

            let pool = ryu_span_pool_create(64);
            assert!(!pool.is_null());
            assert_eq!(ryu_span_pool_insert(pool, ptr::null(), 8), -1);
            // Zero-length inserts are accepted before the data pointer is looked at.
            assert_eq!(ryu_span_pool_insert(pool, ptr::null(), 0), 0);
            ryu_span_pool_destroy(pool);

            ryu_spsc_destroy(ptr::null_mut());
            assert_eq!(ryu_spsc_try_reserve(ptr::null_mut()), -1);
            assert_eq!(ryu_spsc_commit(ptr::null_mut()), -1);
            assert_eq!(ryu_spsc_try_consume(ptr::null_mut()), -1);
            assert_eq!(ryu_spsc_release(ptr::null_mut()), -1);
            assert!(ryu_spsc_get_slot_ptr(ptr::null_mut(), 0).is_null());
            assert_eq!(ryu_spsc_pending_count(ptr::null()), -1);
        }
    }

    #[test]
    fn spsc_create_rejects_overflowing_size() {
        // SAFETY: no pointers are passed.
        unsafe {
            assert!(ryu_spsc_create(usize::MAX, 2).is_null());
        }
    }

    #[test]
    fn span_pool_get_rejects_overflowing_size() {
        // SAFETY: `pool` is live until its destroy; `data` is a live local.
        unsafe {
            let pool = ryu_span_pool_create(64);
            let data = [1u8; 8];
            assert_eq!(ryu_span_pool_insert(pool, data.as_ptr(), 8), 0);
            assert_eq!(ryu_span_pool_advance(pool, 8), 0);
            assert!(ryu_span_pool_get(pool, usize::MAX).is_null());
            ryu_span_pool_destroy(pool);
        }
    }
}
