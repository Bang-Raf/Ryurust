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

#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_destroy(state: *mut RyuDescriptorState) {
    if !state.is_null() {
        drop(Box::from_raw(state));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_vulkan_descriptors_reset(state: *mut RyuDescriptorState) {
    if let Some(s) = state.as_mut() {
        s.reset();
    }
}

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
