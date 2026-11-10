//! C-ABI Foreign Function Interface for SIMD xxHash3 128-bit Content Hashing.

use crate::xxh3::{compute_hash_128, compute_hash_128_with_seed};

pub const HASH_STATUS_SUCCESS: i32 = 0;
pub const HASH_STATUS_NULL_POINTER: i32 = -1;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hash128Result {
    pub low: u64,
    pub high: u64,
}

#[no_mangle]
pub unsafe extern "C" fn ryu_hash_xxh3_128(
    input: *const u8,
    input_len: usize,
    result: *mut Hash128Result,
) -> i32 {
    if result.is_null() {
        return HASH_STATUS_NULL_POINTER;
    }

    let input_slice = if input.is_null() {
        if input_len == 0 {
            &[]
        } else {
            return HASH_STATUS_NULL_POINTER;
        }
    } else {
        std::slice::from_raw_parts(input, input_len)
    };

    let (low, high) = compute_hash_128(input_slice);
    (*result).low = low;
    (*result).high = high;

    HASH_STATUS_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn ryu_hash_xxh3_128_with_seed(
    input: *const u8,
    input_len: usize,
    seed: u64,
    result: *mut Hash128Result,
) -> i32 {
    if result.is_null() {
        return HASH_STATUS_NULL_POINTER;
    }

    let input_slice = if input.is_null() {
        if input_len == 0 {
            &[]
        } else {
            return HASH_STATUS_NULL_POINTER;
        }
    } else {
        std::slice::from_raw_parts(input, input_len)
    };

    let (low, high) = compute_hash_128_with_seed(input_slice, seed);
    (*result).low = low;
    (*result).high = high;

    HASH_STATUS_SUCCESS
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn ryu_hash_xxh3_128_batch_parallel(
    pointers: *const *const u8,
    lengths: *const usize,
    count: usize,
    results: *mut Hash128Result,
) -> i32 {
    if pointers.is_null() || lengths.is_null() || results.is_null() {
        return HASH_STATUS_NULL_POINTER;
    }
    if count == 0 {
        return HASH_STATUS_SUCCESS;
    }

    let res_slice = std::slice::from_raw_parts_mut(results, count);

    #[cfg(feature = "parallel")]
    {
        if count >= 16 {
            let ptrs: Vec<usize> = (0..count).map(|i| *pointers.add(i) as usize).collect();
            let lens: Vec<usize> = (0..count).map(|i| *lengths.add(i)).collect();

            res_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, out_res)| {
                    let p = ptrs[i] as *const u8;
                    let l = lens[i];
                    let slice = if p.is_null() {
                        &[]
                    } else {
                        unsafe { std::slice::from_raw_parts(p, l) }
                    };
                    let (low, high) = compute_hash_128(slice);
                    out_res.low = low;
                    out_res.high = high;
                });
            return HASH_STATUS_SUCCESS;
        }
    }

    for i in 0..count {
        let p = *pointers.add(i);
        let l = *lengths.add(i);
        let slice = if p.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(p, l)
        };
        let (low, high) = compute_hash_128(slice);
        res_slice[i].low = low;
        res_slice[i].high = high;
    }

    HASH_STATUS_SUCCESS
}
