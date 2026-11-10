use ryu_jit_regalloc::*;

#[test]
fn test_bump_arena_reset() {
    let mut arena = BumpArena::with_capacity(8192);
    let s1 = arena.alloc_slice::<u32>(256).unwrap();
    assert_eq!(s1.len(), 256);
    assert!(arena.used_bytes() >= 1024);

    arena.reset();
    assert_eq!(arena.used_bytes(), 0);
}

#[test]
fn test_linear_scan_non_overlapping_and_overlapping() {
    let mut engine = LinearScanEngine::new();
    // Interval 0: [1..10], Int
    engine.add_interval(Interval {
        id: 0,
        start: 1,
        end: 10,
        reg_type: 0,
        ..Default::default()
    });
    // Interval 1: [2..8], Int (overlaps Interval 0 -> needs different register)
    engine.add_interval(Interval {
        id: 1,
        start: 2,
        end: 8,
        reg_type: 0,
        ..Default::default()
    });
    // Interval 2: [12..20], Int (can reuse Interval 0's register)
    engine.add_interval(Interval {
        id: 2,
        start: 12,
        end: 20,
        reg_type: 0,
        ..Default::default()
    });

    let masks = RegisterMasksNative {
        int_available: 0b1111, // 4 Int registers available
        vec_available: 0b1111,
        registers_count: 16,
    };

    let allocs = engine.run_pass(masks);
    assert_eq!(allocs, 3);
    assert_ne!(
        engine.intervals[0].assigned_reg,
        engine.intervals[1].assigned_reg
    );
    assert_eq!(engine.intervals[2].assigned_reg, 0); // Reused reg 0 cleanly
}

#[test]
fn test_linear_scan_spill() {
    let mut engine = LinearScanEngine::new();
    // Only 2 registers available, 3 overlapping intervals
    engine.add_interval(Interval {
        id: 0,
        start: 1,
        end: 20,
        reg_type: 0,
        ..Default::default()
    });
    engine.add_interval(Interval {
        id: 1,
        start: 2,
        end: 20,
        reg_type: 0,
        ..Default::default()
    });
    engine.add_interval(Interval {
        id: 2,
        start: 3,
        end: 20,
        reg_type: 0,
        ..Default::default()
    });

    let masks = RegisterMasksNative {
        int_available: 0b0011, // Only 2 registers (0, 1)
        vec_available: 0,
        registers_count: 4,
    };

    let allocs = engine.run_pass(masks);
    assert_eq!(allocs, 2);
    assert_eq!(engine.intervals[2].assigned_reg, -1);
    assert_eq!(engine.intervals[2].spill_offset, 0);
}

#[test]
fn test_avx2_bitset_or_and_not() {
    let mut dst = vec![0x00FF_00FF_00FF_00FFu64; 16];
    let src = vec![0xFF00_FF00_FF00_FF00u64; 16];
    assert!(bitset_or_avx2(&mut dst, &src));
    for &w in &dst {
        assert_eq!(w, u64::MAX);
    }
    // Calling again should report no modification (fixpoint convergence)
    assert!(!bitset_or_avx2(&mut dst, &src));

    // Test AND-NOT
    assert!(bitset_and_not(&mut dst, &src));
    for &w in &dst {
        assert_eq!(w, 0x00FF_00FF_00FF_00FFu64);
    }
    assert!(!bitset_and_not(&mut dst, &src));
}

#[test]
fn test_c_abi_lifecycle() {
    assert_eq!(ryu_jit_regalloc_version(), 0x00010000);
    let ctx = ryu_regalloc_create();
    assert!(!ctx.is_null());

    ryu_regalloc_add_interval(ctx, 100, 1, 10, 0, 0, -1);
    ryu_regalloc_add_interval(ctx, 101, 2, 12, 0, 0, -1);

    let allocated = ryu_regalloc_run_pass(ctx, 0xFF, 0xFF, 16);
    assert_eq!(allocated, 2);

    let mut id = 0;
    let mut reg = -1;
    let mut spill = -1;
    assert_eq!(
        ryu_regalloc_get_result(ctx, 0, &mut id, &mut reg, &mut spill),
        1
    );
    assert_eq!(id, 100);
    assert!(reg >= 0);

    ryu_regalloc_reset(ctx);
    ryu_regalloc_destroy(ctx);
}
