use ryu_interval_tree::*;

#[test]
fn test_version() {
    assert_eq!(ryu_interval_tree_version(), 0x0001_0000);
}

#[test]
fn test_basic_crud() {
    let mut tree = FlatIntervalTree::new();
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());

    tree.insert(100, 200, 1);
    tree.insert(150, 250, 2);
    tree.insert(50, 80, 3);
    assert_eq!(tree.len(), 3);

    let mut res = Vec::new();
    tree.find_overlaps(120, 180, &mut res);
    res.sort();
    assert_eq!(res, vec![1, 2]);

    assert!(tree.remove(150, 250, 2));
    assert_eq!(tree.len(), 2);

    res.clear();
    tree.find_overlaps(120, 180, &mut res);
    assert_eq!(res, vec![1]);

    tree.clear();
    assert_eq!(tree.len(), 0);
}

#[test]
fn test_boundary_overlaps() {
    let mut tree = FlatIntervalTree::new();
    tree.insert(100, 200, 10);

    let mut res = Vec::new();

    // Exactly before
    tree.find_overlaps(0, 100, &mut res);
    assert!(res.is_empty());

    // Exactly after
    tree.find_overlaps(200, 300, &mut res);
    assert!(res.is_empty());

    // Touch left boundary
    tree.find_overlaps(90, 101, &mut res);
    assert_eq!(res, vec![10]);
    res.clear();

    // Touch right boundary
    tree.find_overlaps(199, 210, &mut res);
    assert_eq!(res, vec![10]);
    res.clear();

    // Completely inside
    tree.find_overlaps(120, 150, &mut res);
    assert_eq!(res, vec![10]);
    res.clear();

    // Completely enclosing
    tree.find_overlaps(50, 250, &mut res);
    assert_eq!(res, vec![10]);
}

#[test]
fn test_find_overlaps_slice() {
    let mut tree = FlatIntervalTree::new();
    for i in 0..16 {
        tree.insert(i * 10, i * 10 + 15, i);
    }

    let mut buf = [0u64; 32];
    let count = tree.find_overlaps_slice(12, 35, &mut buf);
    assert!(count >= 3);
}

#[test]
fn test_dual_differential_verification_against_scalar() {
    let mut tree = FlatIntervalTree::new();
    let mut rng = 987654321u64;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    // Insert 500 intervals
    for id in 0..500 {
        let start = (next() % 50000) * 10;
        let length = (next() % 1000) + 10;
        let end = start + length;
        tree.insert(start, end, id);
    }

    // Run 1,000 queries comparing AVX2 and scalar
    for _ in 0..1000 {
        let q_start = (next() % 55000) * 10;
        let q_len = (next() % 2000) + 1;
        let q_end = q_start + q_len;

        let mut scalar_res = Vec::new();
        tree.find_overlaps_scalar(q_start, q_end, &mut scalar_res);

        let mut simd_res = Vec::new();
        tree.find_overlaps(q_start, q_end, &mut simd_res);

        assert_eq!(
            simd_res, scalar_res,
            "Mismatch at query range [{}, {})",
            q_start, q_end
        );
    }
}

#[test]
fn test_ffi_c_abi() {
    // SAFETY: `tree` comes from `_create`, is used only on this thread and is
    // destroyed once; `out` is a live 8-element buffer.
    unsafe {
        let tree = ryu_interval_tree_create();
        assert!(!tree.is_null());
        assert_eq!(ryu_interval_tree_len(tree), 0);

        ryu_interval_tree_insert(tree, 1000, 2000, 42);
        ryu_interval_tree_insert(tree, 1500, 2500, 43);
        assert_eq!(ryu_interval_tree_len(tree), 2);

        let mut out = [0u64; 8];
        let found = ryu_interval_tree_find_overlaps(tree, 1200, 1800, out.as_mut_ptr(), 8);
        assert_eq!(found, 2);
        assert!(out[..2].contains(&42));
        assert!(out[..2].contains(&43));

        assert_eq!(ryu_interval_tree_remove(tree, 1000, 2000, 42), 1);
        assert_eq!(ryu_interval_tree_len(tree), 1);

        ryu_interval_tree_destroy(tree);
    }
}

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Reference result: every stored interval that overlaps the query, in the
/// documented `(start, end, value)` order.
fn model_overlaps(model: &[(u64, u64, u64)], q_start: u64, q_end: u64) -> Vec<u64> {
    if q_start >= q_end {
        return Vec::new();
    }
    let mut hits: Vec<_> = model
        .iter()
        .copied()
        .filter(|&(start, end, _)| start < q_end && q_start < end)
        .collect();
    hits.sort_unstable();
    hits.into_iter().map(|(_, _, value)| value).collect()
}

