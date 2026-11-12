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
    // SAFETY: `ctx` comes from `ryu_regalloc_create`, is live until the final destroy, and
    // the outputs are valid locals.
    unsafe {
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

fn iv(id: u32, start: i32, end: i32) -> Interval {
    Interval {
        id,
        start,
        end,
        ..Default::default()
    }
}

fn fixed(id: u32, start: i32, end: i32, reg: i16) -> Interval {
    Interval {
        is_fixed: true,
        fixed_reg: reg,
        ..iv(id, start, end)
    }
}

fn by_id(engine: &LinearScanEngine, id: u32) -> Interval {
    *engine.intervals.iter().find(|iv| iv.id == id).unwrap()
}

const TWO_INT_REGS: RegisterMasksNative = RegisterMasksNative {
    int_available: 0b11,
    vec_available: 0,
    registers_count: 2,
};

#[test]
fn test_overlapping_fixed_intervals_on_same_register_are_reported() {
    let mut engine = LinearScanEngine::new();
    engine.add_interval(iv(1, 0, 4));
    engine.add_interval(fixed(10, 2, 20, 1));
    engine.add_interval(iv(2, 5, 9));
    engine.add_interval(fixed(11, 8, 12, 1));

    let expected = FixedRegisterConflict {
        first_id: 10,
        second_id: 11,
        reg_type: 0,
        reg: 1,
    };
    assert_eq!(engine.try_run_pass(TWO_INT_REGS), Err(expected));
    assert_eq!(engine.fixed_conflict(), Some(expected));
    // Nothing is allocated, so no result can be mistaken for a valid one.
    for iv in &engine.intervals {
        assert_eq!((iv.assigned_reg, iv.spill_offset), (-1, -1));
    }
    assert_eq!(engine.next_spill_offset, 0);

    // The infallible entry point reports the same thing through its sentinel.
    assert_eq!(engine.run_pass(TWO_INT_REGS), RUN_PASS_FIXED_CONFLICT);
    assert_eq!(engine.fixed_conflict(), Some(expected));

    // The conflict does not stick to the engine.
    engine.reset();
    assert_eq!(engine.fixed_conflict(), None);
    engine.add_interval(fixed(10, 2, 20, 1));
    assert_eq!(engine.try_run_pass(TWO_INT_REGS), Ok(1));
    assert_eq!(engine.fixed_conflict(), None);
}

#[test]
fn test_fixed_conflict_found_behind_shorter_fixed_interval_and_in_unsorted_input() {
    // 22 overlaps 20, not its chain neighbour 21. Added out of order on purpose.
    let mut engine = LinearScanEngine::new();
    engine.add_interval(fixed(22, 50, 60, 3));
    engine.add_interval(fixed(21, 10, 12, 3));
    engine.add_interval(fixed(20, 0, 100, 3));
    let err = engine.try_run_pass(TWO_INT_REGS).unwrap_err();
    assert_eq!((err.first_id, err.second_id, err.reg), (20, 21, 3));

    engine.reset();
    engine.add_interval(fixed(20, 0, 100, 3));
    engine.add_interval(fixed(21, 100, 112, 3));
    engine.add_interval(fixed(22, 50, 60, 3));
    let err = engine.try_run_pass(TWO_INT_REGS).unwrap_err();
    assert_eq!((err.first_id, err.second_id, err.reg), (20, 22, 3));
    assert!(err.to_string().contains("integer register 3"));
}

#[test]
fn test_fixed_intervals_that_do_not_conflict_are_accepted() {
    let mut engine = LinearScanEngine::new();
    // Back to back on one register (half-open ranges).
    engine.add_interval(fixed(1, 0, 10, 0));
    engine.add_interval(fixed(2, 10, 20, 0));
    // Same time, different register.
    engine.add_interval(fixed(3, 0, 10, 1));
    // Same time and register number, different register class.
    engine.add_interval(Interval {
        reg_type: 1,
        ..fixed(4, 0, 10, 0)
    });
    // An empty interval is live nowhere.
    engine.add_interval(fixed(5, 15, 15, 0));

    assert_eq!(engine.try_run_pass(TWO_INT_REGS), Ok(5));
    assert_eq!(engine.fixed_conflict(), None);
    for iv in &engine.intervals {
        assert_eq!(iv.assigned_reg, iv.fixed_reg);
    }
}

#[test]
fn test_c_abi_reports_fixed_conflict() {
    // SAFETY: `ctx` comes from `ryu_regalloc_create`, is live until the final destroy, and
    // every output is a valid local or null.
    unsafe {
        let ctx = ryu_regalloc_create();
        ryu_regalloc_add_interval(ctx, 7, 0, 10, 1, 1, 2);
        ryu_regalloc_add_interval(ctx, 8, 5, 6, 1, 1, 2);
        ryu_regalloc_add_interval(ctx, 9, 0, 3, 0, 0, -1);

        let (mut first, mut second, mut reg_type, mut reg) = (0u32, 0u32, 0u8, 0i16);
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(ctx, &mut first, &mut second, &mut reg_type, &mut reg),
            0,
            "no pass has run yet"
        );

        let ret = ryu_regalloc_run_pass(ctx, 0xFF, 0xFF, 16);
        assert_eq!(ret, usize::MAX);
        assert_eq!(ret as i32, -1, "what the C# wrapper's (int) cast sees");
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(ctx, &mut first, &mut second, &mut reg_type, &mut reg),
            1
        );
        assert_eq!((first, second, reg_type, reg), (7, 8, 1, 2));
        // Null outputs are skipped.
        let null = std::ptr::null_mut();
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(
                ctx,
                null,
                null,
                std::ptr::null_mut(),
                null as *mut i16
            ),
            1
        );
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(
                std::ptr::null(),
                null,
                null,
                std::ptr::null_mut(),
                null as *mut i16
            ),
            0
        );

        for index in 0..3 {
            let (mut id, mut r, mut spill) = (0u32, 0i16, 0i32);
            assert_eq!(
                ryu_regalloc_get_result(ctx, index, &mut id, &mut r, &mut spill),
                1
            );
            assert_eq!((r, spill), (-1, -1));
        }

        ryu_regalloc_reset(ctx);
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(ctx, &mut first, &mut second, &mut reg_type, &mut reg),
            0
        );
        ryu_regalloc_destroy(ctx);
    }
}

