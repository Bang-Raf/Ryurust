use crate::filter::*;
use std::slice;

pub const RYU_MOTION_FUSION_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_motion_fusion_version() -> u32 {
    RYU_MOTION_FUSION_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_motion_filter_create(
    sample_period: f32,
    kp: f32,
    ki: f32,
) -> *mut MahonySensorFilter {
    Box::into_raw(Box::new(MahonySensorFilter::new(sample_period, kp, ki)))
}

/// # Safety
/// `filter` must be null or a handle from `ryu_motion_filter_create` that has not been
/// destroyed yet; it must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_destroy(filter: *mut MahonySensorFilter) {
    if !filter.is_null() {
        drop(Box::from_raw(filter));
    }
}

/// # Safety
/// `filter` must be null (no-op) or a live handle from `ryu_motion_filter_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_reset(filter: *mut MahonySensorFilter) {
    if let Some(f) = filter.as_mut() {
        f.reset();
    }
}

/// # Safety
/// `filter` must be null (no-op) or a live handle from `ryu_motion_filter_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_set_sample_period(
    filter: *mut MahonySensorFilter,
    period: f32,
) {
    if let Some(f) = filter.as_mut() {
        f.sample_period = period;
    }
}

/// # Safety
/// `filter` must be null (no-op) or a live handle from `ryu_motion_filter_create`, not
/// accessed concurrently by any other call.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_update(
    filter: *mut MahonySensorFilter,
    ax: f32,
    ay: f32,
    az: f32,
    gx: f32,
    gy: f32,
    gz: f32,
) {
    if let Some(f) = filter.as_mut() {
        f.update(Vector3::new(ax, ay, az), Vector3::new(gx, gy, gz));
    }
}

/// Does nothing when either pointer is null.
///
/// # Safety
/// `filter` must be null or a live handle with no concurrent mutating call; `out_xyzw`
/// must be null or valid for writes of 4 `f32`s.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_get_quaternion(
    filter: *const MahonySensorFilter,
    out_xyzw: *mut f32,
) {
    if let Some(f) = filter.as_ref() {
        if !out_xyzw.is_null() {
            let slice = slice::from_raw_parts_mut(out_xyzw, 4);
            slice[0] = f.quaternion.x;
            slice[1] = f.quaternion.y;
            slice[2] = f.quaternion.z;
            slice[3] = f.quaternion.w;
        }
    }
}

/// Does nothing when either pointer is null.
///
/// # Safety
/// `filter` must be null or a live handle with no concurrent mutating call;
/// `out_matrix9` must be null or valid for writes of 9 `f32`s.
#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_get_orientation_matrix(
    filter: *const MahonySensorFilter,
    out_matrix9: *mut f32,
) {
    if let Some(f) = filter.as_ref() {
        if !out_matrix9.is_null() {
            let slice = slice::from_raw_parts_mut(out_matrix9, 9);
            let matrix = f.quaternion.to_rotation_matrix_3x3();
            slice.copy_from_slice(&matrix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_motion_fusion_version(), RYU_MOTION_FUSION_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        unsafe {
            ryu_motion_filter_destroy(ptr::null_mut());
            ryu_motion_filter_reset(ptr::null_mut());
            ryu_motion_filter_set_sample_period(ptr::null_mut(), 0.01);
            ryu_motion_filter_update(ptr::null_mut(), 0.0, 0.0, 1.0, 0.0, 0.0, 0.0);

            let mut quat = [7.0f32; 4];
            let mut mat = [7.0f32; 9];
            ryu_motion_filter_get_quaternion(ptr::null(), quat.as_mut_ptr());
            ryu_motion_filter_get_orientation_matrix(ptr::null(), mat.as_mut_ptr());
            assert_eq!(quat, [7.0; 4]);
            assert_eq!(mat, [7.0; 9]);

            let f = ryu_motion_filter_create(0.01, 1.0, 0.0);
            ryu_motion_filter_get_quaternion(f, ptr::null_mut());
            ryu_motion_filter_get_orientation_matrix(f, ptr::null_mut());
            ryu_motion_filter_destroy(f);
        }
    }
}
