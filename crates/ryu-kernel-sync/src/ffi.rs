//! C-ABI Foreign Function Interface for ryu-kernel-sync.

use crate::arbiter::ArbiterRegistry;
use std::slice;

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_create() -> *mut ArbiterRegistry {
    Box::into_raw(Box::new(ArbiterRegistry::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_destroy(registry: *mut ArbiterRegistry) {
    if !registry.is_null() {
        drop(Box::from_raw(registry));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_insert_waiter(
    registry: *mut ArbiterRegistry,
    address: u64,
    thread_id: u64,
    priority: i32,
) -> i32 {
    if registry.is_null() {
        return -1;
    }
    let reg = &*registry;
    reg.insert_waiter(address, thread_id, priority);
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_remove_waiter(
    registry: *mut ArbiterRegistry,
    address: u64,
    thread_id: u64,
) -> i32 {
    if registry.is_null() {
        return -1;
    }
    let reg = &*registry;
    if reg.remove_waiter(address, thread_id) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_wake_waiters(
    registry: *mut ArbiterRegistry,
    address: u64,
    count: i32,
    out_woken: *mut u64,
    max_woken: usize,
) -> i32 {
    if registry.is_null() {
        return -1;
    }
    if out_woken.is_null() || max_woken == 0 {
        return 0;
    }
    let reg = &*registry;
    let out_slice = slice::from_raw_parts_mut(out_woken, max_woken);
    reg.wake_waiters(address, count, out_slice) as i32
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_get_waiter_count(
    registry: *const ArbiterRegistry,
    address: u64,
) -> i32 {
    if registry.is_null() {
        return -1;
    }
    let reg = &*registry;
    reg.get_waiter_count(address) as i32
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_compute_modify_addend(
    registry: *const ArbiterRegistry,
    address: u64,
    count: i32,
) -> i32 {
    if registry.is_null() {
        return 0;
    }
    let reg = &*registry;
    reg.compute_signal_modify_addend(address, count)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_clear(registry: *mut ArbiterRegistry) -> i32 {
    if registry.is_null() {
        return -1;
    }
    let reg = &*registry;
    reg.clear();
    0
}