#[test]
fn test_spill_evicts_active_interval_that_ends_last() {
    let mut engine = LinearScanEngine::new();
    engine.add_interval(iv(0, 0, 100)); // reg 0, ends last
    engine.add_interval(iv(1, 1, 50)); // reg 1
    engine.add_interval(iv(2, 2, 10)); // nothing free: evicts interval 0
    engine.add_interval(iv(3, 12, 20)); // reg 0 again, after interval 2

    assert_eq!(engine.run_pass(TWO_INT_REGS), 3);
    assert_eq!(by_id(&engine, 0).assigned_reg, -1);
    assert_eq!(by_id(&engine, 0).spill_offset, 0);
    assert_eq!(by_id(&engine, 1).assigned_reg, 1);
    assert_eq!(by_id(&engine, 2).assigned_reg, 0);
    assert_eq!(by_id(&engine, 3).assigned_reg, 0);
    assert_eq!(engine.next_spill_offset, 8);
    assert_eq!(conflicting_pairs(&engine.intervals), 0);
}

#[test]
fn test_spill_keeps_active_intervals_when_current_ends_last_or_ties() {
    for cur_end in [200, 100] {
        let mut engine = LinearScanEngine::new();
        engine.add_interval(iv(0, 0, 100));
        engine.add_interval(iv(1, 1, 50));
        engine.add_interval(iv(2, 2, cur_end));

        assert_eq!(engine.run_pass(TWO_INT_REGS), 2);
        assert_eq!(by_id(&engine, 0).assigned_reg, 0);
        assert_eq!(by_id(&engine, 1).assigned_reg, 1);
        assert_eq!(by_id(&engine, 2).assigned_reg, -1, "cur_end={cur_end}");
        assert_eq!(by_id(&engine, 2).spill_offset, 0);
    }
}

#[test]
fn test_spill_never_evicts_fixed_interval() {
    // The fixed interval ends last of all, but the victim must be the furthest-ending
    // *non-fixed* interval.
    let mut engine = LinearScanEngine::new();
    engine.add_interval(fixed(0, 0, 1000, 0));
    engine.add_interval(iv(1, 1, 50));
    engine.add_interval(iv(2, 2, 10));

    assert_eq!(engine.run_pass(TWO_INT_REGS), 2);
    assert_eq!(by_id(&engine, 0).assigned_reg, 0);
    assert_eq!(by_id(&engine, 1).assigned_reg, -1);
    assert_eq!(by_id(&engine, 2).assigned_reg, 1);

    // Only fixed intervals are active: the current interval is the only candidate.
    engine.reset();
    engine.add_interval(fixed(0, 0, 1000, 0));
    engine.add_interval(fixed(1, 0, 1000, 1));
    engine.add_interval(iv(2, 2, 10));
    assert_eq!(engine.run_pass(TWO_INT_REGS), 2);
    assert_eq!(by_id(&engine, 2).assigned_reg, -1);
    assert_eq!(by_id(&engine, 0).assigned_reg, 0);
    assert_eq!(by_id(&engine, 1).assigned_reg, 1);
}

#[test]
fn test_spill_only_considers_the_same_register_class() {
    let mut engine = LinearScanEngine::new();
    engine.add_interval(Interval {
        reg_type: 1,
        ..iv(0, 0, 1000)
    });
    engine.add_interval(iv(1, 1, 50));
    engine.add_interval(iv(2, 2, 10));
    let allocated = engine.run_pass(RegisterMasksNative {
        int_available: 0b1,
        vec_available: 0b1,
        registers_count: 1,
    });
    assert_eq!(allocated, 2);
    assert_eq!(
        by_id(&engine, 0).assigned_reg,
        0,
        "vector interval untouched"
    );
    assert_eq!(by_id(&engine, 1).assigned_reg, -1);
    assert_eq!(by_id(&engine, 2).assigned_reg, 0);
}