/// Runs `ops` mixed operations against the tree and a naive `Vec` model.
/// `coord` bounds the coordinates, `values` the number of distinct values;
/// small numbers give many equal starts and exact duplicate triples.
fn run_differential(seed: u64, ops: usize, coord: u64, max_len: u64, values: u64) {
    let mut rng = seed;
    let mut tree = FlatIntervalTree::new();
    let mut model: Vec<(u64, u64, u64)> = Vec::new();
    let mut out = Vec::new();
    let mut queries = 0;
    let mut hits = 0;
    let mut peak = 0;

    for op in 0..ops {
        let roll = xorshift(&mut rng) % 100;
        if xorshift(&mut rng).is_multiple_of(4000) {
            tree.clear();
            model.clear();
        } else if roll < 45 {
            let start = xorshift(&mut rng) % coord;
            let end = match xorshift(&mut rng) % 20 {
                0 => start,                                              // empty interval
                1 => start.saturating_sub(xorshift(&mut rng) % max_len), // inverted
                _ => start.saturating_add(1 + xorshift(&mut rng) % max_len),
            };
            let value = xorshift(&mut rng) % values;
            tree.insert(start, end, value);
            model.push((start, end, value));
        } else if roll < 62 {
            if !model.is_empty() {
                let pick = (xorshift(&mut rng) % model.len() as u64) as usize;
                let (start, end, value) = model.swap_remove(pick);
                assert!(
                    tree.remove(start, end, value),
                    "op {op}: remove of a live entry"
                );
            }
        } else if roll < 67 {
            // Mostly absent keys; the model decides what the answer must be.
            let start = xorshift(&mut rng) % coord;
            let end = start.saturating_add(1 + xorshift(&mut rng) % max_len);
            let value = xorshift(&mut rng) % values;
            let pos = model.iter().position(|&e| e == (start, end, value));
            if let Some(pos) = pos {
                model.swap_remove(pos);
            }
            assert_eq!(tree.remove(start, end, value), pos.is_some(), "op {op}");
        } else {
            let q_start = xorshift(&mut rng) % coord;
            let q_end = match xorshift(&mut rng) % 16 {
                0 => q_start,                   // empty query
                1 => q_start.saturating_sub(1), // inverted query
                2 => u64::MAX,                  // everything to the right
                _ => q_start.saturating_add(1 + xorshift(&mut rng) % max_len.saturating_mul(4)),
            };
            let q_start = if xorshift(&mut rng).is_multiple_of(16) {
                0
            } else {
                q_start
            };
            let expected = model_overlaps(&model, q_start, q_end);

            out.clear();
            tree.find_overlaps(q_start, q_end, &mut out);
            assert_eq!(out, expected, "op {op}: find_overlaps [{q_start}, {q_end})");

            out.clear();
            tree.find_overlaps_scalar(q_start, q_end, &mut out);
            assert_eq!(out, expected, "op {op}: find_overlaps_scalar");

            // The slice variant reports the full count but stores only what fits.
            let mut buf = [u64::MAX; 6];
            let count = tree.find_overlaps_slice(q_start, q_end, &mut buf);
            assert_eq!(count, expected.len(), "op {op}: find_overlaps_slice count");
            let stored = count.min(buf.len());
            assert_eq!(
                &buf[..stored],
                &expected[..stored],
                "op {op}: slice contents"
            );
            assert!(buf[stored..].iter().all(|&v| v == u64::MAX));
            queries += 1;
            hits += expected.len();
        }
        assert_eq!(tree.len(), model.len(), "op {op}: len");
        assert_eq!(tree.is_empty(), model.is_empty());
        peak = peak.max(model.len());
    }
    // Guard against a vacuous run.
    assert!(queries > ops / 5);
    assert!(hits > queries, "only {hits} hits in {queries} queries");
    assert!(peak > 500, "model peaked at {peak} entries");
    eprintln!("seed {seed:#x}: {queries} queries, {hits} hits, peak {peak} entries");

    // Drain: every remaining entry must still be removable exactly once.
    let all = model_overlaps(&model, 0, u64::MAX);
    out.clear();
    tree.find_overlaps(0, u64::MAX, &mut out);
    assert_eq!(out, all);
    for (start, end, value) in model.drain(..) {
        assert!(tree.remove(start, end, value));
    }
    assert!(tree.is_empty());
    assert!(!tree.remove(0, 1, 0));
}

#[test]
fn test_differential_against_naive_model_dense() {
    // Tiny coordinate space: equal starts, identical ranges, exact duplicates.
    run_differential(0x1234_5678_9ABC_DEF1, 20_000, 48, 12, 3);
}

#[test]
fn test_differential_against_naive_model_sparse() {
    // Cache-like layout: page-aligned-ish addresses, unique-ish values.
    run_differential(0x0DDB_1A5E_5BAD_5EED, 20_000, 1 << 20, 1 << 14, 1 << 32);
}

