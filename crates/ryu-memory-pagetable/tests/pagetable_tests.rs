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

#[test]
fn test_unmap_range_huge_size_is_fast_and_complete() {
    let mut pt = Radix48PageTable::new();
    let pages = [
        0x0u64,
        0x1000,
        0x20_0000,
        0x4000_0000,
        0x7fff_ffff_f000,
        0x8000_0000_0000 - 0x1000,
    ];
    for &va in &pages {
        pt.map(va, va | 0x1);
    }

    // Used to loop over 2^52 pages; must now finish at once and clear every page.
    let start = std::time::Instant::now();
    pt.unmap_range(0, u64::MAX);
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    for &va in &pages {
        assert_eq!(pt.read(va), 0, "{va:#x} still mapped");
    }
}

#[test]
fn test_unmap_range_matches_per_page_unmap() {
    // Sparse mappings, then the same sub-range unmapped two ways.
    let mut a = Radix48PageTable::new();
    let mut b = Radix48PageTable::new();
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut vas = Vec::new();
    for _ in 0..2000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let va = (x % (1 << 40)) & !(PAGE_SIZE as u64 - 1);
        vas.push(va);
        a.map(va, va | 1);
        b.map(va, va | 1);
    }
    let (lo, size) = (0x40_0000_0000u64, 0x40_0000_0000u64);
    a.unmap_range(lo, size);
    for i in 0..size / PAGE_SIZE as u64 {
        let va = lo + i * PAGE_SIZE as u64;
        if b.read(va) != 0 {
            b.unmap(va);
        }
    }
    for &va in &vas {
        assert_eq!(a.read(va), b.read(va), "{va:#x}");
    }
}

#[test]
fn test_ranges_are_clipped_to_48_bits() {
    let mut pt = Radix48PageTable::new();
    pt.map(0x0, 0xAAAA);

    // A range running past the top of the 48-bit space used to wrap round and
    // overwrite page 0.
    let top = (1u64 << 48) - 2 * PAGE_SIZE as u64;
    pt.map_range(top, 4 * PAGE_SIZE as u64, 0x5000);
    assert_eq!(pt.read(top), 0x5000);
    assert_eq!(pt.read(top + PAGE_SIZE as u64), 0x6000);
    assert_eq!(pt.read(0x0), 0xAAAA);

    pt.unmap_range(top, 4 * PAGE_SIZE as u64);
    assert_eq!(pt.read(top), 0);
    assert_eq!(pt.read(0x0), 0xAAAA);

    // A range starting outside the space does nothing.
    pt.map_range(1 << 48, PAGE_SIZE as u64, 0x7000);
    assert_eq!(pt.read(0x0), 0xAAAA);
}