#[test]
fn test_unsorted_input_gives_same_result_as_stably_sorted_input() {
    let masks = RegisterMasksNative {
        int_available: 0xFFFF & !((1 << 4) | (1 << 5)),
        vec_available: 0xFFFF,
        registers_count: 16,
    };
    // A second engine is reused across all cases, so the scratch left by one pass must not
    // leak into the next.
    let mut unsorted_engine = LinearScanEngine::new();
    for &n in &[1usize, 2, 3, 64, 512, 2048] {
        for seed in 1..=8u32 {
            let mut shuffled = random_intervals(n, seed.wrapping_mul(0x9E37_79B9) | 1, true);
            // Give many intervals the same start so that tie order matters.
            for iv in &mut shuffled {
                let len = iv.end - iv.start;
                iv.start &= !0x1F;
                iv.end = iv.start + len;
                iv.is_fixed = false;
                iv.fixed_reg = -1;
            }
            let mut sorted = shuffled.clone();
            sorted.sort_by_key(|iv| iv.start); // stable reference order

            let mut sorted_engine = LinearScanEngine::new();
            for &iv in &sorted {
                sorted_engine.add_interval(iv);
            }
            unsorted_engine.reset();
            for &iv in &shuffled {
                unsorted_engine.add_interval(iv);
            }

            let a = sorted_engine.run_pass(masks);
            let b = unsorted_engine.run_pass(masks);
            assert_eq!(a, b, "n={n} seed={seed}");
            assert_eq!(
                sorted_engine.intervals, unsorted_engine.intervals,
                "n={n} seed={seed}"
            );
            assert_eq!(
                sorted_engine.next_spill_offset,
                unsorted_engine.next_spill_offset
            );
            assert_eq!(conflicting_pairs(&unsorted_engine.intervals), 0);
        }
    }
}

#[test]
fn test_version_export_returns_constant() {
    assert_eq!(ryu_jit_regalloc_version(), RYU_JIT_REGALLOC_VERSION);
    assert_eq!(RYU_JIT_REGALLOC_VERSION, 0x0001_0000);
}

#[test]
fn test_c_abi_null_pointers() {
    use std::ptr::{null, null_mut};
    let mut words = [1u64, 2, 3, 4, 5];
    let src = [0xF0u64; 5];
    // SAFETY: every pointer is either null or a valid local of the stated length.
    unsafe {
        ryu_regalloc_destroy(null_mut());
        ryu_regalloc_reset(null_mut());
        assert_eq!(ryu_regalloc_add_interval(null_mut(), 1, 0, 5, 0, 0, -1), 0);
        assert_eq!(ryu_regalloc_run_pass(null_mut(), 0xFF, 0xFF, 16), 0);
        assert_eq!(
            ryu_regalloc_get_fixed_conflict(null(), null_mut(), null_mut(), null_mut(), null_mut()),
            0
        );
        assert_eq!(
            ryu_regalloc_get_result(null(), 0, null_mut(), null_mut(), null_mut()),
            0
        );
        assert_eq!(ryu_regalloc_get_highest_index(null(), 0), 0);
        assert_eq!(ryu_regalloc_get_highest_index(null(), 8), 0);
        assert_eq!(ryu_regalloc_bitset_or(null_mut(), src.as_ptr(), 5), 0);
        assert_eq!(ryu_regalloc_bitset_or(words.as_mut_ptr(), null(), 5), 0);
        assert_eq!(ryu_regalloc_bitset_and_not(null_mut(), src.as_ptr(), 5), 0);
        assert_eq!(
            ryu_regalloc_bitset_and_not(words.as_mut_ptr(), null(), 5),
            0
        );
        assert_eq!(words, [1, 2, 3, 4, 5]);

        // Null outputs on a live context are optional, not errors.
        let ctx = ryu_regalloc_create();
        ryu_regalloc_add_interval(ctx, 1, 0, 5, 0, 0, -1);
        ryu_regalloc_run_pass(ctx, 0xFF, 0xFF, 16);
        assert_eq!(
            ryu_regalloc_get_result(ctx, 0, null_mut(), null_mut(), null_mut()),
            1
        );
        ryu_regalloc_destroy(ctx);
    }
}

#[test]
fn test_c_abi_bitset_same_buffer() {
    let mut words = [1u64, 0, 3, 4, 5];
    // SAFETY: `dst` and `src` are the same valid 5-word buffer, which the API allows.
    unsafe {
        let p = words.as_mut_ptr();
        assert_eq!(ryu_regalloc_bitset_or(p, p, 5), 0);
        assert_eq!(words, [1, 0, 3, 4, 5]);
        assert_eq!(ryu_regalloc_bitset_and_not(p, p, 5), 1);
        assert_eq!(words, [0; 5]);
        assert_eq!(ryu_regalloc_bitset_and_not(p, p, 5), 0);
    }
}