#[test]
fn test_differential_against_naive_model_extreme_coordinates() {
    // Coordinates near u64::MAX exercise the saturating / max_end edge cases.
    run_differential(0xC0FF_EE00_DEAD_BEEF, 10_000, u64::MAX, u64::MAX / 3, 8);
}

#[test]
fn test_result_order_and_duplicates() {
    let mut tree = FlatIntervalTree::new();
    tree.insert(30, 40, 3);
    tree.insert(10, 50, 9);
    tree.insert(10, 20, 7);
    tree.insert(10, 20, 2);
    tree.insert(10, 20, 7); // exact duplicate
    tree.insert(20, 25, 1);
    assert_eq!(tree.len(), 6);

    let mut res = Vec::new();
    tree.find_overlaps(0, 100, &mut res);
    // Ascending start, then end, then value.
    assert_eq!(res, vec![2, 7, 7, 9, 1, 3]);

    // Each remove takes out one copy of a duplicate.
    assert!(tree.remove(10, 20, 7));
    res.clear();
    tree.find_overlaps(0, 100, &mut res);
    assert_eq!(res, vec![2, 7, 9, 1, 3]);
    assert!(tree.remove(10, 20, 7));
    assert!(!tree.remove(10, 20, 7));
    // Same range, different value is a different entry.
    assert!(!tree.remove(10, 50, 8));
    assert_eq!(tree.len(), 4);
}

#[test]
fn test_ffi_count_only_and_truncation() {
    // SAFETY: `tree` comes from `_create`, is used only on this thread and is
    // destroyed once; `out` is a live buffer of `out.len()` elements.
    unsafe {
        let tree = ryu_interval_tree_create();
        for i in 0..10u64 {
            ryu_interval_tree_insert(tree, i * 10, i * 10 + 25, i);
        }

        // Null buffer: count only.
        let total = ryu_interval_tree_find_overlaps(tree, 0, 1000, std::ptr::null_mut(), 0);
        assert_eq!(total, 10);

        // Short buffer: full count returned, first entries stored in start order.
        let mut out = [u64::MAX; 4];
        let found = ryu_interval_tree_find_overlaps(tree, 0, 1000, out.as_mut_ptr(), out.len());
        assert_eq!(found, 10);
        assert_eq!(out, [0, 1, 2, 3]);

        assert_eq!(ryu_interval_tree_remove(tree, 0, 25, 0), 1);
        assert_eq!(ryu_interval_tree_remove(tree, 0, 25, 0), 0);
        assert_eq!(ryu_interval_tree_remove(std::ptr::null_mut(), 0, 25, 0), -1);

        ryu_interval_tree_clear(tree);
        assert_eq!(ryu_interval_tree_len(tree), 0);
        ryu_interval_tree_destroy(tree);
    }
}

#[test]
fn test_ffi_null_pointers() {
    let null_mut: *mut FlatIntervalTree = std::ptr::null_mut();
    let null: *const FlatIntervalTree = std::ptr::null();
    let mut out = [7u64; 4];
    // SAFETY: every function documents null `tree` as accepted; `out` is a live
    // 4-element buffer and the null `out_buf` call passes `max_out = 0`.
    unsafe {
        ryu_interval_tree_destroy(null_mut);
        ryu_interval_tree_clear(null_mut);
        ryu_interval_tree_insert(null_mut, 0, 10, 1);
        assert_eq!(ryu_interval_tree_remove(null_mut, 0, 10, 1), -1);
        assert_eq!(
            ryu_interval_tree_find_overlaps(null, 0, 10, out.as_mut_ptr(), 4),
            0
        );
        assert_eq!(
            ryu_interval_tree_find_overlaps(null, 0, 10, std::ptr::null_mut(), 0),
            0
        );
        assert_eq!(ryu_interval_tree_len(null), 0);
    }
    assert_eq!(out, [7; 4]);

    // Null output buffer on a live tree counts without writing, even when
    // `max_out` is non-zero.
    let tree = ryu_interval_tree_create();
    // SAFETY: `tree` is a live handle used only here and destroyed once.
    unsafe {
        ryu_interval_tree_insert(tree, 0, 10, 1);
        ryu_interval_tree_insert(tree, 5, 15, 2);
        assert_eq!(
            ryu_interval_tree_find_overlaps(tree, 0, 20, std::ptr::null_mut(), 0),
            2
        );
        assert_eq!(
            ryu_interval_tree_find_overlaps(tree, 0, 20, std::ptr::null_mut(), 16),
            2
        );
        ryu_interval_tree_destroy(tree);
    }
}

#[test]
fn test_version_constant() {
    assert_eq!(ryu_interval_tree_version(), RYU_INTERVAL_TREE_VERSION);
    assert_eq!(RYU_INTERVAL_TREE_VERSION, 0x0001_0000);
}
