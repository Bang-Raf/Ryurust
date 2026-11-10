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

#[test]
fn test_bump_arena_multiple_live_slices_and_bounds() {
    let mut arena = BumpArena::with_capacity(4096);
    {
        let a = arena.alloc_slice::<u64>(8).unwrap();
        let b = arena.alloc_slice::<u8>(3).unwrap();
        let c = arena.alloc_slice::<u32>(4).unwrap();
        assert!(a.iter().all(|&x| x == 0), "slices are default-initialised");
        a.fill(u64::MAX);
        b.fill(0xAB);
        c.fill(7);
        // Disjoint: writing one does not disturb the others.
        assert!(a.iter().all(|&x| x == u64::MAX));
        assert!(b.iter().all(|&x| x == 0xAB));
        assert!(c.iter().all(|&x| x == 7));
        assert_eq!(c.as_ptr() as usize % std::mem::align_of::<u32>(), 0);
    }
    assert!(arena.alloc_slice::<u8>(usize::MAX).is_none());
    assert!(arena.alloc_slice::<u64>(usize::MAX / 4).is_none());
    assert!(arena.alloc_slice::<u8>(4096).is_none());
    arena.reset();
    assert_eq!(arena.used_bytes(), 0);
    // After reset the whole block is usable again, and memory is re-initialised.
    let all = arena.alloc_slice::<u8>(4096).unwrap();
    assert!(all.iter().all(|&x| x == 0));
}

fn xorshift(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

/// Same distribution as `benches/benches/jit_regalloc.rs`: mostly short temporaries, ~30%
/// vector, ~3% fixed to registers 0..3. Fixed intervals that would overlap an earlier fixed
/// interval on the same register are made non-fixed (two such intervals are unsatisfiable).
fn random_intervals(n: usize, seed: u32, shuffled: bool) -> Vec<Interval> {
    let mut next = xorshift(seed);
    let mut fixed_until = [i32::MIN; 3];
    let mut v: Vec<Interval> = (0..n)
        .map(|i| {
            let start = (i * 2) as i32;
            let roll = next() % 100;
            let len = if roll < 80 {
                1 + next() % 16
            } else if roll < 95 {
                16 + next() % 112
            } else {
                128 + next() % 896
            } as i32;
            let is_vec = next() % 10 < 3;
            let mut is_fixed = !is_vec && next() % 100 < 3;
            let fixed_reg = (next() % 3) as usize;
            if is_fixed {
                if fixed_until[fixed_reg] > start {
                    is_fixed = false;
                } else {
                    fixed_until[fixed_reg] = start + len;
                }
            }
            Interval {
                id: i as u32,
                start,
                end: start + len,
                reg_type: is_vec as u8,
                is_fixed,
                fixed_reg: if is_fixed { fixed_reg as i16 } else { -1 },
                first_use: start,
                ..Default::default()
            }
        })
        .collect();
    if shuffled {
        for i in (1..v.len()).rev() {
            let j = next() as usize % (i + 1);
            v.swap(i, j);
        }
    }
    v
}

fn conflicting_pairs(ivs: &[Interval]) -> usize {
    let mut conflicts = 0;
    for (a_idx, a) in ivs.iter().enumerate() {
        if a.assigned_reg < 0 {
            continue;
        }
        for b in &ivs[a_idx + 1..] {
            if b.assigned_reg == a.assigned_reg
                && b.reg_type == a.reg_type
                && a.start < b.end
                && b.start < a.end
            {
                conflicts += 1;
            }
        }
    }
    conflicts
}

#[test]
fn test_linear_scan_random_with_fixed_intervals_no_shared_registers() {
    let masks = RegisterMasksNative {
        int_available: 0xFFFF & !((1 << 4) | (1 << 5)),
        vec_available: 0xFFFF,
        registers_count: 16,
    };
    for &n in &[512usize, 2048] {
        for seed in 1..=16u32 {
            for shuffled in [false, true] {
                let ivs = random_intervals(n, seed.wrapping_mul(0x9E37_79B9) | 1, shuffled);
                let fixed_count = ivs.iter().filter(|iv| iv.is_fixed).count();
                assert!(fixed_count > 0, "generator must produce fixed intervals");

                let mut engine = LinearScanEngine::new();
                for &iv in &ivs {
                    engine.add_interval(iv);
                }
                let allocated = engine.run_pass(masks);
                let out = &engine.intervals;

                assert_eq!(
                    conflicting_pairs(out),
                    0,
                    "n={n} seed={seed} shuffled={shuffled}"
                );
                assert_eq!(
                    allocated,
                    out.iter().filter(|iv| iv.assigned_reg >= 0).count()
                );
                for iv in out {
                    if iv.is_fixed {
                        assert_eq!(iv.assigned_reg, iv.fixed_reg, "fixed keeps its register");
                    } else if iv.assigned_reg >= 0 {
                        let avail = if iv.reg_type == 0 {
                            masks.int_available
                        } else {
                            masks.vec_available
                        };
                        assert!(avail & (1 << iv.assigned_reg) != 0);
                    } else {
                        assert!(iv.spill_offset >= 0, "spilled interval needs a slot");
                    }
                }
            }
        }
    }
}

#[test]
fn test_linear_scan_fixed_interval_does_not_share_with_live_holder() {
    // Interval 0 is live over [0, 100) and would naturally take register 0; a fixed interval
    // on register 0 starts at 10. They must not share register 0.
    let mut engine = LinearScanEngine::new();
    engine.add_interval(Interval {
        id: 0,
        start: 0,
        end: 100,
        ..Default::default()
    });
    engine.add_interval(Interval {
        id: 1,
        start: 10,
        end: 12,
        is_fixed: true,
        fixed_reg: 0,
        ..Default::default()
    });
    engine.add_interval(Interval {
        id: 2,
        start: 20,
        end: 30,
        ..Default::default()
    });
    let masks = RegisterMasksNative {
        int_available: 0b11,
        vec_available: 0,
        registers_count: 2,
    };
    let allocated = engine.run_pass(masks);
    let iv = &engine.intervals;
    assert_eq!(iv[1].assigned_reg, 0);
    assert_eq!(
        iv[0].assigned_reg, 1,
        "long interval avoids the fixed register"
    );
    assert_eq!(
        iv[2].assigned_reg, 0,
        "reg 0 is free again after the fixed interval"
    );
    assert_eq!(allocated, 3);
    assert_eq!(conflicting_pairs(iv), 0);

    // With only register 0 available the long interval must spill instead of sharing.
    engine.reset();
    engine.add_interval(Interval {
        id: 0,
        start: 0,
        end: 100,
        ..Default::default()
    });
    engine.add_interval(Interval {
        id: 1,
        start: 10,
        end: 12,
        is_fixed: true,
        fixed_reg: 0,
        ..Default::default()
    });
    let allocated = engine.run_pass(RegisterMasksNative {
        int_available: 0b1,
        vec_available: 0,
        registers_count: 1,
    });
    assert_eq!(engine.intervals[0].assigned_reg, -1);
    assert_eq!(engine.intervals[0].spill_offset, 0);
    assert_eq!(engine.intervals[1].assigned_reg, 0);
    assert_eq!(allocated, 1);
}
