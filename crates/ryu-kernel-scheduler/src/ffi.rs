use crate::scheduler::RyuScheduler;
use std::slice;

pub const RYU_KERNEL_SCHEDULER_VERSION: u32 = 0x00010000;

/// Kept for compatibility with existing bindings; same value as
/// `ryu_kernel_scheduler_version`.
#[no_mangle]
pub extern "C" fn ryu_scheduler_version() -> u32 {
    RYU_KERNEL_SCHEDULER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_kernel_scheduler_version() -> u32 {
    RYU_KERNEL_SCHEDULER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_create() -> *mut RyuScheduler {
    Box::into_raw(Box::new(RyuScheduler::new()))
}

/// # Safety
/// `sched` must be null or a handle from `ryu_scheduler_create` that has not been
/// destroyed yet; it must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_destroy(sched: *mut RyuScheduler) {
    if !sched.is_null() {
        // SAFETY: a non-null `sched` came from `Box::into_raw` in `ryu_scheduler_create`
        // and is not used again (caller contract).
        unsafe {
            drop(Box::from_raw(sched));
        }
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_reset(sched: *mut RyuScheduler) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.reset();
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_schedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.schedule(core as usize, prio as usize, thread_id);
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_schedule_prepend(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.schedule_prepend(core as usize, prio as usize, thread_id);
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_unschedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.unschedule(core as usize, prio as usize, thread_id);
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_reschedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) -> u64 {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.reschedule(core as usize, prio as usize, thread_id)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_suggest(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.suggest(core as usize, prio as usize, thread_id);
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_unsuggest(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.unsuggest(core as usize, prio as usize, thread_id);
    }
}

/// # Safety
/// `sched` must be null (no-op) or a live handle from `ryu_scheduler_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_transfer_to_core(
    sched: *mut RyuScheduler,
    prio: u32,
    src_core: i32,
    dst_core: i32,
    thread_id: u64,
) {
    // SAFETY: `sched` is null or a live, exclusively accessed handle (caller contract).
    if let Some(s) = unsafe { sched.as_mut() } {
        s.transfer_to_core(prio as usize, src_core, dst_core, thread_id);
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_first_scheduled(
    sched: *const RyuScheduler,
    core: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_first(core as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_first_suggested(
    sched: *const RyuScheduler,
    core: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.suggested_first(core as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_element_at_scheduled(
    sched: *const RyuScheduler,
    core: u32,
    index: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_element_at(core as usize, index as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_dynamic_prio_first(
    sched: *const RyuScheduler,
    core: u32,
    prio: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_dynamic_prio_first(core as usize, prio as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_has_scheduled(sched: *const RyuScheduler, core: u32) -> i32 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        if s.has_scheduled(core as usize) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_has_suggested(sched: *const RyuScheduler, core: u32) -> i32 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        if s.has_suggested(core as usize) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_get_scheduled_mask(
    sched: *const RyuScheduler,
    core: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.get_scheduled_mask(core as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null (returns 0) or a live handle from `ryu_scheduler_create`, with
/// no concurrent mutating call on it.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_get_suggested_mask(
    sched: *const RyuScheduler,
    core: u32,
) -> u64 {
    // SAFETY: `sched` is null or a live handle that is not being mutated (caller contract).
    if let Some(s) = unsafe { sched.as_ref() } {
        s.get_suggested_mask(core as usize)
    } else {
        0
    }
}

/// # Safety
/// `sched` must be null or a live handle from `ryu_scheduler_create` with no concurrent
/// mutating call; `out_buf` must be null or valid for writes of `max_count` `u64`s.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_export_scheduled(
    sched: *const RyuScheduler,
    core: u32,
    out_buf: *mut u64,
    max_count: u32,
) -> u32 {
    if sched.is_null() || out_buf.is_null() || max_count == 0 {
        return 0;
    }
    // SAFETY: `sched` was checked non-null above and is a live handle (caller contract).
    let s = unsafe { &*sched };
    // SAFETY: `out_buf` is non-null and valid for `max_count` writes (caller contract).
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, max_count as usize) };
    s.export_scheduled(core as usize, buf) as u32
}

/// # Safety
/// `sched` must be null or a live handle from `ryu_scheduler_create` with no concurrent
/// mutating call; `out_buf` must be null or valid for writes of `max_count` `u64`s.
#[no_mangle]
pub unsafe extern "C" fn ryu_scheduler_export_suggested(
    sched: *const RyuScheduler,
    core: u32,
    out_buf: *mut u64,
    max_count: u32,
) -> u32 {
    if sched.is_null() || out_buf.is_null() || max_count == 0 {
        return 0;
    }
    // SAFETY: `sched` was checked non-null above and is a live handle (caller contract).
    let s = unsafe { &*sched };
    // SAFETY: `out_buf` is non-null and valid for `max_count` writes (caller contract).
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, max_count as usize) };
    s.export_suggested(core as usize, buf) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_kernel_scheduler_version(), RYU_KERNEL_SCHEDULER_VERSION);
        assert_eq!(ryu_scheduler_version(), RYU_KERNEL_SCHEDULER_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let null_mut = ptr::null_mut::<RyuScheduler>();
        let null = ptr::null::<RyuScheduler>();
        let mut buf = [0u64; 4];
        unsafe {
            ryu_scheduler_destroy(null_mut);
            ryu_scheduler_reset(null_mut);
            ryu_scheduler_schedule(null_mut, 1, 0, 7);
            ryu_scheduler_schedule_prepend(null_mut, 1, 0, 7);
            ryu_scheduler_unschedule(null_mut, 1, 0, 7);
            assert_eq!(ryu_scheduler_reschedule(null_mut, 1, 0, 7), 0);
            ryu_scheduler_suggest(null_mut, 1, 0, 7);
            ryu_scheduler_unsuggest(null_mut, 1, 0, 7);
            ryu_scheduler_transfer_to_core(null_mut, 1, 0, 1, 7);
            assert_eq!(ryu_scheduler_first_scheduled(null, 0), 0);
            assert_eq!(ryu_scheduler_first_suggested(null, 0), 0);
            assert_eq!(ryu_scheduler_element_at_scheduled(null, 0, 0), 0);
            assert_eq!(ryu_scheduler_dynamic_prio_first(null, 0, 1), 0);
            assert_eq!(ryu_scheduler_has_scheduled(null, 0), 0);
            assert_eq!(ryu_scheduler_has_suggested(null, 0), 0);
            assert_eq!(ryu_scheduler_get_scheduled_mask(null, 0), 0);
            assert_eq!(ryu_scheduler_get_suggested_mask(null, 0), 0);
            assert_eq!(
                ryu_scheduler_export_scheduled(null, 0, buf.as_mut_ptr(), 4),
                0
            );
            assert_eq!(
                ryu_scheduler_export_suggested(null, 0, buf.as_mut_ptr(), 4),
                0
            );

            let s = ryu_scheduler_create();
            ryu_scheduler_schedule(s, 1, 0, 7);
            ryu_scheduler_suggest(s, 1, 1, 7);
            assert_eq!(ryu_scheduler_export_scheduled(s, 0, ptr::null_mut(), 4), 0);
            assert_eq!(ryu_scheduler_export_suggested(s, 1, ptr::null_mut(), 4), 0);
            ryu_scheduler_destroy(s);
        }
    }
}
