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

#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_destroy(filter: *mut MahonySensorFilter) {
    if !filter.is_null() {
        drop(Box::from_raw(filter));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_reset(filter: *mut MahonySensorFilter) {
    if let Some(f) = filter.as_mut() {
        f.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_motion_filter_set_sample_period(
    filter: *mut MahonySensorFilter,
    period: f32,
) {
    if let Some(f) = filter.as_mut() {
        f.sample_period = period;
    }
}

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
