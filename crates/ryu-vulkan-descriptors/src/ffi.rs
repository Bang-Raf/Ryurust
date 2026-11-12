use crate::batcher::RyuDescriptorState;
use crate::types::{RyuDescriptorBufferInfo, RyuDescriptorImageInfo};
use std::slice;

pub const RYU_VULKAN_DESCRIPTORS_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_vulkan_descriptors_version() -> u32 {
    RYU_VULKAN_DESCRIPTORS_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_vulkan_descriptors_create() -> *mut RyuDescriptorState {
    Box::into_raw(Box::new(RyuDescriptorState::new()))
}

/// # Safety
/// `state` must be null or a handle from `ryu_vulkan_descriptors_create` that has not
/// been destroyed yet; it must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_destroy(state: *mut RyuDescriptorState) {
    if !state.is_null() {
        drop(Box::from_raw(state));
    }
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_reset(state: *mut RyuDescriptorState) {
    if let Some(s) = state.as_mut() {
        s.reset();
    }
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_update_buffer(
    state: *mut RyuDescriptorState,
    set_type: u32,
    index: u32,
    buffer_handle: u64,
    offset: u64,
    range: u64,
) -> i32 {
    let s = match state.as_mut() {
        Some(v) => v,
        None => return -1,
    };

    if s.update_buffer(set_type, index as usize, buffer_handle, offset, range) {
        1
    } else {
        0
    }
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_update_image(
    state: *mut RyuDescriptorState,
    set_type: u32,
    index: u32,
    sampler_handle: u64,
    image_view_handle: u64,
    image_layout: u32,
) -> i32 {
    let s = match state.as_mut() {
        Some(v) => v,
        None => return -1,
    };

    if s.update_image(
        set_type,
        index as usize,
        sampler_handle,
        image_view_handle,
        image_layout,
    ) {
        1
    } else {
        0
    }
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread. `out_indices` and `out_infos` must each
/// be null or writable for `capacity` elements and must not overlap; `out_count` must
/// be null or writable.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_filter_redundant_buffers(
    state: *mut RyuDescriptorState,
    set_type: u32,
    out_indices: *mut u32,
    out_infos: *mut RyuDescriptorBufferInfo,
    capacity: usize,
    out_count: *mut usize,
) -> i32 {
    let s = match state.as_mut() {
        Some(v) => v,
        None => return -1,
    };

    if out_indices.is_null() || out_infos.is_null() || out_count.is_null() {
        return -2;
    }

    let indices_slice = slice::from_raw_parts_mut(out_indices, capacity);
    let infos_slice = slice::from_raw_parts_mut(out_infos, capacity);

    let count = s.filter_redundant_buffers(set_type, indices_slice, infos_slice);
    *out_count = count;
    0
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread. `out_indices` and `out_infos` must each
/// be null or writable for `capacity` elements and must not overlap; `out_count` must
/// be null or writable.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_filter_redundant_images(
    state: *mut RyuDescriptorState,
    set_type: u32,
    out_indices: *mut u32,
    out_infos: *mut RyuDescriptorImageInfo,
    capacity: usize,
    out_count: *mut usize,
) -> i32 {
    let s = match state.as_mut() {
        Some(v) => v,
        None => return -1,
    };

    if out_indices.is_null() || out_infos.is_null() || out_count.is_null() {
        return -2;
    }

    let indices_slice = slice::from_raw_parts_mut(out_indices, capacity);
    let infos_slice = slice::from_raw_parts_mut(out_infos, capacity);

    let count = s.filter_redundant_images(set_type, indices_slice, infos_slice);
    *out_count = count;
    0
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create` that is
/// not being mutated concurrently. `out_buf` must be null or writable for
/// `out_buf_len` bytes; `out_written` must be null or writable.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_pack_template_slice(
    state: *const RyuDescriptorState,
    set_type: u32,
    binding: u32,
    count: u32,
    out_buf: *mut u8,
    out_buf_len: usize,
    out_written: *mut usize,
) -> i32 {
    let s = match state.as_ref() {
        Some(v) => v,
        None => return -1,
    };

    if out_buf.is_null() || out_written.is_null() {
        return -2;
    }

    let dest = slice::from_raw_parts_mut(out_buf, out_buf_len);
    let bytes = s.pack_template_buffer_slice(set_type, binding as usize, count as usize, dest);
    *out_written = bytes;
    if bytes > 0 {
        0
    } else {
        -3
    }
}

/// # Safety
/// `state` must be null or a live handle from `ryu_vulkan_descriptors_create`, not used
/// concurrently by another thread.
#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_commit_all(
    state: *mut RyuDescriptorState,
    set_type: u32,
) -> i32 {
    let s = match state.as_mut() {
        Some(v) => v,
        None => return -1,
    };

    s.commit_all(set_type);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SET_UNIFORM;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(
            ryu_vulkan_descriptors_version(),
            RYU_VULKAN_DESCRIPTORS_VERSION
        );
        assert_eq!(RYU_VULKAN_DESCRIPTORS_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let mut indices = [0u32; 4];
        let mut buffers = [RyuDescriptorBufferInfo::default(); 4];
        let mut images = [RyuDescriptorImageInfo::default(); 4];
        let mut bytes = [0u8; 96];
        let mut count = 7usize;
        // SAFETY: null handles/buffers are rejected before use; `s` is live until
        // its destroy and the other pointers refer to live locals of the stated length.
        unsafe {
            let null = ptr::null_mut::<RyuDescriptorState>();
            ryu_vulkan_descriptors_destroy(null);
            ryu_vulkan_descriptors_reset(null);
            assert_eq!(
                ryu_vulkan_descriptors_update_buffer(null, SET_UNIFORM, 0, 1, 0, 16),
                -1
            );
            assert_eq!(
                ryu_vulkan_descriptors_update_image(null, SET_UNIFORM, 0, 1, 2, 3),
                -1
            );
            assert_eq!(
                ryu_vulkan_descriptors_filter_redundant_buffers(
                    null,
                    SET_UNIFORM,
                    indices.as_mut_ptr(),
                    buffers.as_mut_ptr(),
                    4,
                    &mut count
                ),
                -1
            );
            assert_eq!(
                ryu_vulkan_descriptors_filter_redundant_images(
                    null,
                    SET_UNIFORM,
                    indices.as_mut_ptr(),
                    images.as_mut_ptr(),
                    4,
                    &mut count
                ),
                -1
            );
            assert_eq!(
                ryu_vulkan_descriptors_pack_template_slice(
                    null,
                    SET_UNIFORM,
                    0,
                    4,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut count
                ),
                -1
            );
            assert_eq!(ryu_vulkan_descriptors_commit_all(null, SET_UNIFORM), -1);

            let s = ryu_vulkan_descriptors_create();
            assert!(!s.is_null());
            let ip = indices.as_mut_ptr();
            let bp = buffers.as_mut_ptr();
            let mp = images.as_mut_ptr();
            let cp: *mut usize = &mut count;
            for (i, b, c) in [
                (ptr::null_mut(), bp, cp),
                (ip, ptr::null_mut(), cp),
                (ip, bp, ptr::null_mut()),
            ] {
                assert_eq!(
                    ryu_vulkan_descriptors_filter_redundant_buffers(s, SET_UNIFORM, i, b, 4, c),
                    -2
                );
            }
            for (i, m, c) in [
                (ptr::null_mut(), mp, cp),
                (ip, ptr::null_mut(), cp),
                (ip, mp, ptr::null_mut()),
            ] {
                assert_eq!(
                    ryu_vulkan_descriptors_filter_redundant_images(s, SET_UNIFORM, i, m, 4, c),
                    -2
                );
            }
            assert_eq!(
                ryu_vulkan_descriptors_pack_template_slice(
                    s,
                    SET_UNIFORM,
                    0,
                    4,
                    ptr::null_mut(),
                    96,
                    cp
                ),
                -2
            );
            assert_eq!(
                ryu_vulkan_descriptors_pack_template_slice(
                    s,
                    SET_UNIFORM,
                    0,
                    4,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    ptr::null_mut()
                ),
                -2
            );
            assert_eq!(count, 7);
            ryu_vulkan_descriptors_destroy(s);
        }
    }
}
