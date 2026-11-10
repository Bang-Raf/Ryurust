use crate::allocator::{bitset_and_not, bitset_or_avx2, LinearScanEngine};
use crate::types::{Interval, RegisterMasksNative};
use std::slice;

pub const RYU_JIT_REGALLOC_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_jit_regalloc_version() -> u32 {
    RYU_JIT_REGALLOC_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_create() -> *mut LinearScanEngine {
    Box::into_raw(Box::new(LinearScanEngine::new()))
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_destroy(ctx: *mut LinearScanEngine) {
    if !ctx.is_null() {
        unsafe {
            drop(Box::from_raw(ctx));
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_reset(ctx: *mut LinearScanEngine) {
    if !ctx.is_null() {
        let engine = unsafe { &mut *ctx };
        engine.reset();
    }
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_add_interval(
    ctx: *mut LinearScanEngine,
    id: u32,
    start: i32,
    end: i32,
    reg_type: u8,
    is_fixed: u8,
    fixed_reg: i16,
) -> u32 {
    if ctx.is_null() {
        return 0;
    }
    let engine = unsafe { &mut *ctx };
    engine.add_interval(Interval {
        id,
        start,
        end,
        reg_type,
        is_fixed: is_fixed != 0,
        fixed_reg,
        assigned_reg: if is_fixed != 0 { fixed_reg } else { -1 },
        spill_offset: -1,
        first_use: start,
        next_use: end,
        weight: 1.0,
    });
    engine.intervals.len() as u32
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_run_pass(
    ctx: *mut LinearScanEngine,
    int_avail: u32,
    vec_avail: u32,
    reg_count: u32,
) -> usize {
    if ctx.is_null() {
        return 0;
    }
    let engine = unsafe { &mut *ctx };
    let masks = RegisterMasksNative {
        int_available: int_avail,
        vec_available: vec_avail,
        registers_count: reg_count,
    };
    engine.run_pass(masks)
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_get_result(
    ctx: *const LinearScanEngine,
    index: usize,
    out_id: *mut u32,
    out_reg: *mut i16,
    out_spill: *mut i32,
) -> i32 {
    if ctx.is_null() {
        return 0;
    }
    let engine = unsafe { &*ctx };
    if index < engine.intervals.len() {
        let iv = &engine.intervals[index];
        unsafe {
            if !out_id.is_null() {
                *out_id = iv.id;
            }
            if !out_reg.is_null() {
                *out_reg = iv.assigned_reg;
            }
            if !out_spill.is_null() {
                *out_spill = iv.spill_offset;
            }
        }
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_get_highest_index(span: *const i32, len: usize) -> i32 {
    if span.is_null() || len == 0 {
        return 0;
    }
    let slice = unsafe { slice::from_raw_parts(span, len) };
    LinearScanEngine::get_highest_value_index(slice) as i32
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_bitset_or(dst: *mut u64, src: *const u64, count: usize) -> u32 {
    if !dst.is_null() && !src.is_null() && count > 0 {
        let d = unsafe { slice::from_raw_parts_mut(dst, count) };
        let s = unsafe { slice::from_raw_parts(src, count) };
        if bitset_or_avx2(d, s) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_bitset_and_not(dst: *mut u64, src: *const u64, count: usize) -> u32 {
    if !dst.is_null() && !src.is_null() && count > 0 {
        let d = unsafe { slice::from_raw_parts_mut(dst, count) };
        let s = unsafe { slice::from_raw_parts(src, count) };
        if bitset_and_not(d, s) {
            1
        } else {
            0
        }
    } else {
        0
    }
}
