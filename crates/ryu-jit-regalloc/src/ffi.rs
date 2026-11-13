use crate::allocator::{bitset_and_not, bitset_or_avx2, LinearScanEngine};
use crate::types::{Interval, RegisterMasksNative};
use std::slice;

pub const RYU_JIT_REGALLOC_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_jit_regalloc_version() -> u32 {
    RYU_JIT_REGALLOC_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_regalloc_create() -> *mut LinearScanEngine {
    Box::into_raw(Box::new(LinearScanEngine::new()))
}

/// Frees an engine. Null is a no-op.
///
/// # Safety
/// `ctx` must be null or a handle from `ryu_regalloc_create` that has not been destroyed yet;
/// it must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_destroy(ctx: *mut LinearScanEngine) {
    if !ctx.is_null() {
        // SAFETY: non-null `ctx` came from `Box::into_raw` in `ryu_regalloc_create` and the
        // caller guarantees it is not freed twice.
        unsafe {
            drop(Box::from_raw(ctx));
        }
    }
}

/// Clears all intervals. Null is a no-op.
///
/// # Safety
/// `ctx` must be null or a live handle from `ryu_regalloc_create`, not used concurrently.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_reset(ctx: *mut LinearScanEngine) {
    if !ctx.is_null() {
        // SAFETY: non-null live handle with exclusive access, per the caller contract.
        let engine = unsafe { &mut *ctx };
        engine.reset();
    }
}

/// Adds an interval and returns the new interval count, or `0` for a null context.
///
/// # Safety
/// `ctx` must be null or a live handle from `ryu_regalloc_create`, not used concurrently.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_add_interval(
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
    // SAFETY: non-null live handle with exclusive access, per the caller contract.
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

/// Runs the allocation pass. Returns the number of intervals that got a register, `0` for a
/// null context, or `usize::MAX` ([`RUN_PASS_FIXED_CONFLICT`](crate::RUN_PASS_FIXED_CONFLICT),
/// `-1` when read as a signed integer) if two overlapping fixed intervals demand the same
/// register or a fixed interval names a register outside `0..32`. In that case nothing was allocated (every result has register `-1` and spill
/// offset `-1`) and `ryu_regalloc_get_fixed_conflict` names the offending pair.
///
/// # Safety
/// `ctx` must be null or a live handle from `ryu_regalloc_create`, not used concurrently.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_run_pass(
    ctx: *mut LinearScanEngine,
    int_avail: u32,
    vec_avail: u32,
    reg_count: u32,
) -> usize {
    if ctx.is_null() {
        return 0;
    }
    // SAFETY: non-null live handle with exclusive access, per the caller contract.
    let engine = unsafe { &mut *ctx };
    let masks = RegisterMasksNative {
        int_available: int_avail,
        vec_available: vec_avail,
        registers_count: reg_count,
    };
    engine.run_pass(masks)
}

/// Reports the fixed-register conflict that made the last `ryu_regalloc_run_pass` on `ctx`
/// fail. Returns `1` and fills the non-null outputs (ids of the two intervals in start
/// order, their register class and the register both are fixed to) if there was one, `0`
/// otherwise (including a null context). Both ids are the same interval's when it names a
/// register outside `0..32`.
///
/// # Safety
/// `ctx` must be null or a live handle from `ryu_regalloc_create`. Each output pointer must be
/// null or valid for one aligned write of its type.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_get_fixed_conflict(
    ctx: *const LinearScanEngine,
    out_first_id: *mut u32,
    out_second_id: *mut u32,
    out_reg_type: *mut u8,
    out_reg: *mut i16,
) -> i32 {
    if ctx.is_null() {
        return 0;
    }
    // SAFETY: `ctx` is non-null and, per the API contract, came from `ryu_regalloc_create`
    // and has not been destroyed.
    let engine = unsafe { &*ctx };
    let Some(conflict) = engine.fixed_conflict() else {
        return 0;
    };
    // SAFETY: each output pointer is either null (skipped) or, per the API contract, valid
    // for a write of its pointee type.
    unsafe {
        if !out_first_id.is_null() {
            *out_first_id = conflict.first_id;
        }
        if !out_second_id.is_null() {
            *out_second_id = conflict.second_id;
        }
        if !out_reg_type.is_null() {
            *out_reg_type = conflict.reg_type;
        }
        if !out_reg.is_null() {
            *out_reg = conflict.reg;
        }
    }
    1
}

