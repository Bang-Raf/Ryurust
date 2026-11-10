use crate::scheduler::RyuScheduler;
use std::slice;

pub const RYU_KERNEL_SCHEDULER_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_scheduler_version() -> u32 {
    RYU_KERNEL_SCHEDULER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_create() -> *mut RyuScheduler {
    Box::into_raw(Box::new(RyuScheduler::new()))
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_destroy(sched: *mut RyuScheduler) {
    if !sched.is_null() {
        unsafe {
            drop(Box::from_raw(sched));
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_reset(sched: *mut RyuScheduler) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.reset();
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_schedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.schedule(core as usize, prio as usize, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_schedule_prepend(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.schedule_prepend(core as usize, prio as usize, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_unschedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.unschedule(core as usize, prio as usize, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_reschedule(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) -> u64 {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.reschedule(core as usize, prio as usize, thread_id)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_suggest(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.suggest(core as usize, prio as usize, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_unsuggest(
    sched: *mut RyuScheduler,
    prio: u32,
    core: u32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.unsuggest(core as usize, prio as usize, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_transfer_to_core(
    sched: *mut RyuScheduler,
    prio: u32,
    src_core: i32,
    dst_core: i32,
    thread_id: u64,
) {
    if let Some(s) = unsafe { sched.as_mut() } {
        s.transfer_to_core(prio as usize, src_core, dst_core, thread_id);
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_first_scheduled(sched: *const RyuScheduler, core: u32) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_first(core as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_first_suggested(sched: *const RyuScheduler, core: u32) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.suggested_first(core as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_element_at_scheduled(
    sched: *const RyuScheduler,
    core: u32,
    index: u32,
) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_element_at(core as usize, index as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_dynamic_prio_first(
    sched: *const RyuScheduler,
    core: u32,
    prio: u32,
) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.scheduled_dynamic_prio_first(core as usize, prio as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_has_scheduled(sched: *const RyuScheduler, core: u32) -> i32 {
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

#[no_mangle]
pub extern "C" fn ryu_scheduler_has_suggested(sched: *const RyuScheduler, core: u32) -> i32 {
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

#[no_mangle]
pub extern "C" fn ryu_scheduler_get_scheduled_mask(sched: *const RyuScheduler, core: u32) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.get_scheduled_mask(core as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_get_suggested_mask(sched: *const RyuScheduler, core: u32) -> u64 {
    if let Some(s) = unsafe { sched.as_ref() } {
        s.get_suggested_mask(core as usize)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_export_scheduled(
    sched: *const RyuScheduler,
    core: u32,
    out_buf: *mut u64,
    max_count: u32,
) -> u32 {
    if sched.is_null() || out_buf.is_null() || max_count == 0 {
        return 0;
    }
    let s = unsafe { &*sched };
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, max_count as usize) };
    s.export_scheduled(core as usize, buf) as u32
}

#[no_mangle]
pub extern "C" fn ryu_scheduler_export_suggested(
    sched: *const RyuScheduler,
    core: u32,
    out_buf: *mut u64,
    max_count: u32,
) -> u32 {
    if sched.is_null() || out_buf.is_null() || max_count == 0 {
        return 0;
    }
    let s = unsafe { &*sched };
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, max_count as usize) };
    s.export_suggested(core as usize, buf) as u32
}
