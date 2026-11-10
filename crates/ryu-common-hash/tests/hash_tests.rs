//! Two-Step Verification Suite for ryu-common-hash
//!
//! Step 1: Unit, Boundary, Argument & FFI Safety Verification
//! Step 2: Deterministic Differential & Bit-Identical Dual-Execution Verification

use ryu_common_hash::*;

// =========================================================================
// STEP 1: Unit, Boundary, Argument & FFI Safety Verification
// =========================================================================

#[test]
fn step1_test_empty_and_small_lengths() {
    let empty: [u8; 0] = [];
    let (low0, high0) = compute_hash_128(&empty);
    assert!(
        low0 != 0 || high0 != 0,
        "Empty buffer must produce valid non-zero hash"
    );

    // Lengths 1 through 16 (short path)
    for len in 1..=16 {
        let buf = vec![0xABu8; len];
        let (low, high) = compute_hash_128(&buf);
        assert!(low != 0 || high != 0, "Length {} produced zero hash", len);
    }

    // Mid sizes (17 to 240)
    for len in [32, 64, 96, 128, 160, 200, 240] {
        let buf = vec![0xCDu8; len];
        let (low, high) = compute_hash_128(&buf);
        assert!(low != 0 || high != 0, "Length {} produced zero hash", len);
    }
}

#[test]
fn step1_test_ffi_null_safety() {
    unsafe {
        // Null result pointer
        assert_eq!(
            ryu_hash_xxh3_128([0u8; 16].as_ptr(), 16, std::ptr::null_mut()),
            HASH_STATUS_NULL_POINTER
        );

        // Null input with non-zero length
        let mut res = Hash128Result::default();
        assert_eq!(
            ryu_hash_xxh3_128(std::ptr::null(), 16, &mut res),
            HASH_STATUS_NULL_POINTER
        );

        // Null input with zero length (valid empty slice)
        assert_eq!(
            ryu_hash_xxh3_128(std::ptr::null(), 0, &mut res),
            HASH_STATUS_SUCCESS
        );
        assert!(res.low != 0 || res.high != 0);
    }
}

// =========================================================================
// STEP 2: Deterministic Differential & Bit-Identical Dual-Execution Verification
// =========================================================================

#[test]
fn step2_test_deterministic_repeatability() {
    let test_data =
        b"Ryubing Nintendo Switch Emulator High Performance xxHash3 128-bit Testing Vector";

    let (low1, high1) = compute_hash_128(test_data);
    let (low2, high2) = compute_hash_128(test_data);
    let (low3, high3) = compute_hash_128(test_data);

    assert_eq!(low1, low2);
    assert_eq!(high1, high2);
    assert_eq!(low2, low3);
    assert_eq!(high2, high3);
}

#[test]
fn step2_test_batch_parallel_bit_identical_dual_execution() {
    let num_items = 64;
    let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(num_items);

    for i in 0..num_items {
        let len = (i * 137 + 17) % 4096;
        let mut buf = vec![0u8; len];
        for (j, b) in buf.iter_mut().enumerate() {
            *b = ((i * 31 + j * 17) & 0xFF) as u8;
        }
        buffers.push(buf);
    }

    // 1. Sequential execution
    let mut seq_results = Vec::with_capacity(num_items);
    for buf in &buffers {
        seq_results.push(compute_hash_128(buf));
    }

    // 2. Parallel Rayon batch execution
    let pointers: Vec<*const u8> = buffers.iter().map(|b| b.as_ptr()).collect();
    let lengths: Vec<usize> = buffers.iter().map(|b| b.len()).collect();
    let mut par_results = vec![Hash128Result::default(); num_items];

    unsafe {
        let status = ryu_hash_xxh3_128_batch_parallel(
            pointers.as_ptr(),
            lengths.as_ptr(),
            num_items,
            par_results.as_mut_ptr(),
        );
        assert_eq!(status, HASH_STATUS_SUCCESS);
    }

    // 3. Differential Verification: 100% Bit-Identical Assertion
    for i in 0..num_items {
        assert_eq!(
            seq_results[i].0, par_results[i].low,
            "Sequential vs Parallel LOW mismatch at index {}",
            i
        );
        assert_eq!(
            seq_results[i].1, par_results[i].high,
            "Sequential vs Parallel HIGH mismatch at index {}",
            i
        );
    }
}
