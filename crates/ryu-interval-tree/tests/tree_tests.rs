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
