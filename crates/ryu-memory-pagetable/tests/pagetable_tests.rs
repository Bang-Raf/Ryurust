use ryu_memory_pagetable::ffi::*;
use ryu_memory_pagetable::page_table::*;
use std::ptr;

// --- STEP 1: INVARIANTS & SAFETY TESTS ---

#[test]
fn test_step1_null_safety() {
    unsafe {
        assert_eq!(ryu_pagetable_read(ptr::null(), 0x1000), 0);
        ryu_pagetable_map(ptr::null_mut(), 0x1000, 0x5000);
        ryu_pagetable_unmap(ptr::null_mut(), 0x1000);
        ryu_pagetable_destroy(ptr::null_mut()); // Should not panic or crash

        let mut out = 0u64;
        assert_eq!(
            ryu_pagetable_read_batch(ptr::null(), ptr::null(), 0, &mut out),
            RYU_PAGETABLE_ERR_NULL_PTR
        );
    }
}

#[test]
fn test_step1_unmapped_read_returns_zero() {
    let pt = Radix48PageTable::new();
    assert_eq!(pt.read(0), 0);
    assert_eq!(pt.read(0x1000), 0);
    assert_eq!(pt.read(0x8000_0000), 0);
    assert_eq!(pt.read(0x0000_7fff_ffff_f000), 0);
    assert_eq!(pt.read(0x0000_ffff_ffff_f000), 0);
}

// --- STEP 2: DUAL-EXECUTION BIT-IDENTICAL TESTS ---

#[test]
fn test_step2_map_and_read_bit_identical() {
    let mut pt = Radix48PageTable::new();

    // Test addresses across different levels of 48-bit address space
    let test_mappings = [
        (0x0000_0000_1000u64, 0x1111_0000u64), // Level 0..3 low
        (0x0000_0020_0000u64, 0x2222_0000u64), // 2MB boundary
        (0x0000_4000_0000u64, 0x3333_0000u64), // 1GB boundary
        (0x0080_0000_0000u64, 0x4444_0000u64), // 512GB boundary
        (0x7fff_ffff_f000u64, 0x5555_0000u64), // High 48-bit address
    ];

    for &(va, val) in &test_mappings {
        pt.map(va, val);
    }

    for &(va, val) in &test_mappings {
        assert_eq!(
            pt.read(va),
            val,
            "VA 0x{:x} did not return mapped value",
            va
        );
    }

    // Unmap one and verify neighbors stay intact
    pt.unmap(0x0000_4000_0000);
    assert_eq!(pt.read(0x0000_4000_0000), 0);
    assert_eq!(pt.read(0x0000_0000_1000), 0x1111_0000);
    assert_eq!(pt.read(0x7fff_ffff_f000), 0x5555_0000);
}

#[test]
fn test_step2_range_mapping_and_unmapping() {
    let mut pt = Radix48PageTable::new();
    let base_va = 0x1000_0000u64;
    let base_host = 0x5000_0000u64;
    let page_count = 100u64;
    let size = page_count * (PAGE_SIZE as u64);

    pt.map_range(base_va, size, base_host);

    for i in 0..page_count {
        let va = base_va + i * (PAGE_SIZE as u64);
        let expected = base_host + i * (PAGE_SIZE as u64);
        assert_eq!(pt.read(va), expected);
    }

    // Unmap range
    pt.unmap_range(base_va, size);

    for i in 0..page_count {
        let va = base_va + i * (PAGE_SIZE as u64);
        assert_eq!(pt.read(va), 0);
    }
}

#[test]
fn test_step2_batch_read_identical() {
    let mut pt = Radix48PageTable::new();

    let mut vas = Vec::new();
    let mut expected_values = Vec::new();

    for i in 0..1000u64 {
        let va = 0x2000_0000u64 + i * (PAGE_SIZE as u64);
        let val = 0x8000_0000u64 + i * 17;
        pt.map(va, val);
        vas.push(va);
        expected_values.push(val);
    }

    let mut out_values = vec![0u64; 1000];
    unsafe {
        assert_eq!(
            ryu_pagetable_read_batch(&pt, vas.as_ptr(), vas.len(), out_values.as_mut_ptr()),
            RYU_PAGETABLE_OK
        );
    }

    assert_eq!(out_values, expected_values);
}
