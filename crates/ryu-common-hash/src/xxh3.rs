//! High-throughput xxHash3 128-bit hashing implementation.

use twox_hash::xxhash3_128::Hasher as XxHash3_128;

#[inline(always)]
pub fn compute_hash_128(input: &[u8]) -> (u64, u64) {
    let val: u128 = XxHash3_128::oneshot(input);
    let low = (val & 0xFFFF_FFFF_FFFF_FFFF) as u64;
    let high = (val >> 64) as u64;
    (low, high)
}

#[inline(always)]
pub fn compute_hash_128_with_seed(input: &[u8], seed: u64) -> (u64, u64) {
    let val: u128 = XxHash3_128::oneshot_with_seed(seed, input);
    let low = (val & 0xFFFF_FFFF_FFFF_FFFF) as u64;
    let high = (val >> 64) as u64;
    (low, high)
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Computes hashes for multiple memory buffers in parallel using Rayon.
pub fn compute_hashes_batch_parallel(inputs: &[&[u8]], results: &mut [(u64, u64)]) {
    assert_eq!(inputs.len(), results.len());

    #[cfg(feature = "parallel")]
    {
        if inputs.len() >= 16 {
            inputs
                .par_iter()
                .zip(results.par_iter_mut())
                .for_each(|(in_buf, out_res)| {
                    *out_res = compute_hash_128(in_buf);
                });
            return;
        }
    }

    for (in_buf, out_res) in inputs.iter().zip(results.iter_mut()) {
        *out_res = compute_hash_128(in_buf);
    }
}
