//! C-ABI Foreign Function Interface for ryu-gal-queue.

use crate::command_queue::GalSpscQueue;
use crate::span_pool::GalSpanPool;
use std::slice;

// ----------------------------------------------------------------------------
// GalSpanPool FFI
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_create(size: usize) -> *mut GalSpanPool {
    if size == 0 {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(GalSpanPool::new(size)))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_destroy(pool: *mut GalSpanPool) {
    if !pool.is_null() {
        drop(Box::from_raw(pool));
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_get(pool: *mut GalSpanPool, size: usize) -> *const u8 {
    if pool.is_null() {
        return std::ptr::null();
    }
    let p = &*pool;
    p.get_consumer_ptr(size)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_span_pool_advance(pool: *mut GalSpanPool, size: usize) -> i32 {
    if pool.is_null() {
        return -1;
    }
    let p = &*pool;
    p.advance_consumer(size);
    0
}

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

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_create(
    capacity: usize,
    element_size: usize,
) -> *mut GalSpscQueue {
    if capacity == 0 || element_size == 0 {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(GalSpscQueue::new(capacity, element_size)))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_destroy(queue: *mut GalSpscQueue) {
    if !queue.is_null() {
        drop(Box::from_raw(queue));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_try_reserve(queue: *mut GalSpscQueue) -> isize {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.try_reserve()
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_commit(queue: *mut GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.commit();
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_try_consume(queue: *mut GalSpscQueue) -> isize {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.try_consume()
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_release(queue: *mut GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.release();
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_get_slot_ptr(queue: *mut GalSpscQueue, slot: usize) -> *mut u8 {
    if queue.is_null() {
        return std::ptr::null_mut();
    }
    let q = &*queue;
    q.get_slot_ptr(slot)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_spsc_pending_count(queue: *const GalSpscQueue) -> i32 {
    if queue.is_null() {
        return -1;
    }
    let q = &*queue;
    q.pending_count() as i32
}
