//! C-ABI Foreign Function Interface for ryu-kernel-sync.

use crate::arbiter::ArbiterRegistry;
use std::slice;

pub const RYU_KERNEL_SYNC_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_kernel_sync_version() -> u32 {
    RYU_KERNEL_SYNC_VERSION
}

/// # Safety
/// Always sound to call; release the returned handle with `ryu_arbiter_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_create() -> *mut ArbiterRegistry {
    Box::into_raw(Box::new(ArbiterRegistry::new()))
}

/// # Safety
/// `registry` must be null or a handle from `ryu_arbiter_create` that has not been
/// destroyed yet; it must not be used (by any thread) after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_destroy(registry: *mut ArbiterRegistry) {
    if !registry.is_null() {
        drop(Box::from_raw(registry));
    }
}

/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`.
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

/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`.
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

/// Returns 0 without waking anyone when `out_woken` is null or `max_woken` is 0.
///
/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`. `out_woken` must be
/// null or valid for writes of `max_woken` `u64`s.
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

/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`.
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

/// Returns 0 when `registry` is null (this function has no error channel).
///
/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`.
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

/// # Safety
/// `registry` must be null or a live handle from `ryu_arbiter_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_arbiter_clear(registry: *mut ArbiterRegistry) -> i32 {
    if registry.is_null() {
        return -1;
    }
    let reg = &*registry;
    reg.clear();
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_kernel_sync_version(), RYU_KERNEL_SYNC_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        unsafe {
            let mut woken = [0u64; 4];
            ryu_arbiter_destroy(ptr::null_mut());
            assert_eq!(ryu_arbiter_insert_waiter(ptr::null_mut(), 0x1000, 1, 0), -1);
            assert_eq!(ryu_arbiter_remove_waiter(ptr::null_mut(), 0x1000, 1), -1);
            assert_eq!(
                ryu_arbiter_wake_waiters(ptr::null_mut(), 0x1000, 1, woken.as_mut_ptr(), 4),
                -1
            );
            assert_eq!(ryu_arbiter_get_waiter_count(ptr::null(), 0x1000), -1);
            assert_eq!(ryu_arbiter_compute_modify_addend(ptr::null(), 0x1000, 1), 0);
            assert_eq!(ryu_arbiter_clear(ptr::null_mut()), -1);

            let reg = ryu_arbiter_create();
            assert_eq!(ryu_arbiter_insert_waiter(reg, 0x1000, 1, 0), 0);
            assert_eq!(
                ryu_arbiter_wake_waiters(reg, 0x1000, 1, ptr::null_mut(), 4),
                0
            );
            assert_eq!(ryu_arbiter_get_waiter_count(reg, 0x1000), 1);
            ryu_arbiter_destroy(reg);
        }
    }
}
