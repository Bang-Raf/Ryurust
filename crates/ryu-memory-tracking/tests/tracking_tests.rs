use ryu_memory_tracking::bitmap::*;
use ryu_memory_tracking::ffi::*;
use std::ptr;
use std::sync::Arc;
use std::thread;

// --- STEP 1: INVARIANTS & SAFETY TESTS ---

#[test]
fn test_step1_null_safety() {
    unsafe {
        assert_eq!(ryu_bitmap_is_set(ptr::null(), 10), 0);
        assert_eq!(ryu_bitmap_is_set_range(ptr::null(), 0, 10), 0);
        assert_eq!(ryu_bitmap_any_set(ptr::null()), 0);
        assert_eq!(ryu_bitmap_count_set(ptr::null()), 0);
        let mut out = 0usize;
        assert_eq!(
            ryu_bitmap_find_next_set(ptr::null(), 0, &mut out),
            RYU_BITMAP_NOT_FOUND
        );
        ryu_bitmap_destroy(ptr::null_mut()); // Safe no-op
    }
}

#[test]
fn test_step1_out_of_bounds_safety() {
    let bm = AtomicPageBitmap::new(100, false);
    assert!(!bm.is_set(100));
    assert!(!bm.is_set(1000));
    assert!(!bm.is_set_range(100, 200));
    assert!(!bm.is_set_range(50, 40)); // Invalid start > end
    bm.set(500, true); // No-op, should not panic
}

// --- STEP 2: DUAL-EXECUTION BIT-IDENTICAL TESTS ---

#[test]
fn test_step2_word_boundaries_bit_identical() {
    let bm = AtomicPageBitmap::new(1024, false);

    // Test exact boundary bits around 64-bit boundaries
    let boundary_bits = [0, 63, 64, 65, 127, 128, 129, 255, 256, 1023];
    for &bit in &boundary_bits {
        assert!(!bm.is_set(bit));
        bm.set(bit, true);
        assert!(bm.is_set(bit));
    }

    assert_eq!(bm.count_set_bits(), boundary_bits.len());

    for &bit in &boundary_bits {
        bm.set(bit, false);
        assert!(!bm.is_set(bit));
    }

    assert_eq!(bm.count_set_bits(), 0);
    assert!(!bm.any_set());
}

#[test]
fn test_step2_range_query_bit_identical() {
    let bm = AtomicPageBitmap::new(512, false);

    // Set bit 100
    bm.set(100, true);

    // Range entirely before 100
    assert!(!bm.is_set_range(0, 99));
    // Range including 100
    assert!(bm.is_set_range(90, 110));
    assert!(bm.is_set_range(100, 100));
    // Range entirely after 100
    assert!(!bm.is_set_range(101, 200));

    // Multi-word range set
    bm.set_range(60, 130, true);
    assert!(bm.is_set_range(50, 65));
    assert!(bm.is_set_range(125, 200));
    assert!(bm.is_set(60));
    assert!(bm.is_set(130));
    assert!(!bm.is_set(59));
    assert!(!bm.is_set(131));
}

#[test]
fn test_step2_concurrent_multithreaded_bit_identical() {
    let bm = Arc::new(AtomicPageBitmap::new(64 * 1024, false));
    let mut handles = Vec::new();

    // 8 concurrent threads modifying distinct interleaved bit ranges
    for t in 0..8 {
        let bm_clone = Arc::clone(&bm);
        handles.push(thread::spawn(move || {
            for i in 0..1000 {
                let bit = t + i * 8;
                bm_clone.set(bit, true);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(bm.count_set_bits(), 8000);

    for t in 0..8 {
        for i in 0..1000 {
            let bit = t + i * 8;
            assert!(bm.is_set(bit));
        }
    }
}

#[test]
fn test_step2_find_next_set_scan() {
    let bm = AtomicPageBitmap::new(1000, false);
    let set_bits = [5, 42, 64, 128, 250, 777, 999];
    for &b in &set_bits {
        bm.set(b, true);
    }

    let mut scanned = Vec::new();
    let mut cur = 0;
    while let Some(bit) = bm.find_next_set(cur) {
        scanned.push(bit);
        cur = bit + 1;
    }

    assert_eq!(scanned, set_bits);
}
