//! C-ABI Foreign Function Interface for SIMD xxHash3 128-bit Content Hashing.

use crate::xxh3::{compute_hash_128, compute_hash_128_with_seed};

pub const HASH_STATUS_SUCCESS: i32 = 0;
pub const HASH_STATUS_NULL_POINTER: i32 = -1;

pub const RYU_COMMON_HASH_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_common_hash_version() -> u32 {
    RYU_COMMON_HASH_VERSION
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hash128Result {
    pub low: u64,
    pub high: u64,
}

/// # Safety
/// `input` must be valid for reads of `input_len` bytes (it may be null only when
/// `input_len` is 0); `result` must be valid for writing one `Hash128Result`.
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

/// # Safety
/// `input` must be valid for reads of `input_len` bytes (it may be null only when
/// `input_len` is 0); `result` must be valid for writing one `Hash128Result`.
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

/// A null entry in `pointers` is hashed as an empty input.
///
/// # Safety
/// `pointers` and `lengths` must be valid for reads and `results` for writes of `count`
/// elements. Each non-null `pointers[i]` must be valid for reads of `lengths[i]` bytes
/// for the whole call, and no input may overlap `results`.
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
                        // SAFETY: `p` is non-null and the caller guarantees it stays valid for
                        // `l` bytes of reads until this call returns.
                        unsafe { std::slice::from_raw_parts(p, l) }
                    };
                    let (low, high) = compute_hash_128(slice);
                    out_res.low = low;
                    out_res.high = high;
                });
            return HASH_STATUS_SUCCESS;
        }
    }

    for (i, out_res) in res_slice.iter_mut().enumerate() {
        let p = *pointers.add(i);
        let l = *lengths.add(i);
        let slice = if p.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(p, l)
        };
        let (low, high) = compute_hash_128(slice);
        out_res.low = low;
        out_res.high = high;
    }

    HASH_STATUS_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_common_hash_version(), RYU_COMMON_HASH_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let data = [1u8; 8];
        let mut res = Hash128Result::default();
        unsafe {
            assert_eq!(
                ryu_hash_xxh3_128(data.as_ptr(), 8, ptr::null_mut()),
                HASH_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_hash_xxh3_128(ptr::null(), 8, &mut res),
                HASH_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_hash_xxh3_128(ptr::null(), 0, &mut res),
                HASH_STATUS_SUCCESS
            );
            assert_eq!(
                ryu_hash_xxh3_128_with_seed(data.as_ptr(), 8, 1, ptr::null_mut()),
                HASH_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_hash_xxh3_128_with_seed(ptr::null(), 8, 1, &mut res),
                HASH_STATUS_NULL_POINTER
            );

            let ptrs = [data.as_ptr(), ptr::null()];
            let lens = [8usize, 0];
            let mut out = [Hash128Result::default(); 2];
            assert_eq!(
                ryu_hash_xxh3_128_batch_parallel(ptr::null(), lens.as_ptr(), 2, out.as_mut_ptr()),
                HASH_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_hash_xxh3_128_batch_parallel(ptrs.as_ptr(), ptr::null(), 2, out.as_mut_ptr()),
                HASH_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_hash_xxh3_128_batch_parallel(ptrs.as_ptr(), lens.as_ptr(), 2, ptr::null_mut()),
                HASH_STATUS_NULL_POINTER
            );
            // A null element is hashed as empty input rather than rejected.
            assert_eq!(
                ryu_hash_xxh3_128_batch_parallel(ptrs.as_ptr(), lens.as_ptr(), 2, out.as_mut_ptr()),
                HASH_STATUS_SUCCESS
            );
            let (low, high) = compute_hash_128(&[]);
            assert_eq!(out[1], Hash128Result { low, high });
        }
    }
}
