use ryu_motion_fusion::*;

#[test]
fn test_version() {
    assert_eq!(ryu_motion_fusion_version(), 0x0001_0000);
}

#[test]
fn test_identity_and_matrix() {
    let filter = MahonySensorFilter::new(0.001, 1.0, 0.0);
    assert_eq!(filter.quaternion, Quaternion::identity());

    let m = filter.quaternion.to_rotation_matrix_3x3();
    // 3x3 identity matrix
    assert_eq!(m, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_quaternion_unit_norm_invariant() {
    let mut filter = MahonySensorFilter::new(0.001, 1.0, 0.0);

    // Simulate 10,000 1,000 Hz sensor steps with dynamic movement
    for i in 0..10_000 {
        let t = i as f32 * 0.001;
        let ax = (t * 2.0).sin();
        let ay = (t * 3.0).cos();
        let az = 1.0;
        let gx = (t * 5.0).sin() * 0.1;
        let gy = (t * 4.0).cos() * 0.1;
        let gz = 0.05;

        filter.update(Vector3::new(ax, ay, az), Vector3::new(gx, gy, gz));

        let norm = filter.quaternion.length();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "Norm deviation at step {}: {}",
            i,
            norm
        );
    }
}

#[test]
fn test_rotation_matrix_orthogonal_invariant() {
    let mut filter = MahonySensorFilter::new(0.001, 1.0, 0.0);

    filter.update(Vector3::new(0.1, 0.2, 0.98), Vector3::new(0.5, 0.2, -0.3));

    let m = filter.quaternion.to_rotation_matrix_3x3();

    // Row norms should be 1.0
    let r1 = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
    let r2 = (m[3] * m[3] + m[4] * m[4] + m[5] * m[5]).sqrt();
    let r3 = (m[6] * m[6] + m[7] * m[7] + m[8] * m[8]).sqrt();

    assert!((r1 - 1.0).abs() < 1e-4);
    assert!((r2 - 1.0).abs() < 1e-4);
    assert!((r3 - 1.0).abs() < 1e-4);
}

#[test]
fn test_stationary_gravity_convergence() {
    let mut filter = MahonySensorFilter::new(0.002, 2.0, 0.05);

    // Pure gravity pointing along +Z axis for 1,000 steps
    for _ in 0..1000 {
        filter.update(Vector3::new(0.0, 0.0, 1.0), Vector3::zero());
    }

    // Gravity should point along Z in estimated frame
    let m = filter.quaternion.to_rotation_matrix_3x3();
    assert!((m[8] - 1.0).abs() < 0.05); // M33 close to 1
}

#[test]
fn test_c_abi_lifecycle() {
    unsafe {
        let filter = ryu_motion_filter_create(0.001, 1.0, 0.0);
        assert!(!filter.is_null());

        ryu_motion_filter_update(filter, 0.0, 0.0, 1.0, 0.01, 0.0, 0.0);

        let mut q = [0.0f32; 4];
        ryu_motion_filter_get_quaternion(filter, q.as_mut_ptr());
        let norm = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        assert!((norm - 1.0).abs() < 1e-5);

        let mut m = [0.0f32; 9];
        ryu_motion_filter_get_orientation_matrix(filter, m.as_mut_ptr());
        assert!((m[0] - 1.0).abs() < 0.01);

        ryu_motion_filter_reset(filter);
        ryu_motion_filter_get_quaternion(filter, q.as_mut_ptr());
        assert_eq!(q, [0.0, 0.0, 0.0, 1.0]);

        ryu_motion_filter_destroy(filter);
    }
}
