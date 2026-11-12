use ryu_maxwell_state::*;

#[test]
fn test_register_mapping_and_dirty() {
    let mut tracker = MaxwellStateTracker::new();
    tracker.map_register(0x10, GROUP_VERTEX_BUFFER);
    tracker.map_register(0x14, GROUP_VERTEX_BUFFER);
    tracker.map_register(0x20, GROUP_BLEND_STATE);

    assert!(!tracker.is_dirty(GROUP_VERTEX_BUFFER));
    assert!(!tracker.is_dirty(GROUP_BLEND_STATE));

    tracker.set_dirty(0x10);
    assert!(tracker.is_dirty(GROUP_VERTEX_BUFFER));
    assert!(!tracker.is_dirty(GROUP_BLEND_STATE));

    tracker.set_dirty(0x20);
    assert!(tracker.is_dirty(GROUP_BLEND_STATE));

    // Clear
    tracker.clear_dirty(1 << GROUP_VERTEX_BUFFER);
    assert!(!tracker.is_dirty(GROUP_VERTEX_BUFFER));
    assert!(tracker.is_dirty(GROUP_BLEND_STATE));
}

#[test]
fn test_scan_changes_avx2_and_scalar() {
    let mut tracker = MaxwellStateTracker::new();
    tracker.map_register(0x00, 0);
    tracker.map_register(0x04, 1);
    tracker.map_register(0x20, 2); // In chunk 1 (offset 32)
    tracker.map_register(0x100, 3); // In chunk 8

    let mut current = vec![0u8; MAXWELL_STATE_BYTES];
    let shadow = vec![0u8; MAXWELL_STATE_BYTES];

    // Initially identical
    let mask = tracker.scan_changes(&current, &shadow);
    assert_eq!(mask, 0);

    // Modify offset 0x20 (group 2)
    current[0x20] = 0x42;
    let mask = tracker.scan_changes(&current, &shadow);
    assert_eq!(mask, 1 << 2);

    // Modify offset 0x100 (group 3)
    current[0x100] = 0x99;
    let mask = tracker.scan_changes(&current, &shadow);
    assert_eq!(mask, (1 << 2) | (1 << 3));

    // Scalar explicit test
    let mut tracker2 = MaxwellStateTracker::new();
    tracker2.map_register(0x20, 2);
    tracker2.map_register(0x100, 3);
    let mask_scalar = tracker2.scan_changes_scalar(&current, &shadow);
    assert_eq!(mask_scalar, (1 << 2) | (1 << 3));
}

#[test]
fn test_collect_dirty_groups() {
    let mut tracker = MaxwellStateTracker::new();
    tracker.force_dirty(1);
    tracker.force_dirty(5);
    tracker.force_dirty(10);

    let mut out = [0u32; 16];
    let count = tracker.collect_dirty_groups(0xFFFF, &mut out);
    assert_eq!(count, 3);
    assert_eq!(out[0], 1);
    assert_eq!(out[1], 5);
    assert_eq!(out[2], 10);

    assert_eq!(tracker.get_dirty_mask(), 0);
}

#[test]
fn test_pipeline_hasher_and_equality() {
    let d1 = [
        1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32,
    ];
    let mut d2 = d1;
    let h1 = pipeline_state_hash64(&d1);
    let h2 = pipeline_state_hash64(&d2);
    assert_eq!(h1, h2);
    assert!(pipeline_state_equals(&d1, &d2));

    d2[15] = 0xFF;
    let h3 = pipeline_state_hash64(&d2);
    assert_ne!(h1, h3);
    assert!(!pipeline_state_equals(&d1, &d2));
}

#[test]
fn test_c_abi_lifecycle() {
    assert_eq!(ryu_maxwell_state_version(), 0x00010000);

    let handle = ryu_maxwell_tracker_create();
    assert!(!handle.is_null());

    // SAFETY: `handle` is live until the final destroy; buffers are live locals
    // passed with their own lengths.
    unsafe {
        ryu_maxwell_tracker_map_register(handle, 0x10, 5);
        ryu_maxwell_tracker_set_dirty(handle, 0x10);

        let mask = ryu_maxwell_tracker_get_dirty_mask(handle);
        assert_eq!(mask, 1 << 5);

        let mut out = [0u32; 4];
        let cnt = ryu_maxwell_tracker_collect_dirty(handle, 0xFF, out.as_mut_ptr(), 4);
        assert_eq!(cnt, 1);
        assert_eq!(out[0], 5);

        let buf1 = [10u8; 64];
        let buf2 = [10u8; 64];
        assert_eq!(
            ryu_maxwell_pipeline_equals(buf1.as_ptr(), buf2.as_ptr(), 64),
            1
        );

        ryu_maxwell_tracker_destroy(handle);
    }
}