/// Reads interval `index` after a pass. Returns `1` and fills the non-null outputs, or `0`
/// for a null context or an out-of-range index.
///
/// # Safety
/// `ctx` must be null or a live handle from `ryu_regalloc_create`. Each output pointer must be
/// null or valid for one aligned write of its type.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_get_result(
    ctx: *const LinearScanEngine,
    index: usize,
    out_id: *mut u32,
    out_reg: *mut i16,
    out_spill: *mut i32,
) -> i32 {
    if ctx.is_null() {
        return 0;
    }
    // SAFETY: non-null live handle, per the caller contract; only read here.
    let engine = unsafe { &*ctx };
    if index < engine.intervals.len() {
        let iv = &engine.intervals[index];
        // SAFETY: each output pointer is either null (skipped) or valid for a write of its
        // pointee type, per the caller contract.
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

/// Returns the index of the highest value in `span[..len]`, or `0` if `span` is null or
/// `len` is `0`.
///
/// # Safety
/// `span` must be null or valid for reads of `len` `i32`s for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_get_highest_index(span: *const i32, len: usize) -> i32 {
    if span.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: `span` is non-null and valid for `len` reads, per the caller contract.
    let slice = unsafe { slice::from_raw_parts(span, len) };
    LinearScanEngine::get_highest_value_index(slice) as i32
}

/// `dst[..count] |= src[..count]`. Returns `1` if `dst` changed, `0` otherwise (including a
/// null pointer or `count == 0`). `dst == src` is allowed (never changes anything).
///
/// # Safety
/// `dst` must be valid for reads and writes and `src` for reads of `count` `u64`s. The two
/// ranges must either be identical or not overlap at all.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_bitset_or(
    dst: *mut u64,
    src: *const u64,
    count: usize,
) -> u32 {
    if !dst.is_null() && !src.is_null() && count > 0 {
        if dst.cast_const() == src {
            // x | x == x; handled here so no `&mut`/`&` pair aliases the same memory.
            return 0;
        }
        // SAFETY: both pointers are non-null and valid for `count` elements, and the ranges
        // do not overlap (identical ranges were handled above), per the caller contract.
        let d = unsafe { slice::from_raw_parts_mut(dst, count) };
        // SAFETY: as above; `src` is only read.
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

/// `dst[..count] &= !src[..count]`. Returns `1` if `dst` changed, `0` otherwise (including a
/// null pointer or `count == 0`). `dst == src` is allowed (clears `dst`).
///
/// # Safety
/// `dst` must be valid for reads and writes and `src` for reads of `count` `u64`s. The two
/// ranges must either be identical or not overlap at all.
#[no_mangle]
pub unsafe extern "C" fn ryu_regalloc_bitset_and_not(
    dst: *mut u64,
    src: *const u64,
    count: usize,
) -> u32 {
    if !dst.is_null() && !src.is_null() && count > 0 {
        // SAFETY: `dst` is non-null and valid for `count` reads and writes, per the caller
        // contract; `src` is not turned into a separate reference when it equals `dst`.
        let d = unsafe { slice::from_raw_parts_mut(dst, count) };
        if dst.cast_const() == src {
            // x & !x == 0; handled here so no `&mut`/`&` pair aliases the same memory.
            let modified = d.iter().any(|&w| w != 0);
            d.fill(0);
            return modified as u32;
        }
        // SAFETY: `src` is non-null, valid for `count` reads and does not overlap `dst`
        // (identical ranges were handled above), per the caller contract.
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
