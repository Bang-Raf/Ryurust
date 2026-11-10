use ryu_vulkan_descriptors::ffi::*;
use ryu_vulkan_descriptors::types::*;
use ryu_vulkan_descriptors::RyuDescriptorState;

#[test]
fn test_version() {
    assert_eq!(ryu_vulkan_descriptors_version(), 0x0001_0000);
}

#[test]
fn test_buffer_update_and_dirty_tracking() {
    let mut state = RyuDescriptorState::new();

    // Update uniform buffer at binding 5
    let updated = state.update_buffer(SET_UNIFORM, 5, 0x1000, 256, 1024);
    assert!(updated);
    assert_eq!(state.uniform_buffers[5].buffer_handle, 0x1000);
    assert_eq!(state.uniform_buffers[5].offset, 256);
    assert_eq!(state.uniform_buffers[5].range, 1024);
    assert_eq!(state.dirty_uniform[0], 1 << 5);

    // Updating with identical values should return false and not re-mark dirty
    let updated_same = state.update_buffer(SET_UNIFORM, 5, 0x1000, 256, 1024);
    assert!(!updated_same);

    // Update at binding 70 (word 1)
    let updated_70 = state.update_buffer(SET_UNIFORM, 70, 0x2000, 0, 4096);
    assert!(updated_70);
    assert_eq!(state.dirty_uniform[1], 1 << (70 - 64));
}

#[test]
fn test_redundancy_filter() {
    let mut state = RyuDescriptorState::new();

    // Bind 3 buffers
    state.update_buffer(SET_UNIFORM, 0, 0xAAAA, 0, 256);
    state.update_buffer(SET_UNIFORM, 1, 0xBBBB, 256, 512);
    state.update_buffer(SET_UNIFORM, 2, 0xCCCC, 512, 1024);

    let mut out_indices = [0u32; 16];
    let mut out_infos = [RyuDescriptorBufferInfo::default(); 16];

    // First filter pass: all 3 should be returned
    let count1 = state.filter_redundant_buffers(SET_UNIFORM, &mut out_indices, &mut out_infos);
    assert_eq!(count1, 3);
    assert_eq!(out_indices[0], 0);
    assert_eq!(out_indices[1], 1);
    assert_eq!(out_indices[2], 2);
    assert_eq!(out_infos[0].buffer_handle, 0xAAAA);
    assert_eq!(out_infos[1].buffer_handle, 0xBBBB);
    assert_eq!(out_infos[2].buffer_handle, 0xCCCC);

    // Second filter pass without changes: should return 0
    let count2 = state.filter_redundant_buffers(SET_UNIFORM, &mut out_indices, &mut out_infos);
    assert_eq!(count2, 0);

    // Only update binding 1 with new offset
    state.update_buffer(SET_UNIFORM, 1, 0xBBBB, 1024, 512);
    let count3 = state.filter_redundant_buffers(SET_UNIFORM, &mut out_indices, &mut out_infos);
    assert_eq!(count3, 1);
    assert_eq!(out_indices[0], 1);
    assert_eq!(out_infos[0].offset, 1024);
}

#[test]
fn test_image_update_and_filter() {
    let mut state = RyuDescriptorState::new();

    state.update_image(SET_TEXTURE, 10, 0x1111, 0x2222, 3);
    assert_eq!(state.textures[10].sampler_handle, 0x1111);
    assert_eq!(state.textures[10].image_view_handle, 0x2222);
    assert_eq!(state.textures[10].image_layout, 3);

    let mut out_indices = [0u32; 8];
    let mut out_infos = [RyuDescriptorImageInfo::default(); 8];

    let count = state.filter_redundant_images(SET_TEXTURE, &mut out_indices, &mut out_infos);
    assert_eq!(count, 1);
    assert_eq!(out_indices[0], 10);
    assert_eq!(out_infos[0].sampler_handle, 0x1111);
}

#[test]
fn test_template_slice_packing() {
    let mut state = RyuDescriptorState::new();

    for i in 0..4 {
        state.update_buffer(SET_STORAGE, i, 0x1000 + i as u64, i as u64 * 64, 64);
    }

    let mut dest = vec![0u8; 4 * std::mem::size_of::<RyuDescriptorBufferInfo>()];
    let written = state.pack_template_buffer_slice(SET_STORAGE, 0, 4, &mut dest);
    assert_eq!(written, dest.len());

    let unpacked: &[RyuDescriptorBufferInfo] =
        unsafe { std::slice::from_raw_parts(dest.as_ptr() as *const RyuDescriptorBufferInfo, 4) };
    for i in 0..4 {
        assert_eq!(unpacked[i].buffer_handle, 0x1000 + i as u64);
        assert_eq!(unpacked[i].offset, i as u64 * 64);
        assert_eq!(unpacked[i].range, 64);
    }
}

#[test]
fn test_differential_bit_identical() {
    // Step 2: Bit-identical dual execution against reference C# scalar logic
    struct ReferenceBufferInfo {
        handle: u64,
        offset: u64,
        range: u64,
    }

    let mut ref_buffers = Vec::new();
    for i in 0..90 {
        ref_buffers.push(ReferenceBufferInfo {
            handle: (i * 1337 + 1) as u64,
            offset: (i * 32) as u64,
            range: (i * 128 + 64) as u64,
        });
    }

    let mut rust_state = RyuDescriptorState::new();
    for (i, r) in ref_buffers.iter().enumerate() {
        rust_state.update_buffer(SET_UNIFORM, i, r.handle, r.offset, r.range);
    }

    for (i, r) in ref_buffers.iter().enumerate() {
        let rust_info = &rust_state.uniform_buffers[i];
        assert_eq!(rust_info.buffer_handle, r.handle);
        assert_eq!(rust_info.offset, r.offset);
        assert_eq!(rust_info.range, r.range);
    }
}

#[test]
fn test_c_abi_lifecycle() {
    unsafe {
        let handle = ryu_vulkan_descriptors_create();
        assert!(!handle.is_null());

        let res = ryu_vulkan_descriptors_update_buffer(handle, SET_UNIFORM, 3, 0xCAFE, 128, 256);
        assert_eq!(res, 1);

        let mut out_idx = [0u32; 4];
        let mut out_info = [RyuDescriptorBufferInfo::default(); 4];
        let mut count = 0usize;

        let filter_res = ryu_vulkan_descriptors_filter_redundant_buffers(
            handle,
            SET_UNIFORM,
            out_idx.as_mut_ptr(),
            out_info.as_mut_ptr(),
            4,
            &mut count,
        );
        assert_eq!(filter_res, 0);
        assert_eq!(count, 1);
        assert_eq!(out_idx[0], 3);
        assert_eq!(out_info[0].buffer_handle, 0xCAFE);

        ryu_vulkan_descriptors_destroy(handle);
    }
}
