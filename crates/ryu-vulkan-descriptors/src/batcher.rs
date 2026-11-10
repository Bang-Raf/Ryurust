use crate::types::*;

#[inline(always)]
pub fn buffers_equal(a: &RyuDescriptorBufferInfo, b: &RyuDescriptorBufferInfo) -> bool {
    ((a.buffer_handle ^ b.buffer_handle) | (a.offset ^ b.offset) | (a.range ^ b.range)) == 0
}

#[inline(always)]
pub fn images_equal(a: &RyuDescriptorImageInfo, b: &RyuDescriptorImageInfo) -> bool {
    ((a.sampler_handle ^ b.sampler_handle)
        | (a.image_view_handle ^ b.image_view_handle)
        | ((a.image_layout ^ b.image_layout) as u64))
        == 0
}

#[derive(Debug, Clone)]
pub struct RyuDescriptorState {
    pub uniform_buffers: [RyuDescriptorBufferInfo; MAX_UNIFORM_BUFFER_BINDINGS],
    pub storage_buffers: [RyuDescriptorBufferInfo; MAX_STORAGE_BUFFER_BINDINGS],
    pub textures: [RyuDescriptorImageInfo; MAX_TEXTURE_BINDINGS],
    pub images: [RyuDescriptorImageInfo; MAX_IMAGE_BINDINGS],

    pub committed_uniform: [RyuDescriptorBufferInfo; MAX_UNIFORM_BUFFER_BINDINGS],
    pub committed_storage: [RyuDescriptorBufferInfo; MAX_STORAGE_BUFFER_BINDINGS],
    pub committed_textures: [RyuDescriptorImageInfo; MAX_TEXTURE_BINDINGS],
    pub committed_images: [RyuDescriptorImageInfo; MAX_IMAGE_BINDINGS],

    pub dirty_uniform: [u64; 2],  // 128 bits
    pub dirty_storage: [u64; 2],  // 128 bits
    pub dirty_textures: [u64; 5], // 320 bits
    pub dirty_images: [u64; 2],   // 128 bits
}

impl Default for RyuDescriptorState {
    fn default() -> Self {
        Self::new()
    }
}

impl RyuDescriptorState {
    pub fn new() -> Self {
        Self {
            uniform_buffers: [RyuDescriptorBufferInfo::default(); MAX_UNIFORM_BUFFER_BINDINGS],
            storage_buffers: [RyuDescriptorBufferInfo::default(); MAX_STORAGE_BUFFER_BINDINGS],
            textures: [RyuDescriptorImageInfo::default(); MAX_TEXTURE_BINDINGS],
            images: [RyuDescriptorImageInfo::default(); MAX_IMAGE_BINDINGS],

            committed_uniform: [RyuDescriptorBufferInfo::default(); MAX_UNIFORM_BUFFER_BINDINGS],
            committed_storage: [RyuDescriptorBufferInfo::default(); MAX_STORAGE_BUFFER_BINDINGS],
            committed_textures: [RyuDescriptorImageInfo::default(); MAX_TEXTURE_BINDINGS],
            committed_images: [RyuDescriptorImageInfo::default(); MAX_IMAGE_BINDINGS],

            dirty_uniform: [0; 2],
            dirty_storage: [0; 2],
            dirty_textures: [0; 5],
            dirty_images: [0; 2],
        }
    }

    pub fn reset(&mut self) {
        self.uniform_buffers
            .fill(RyuDescriptorBufferInfo::default());
        self.storage_buffers
            .fill(RyuDescriptorBufferInfo::default());
        self.textures.fill(RyuDescriptorImageInfo::default());
        self.images.fill(RyuDescriptorImageInfo::default());

        self.committed_uniform
            .fill(RyuDescriptorBufferInfo::default());
        self.committed_storage
            .fill(RyuDescriptorBufferInfo::default());
        self.committed_textures
            .fill(RyuDescriptorImageInfo::default());
        self.committed_images
            .fill(RyuDescriptorImageInfo::default());

        self.dirty_uniform = [0; 2];
        self.dirty_storage = [0; 2];
        self.dirty_textures = [0; 5];
        self.dirty_images = [0; 2];
    }

    #[inline]
    pub fn update_buffer(
        &mut self,
        set_type: u32,
        index: usize,
        buffer_handle: u64,
        offset: u64,
        range: u64,
    ) -> bool {
        let new_info = RyuDescriptorBufferInfo {
            buffer_handle,
            offset,
            range,
        };

        match set_type {
            SET_UNIFORM => {
                if index >= MAX_UNIFORM_BUFFER_BINDINGS {
                    return false;
                }
                if !buffers_equal(&self.uniform_buffers[index], &new_info) {
                    self.uniform_buffers[index] = new_info;
                    let word = index / 64;
                    let bit = index % 64;
                    self.dirty_uniform[word] |= 1u64 << bit;
                    true
                } else {
                    false
                }
            }
            SET_STORAGE => {
                if index >= MAX_STORAGE_BUFFER_BINDINGS {
                    return false;
                }
                if !buffers_equal(&self.storage_buffers[index], &new_info) {
                    self.storage_buffers[index] = new_info;
                    let word = index / 64;
                    let bit = index % 64;
                    self.dirty_storage[word] |= 1u64 << bit;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    #[inline]
    pub fn update_image(
        &mut self,
        set_type: u32,
        index: usize,
        sampler_handle: u64,
        image_view_handle: u64,
        image_layout: u32,
    ) -> bool {
        let new_info = RyuDescriptorImageInfo {
            sampler_handle,
            image_view_handle,
            image_layout,
            _pad: 0,
        };

        match set_type {
            SET_TEXTURE => {
                if index >= MAX_TEXTURE_BINDINGS {
                    return false;
                }
                if !images_equal(&self.textures[index], &new_info) {
                    self.textures[index] = new_info;
                    let word = index / 64;
                    let bit = index % 64;
                    self.dirty_textures[word] |= 1u64 << bit;
                    true
                } else {
                    false
                }
            }
            SET_IMAGE => {
                if index >= MAX_IMAGE_BINDINGS {
                    return false;
                }
                if !images_equal(&self.images[index], &new_info) {
                    self.images[index] = new_info;
                    let word = index / 64;
                    let bit = index % 64;
                    self.dirty_images[word] |= 1u64 << bit;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Scans dirty bits using hardware CTZ, filters out bindings identical to committed state,
    /// writes changed indices and buffer infos into contiguous output arrays, and marks them committed.
    pub fn filter_redundant_buffers(
        &mut self,
        set_type: u32,
        out_indices: &mut [u32],
        out_infos: &mut [RyuDescriptorBufferInfo],
    ) -> usize {
        let (current, committed, dirty, max_bindings) = match set_type {
            SET_UNIFORM => (
                &self.uniform_buffers[..],
                &mut self.committed_uniform[..],
                &mut self.dirty_uniform[..],
                MAX_UNIFORM_BUFFER_BINDINGS,
            ),
            SET_STORAGE => (
                &self.storage_buffers[..],
                &mut self.committed_storage[..],
                &mut self.dirty_storage[..],
                MAX_STORAGE_BUFFER_BINDINGS,
            ),
            _ => return 0,
        };

        let mut count = 0;
        let limit = out_indices.len().min(out_infos.len());

        for (word_idx, mask_ref) in dirty.iter_mut().enumerate() {
            let mut mask = *mask_ref;
            while mask != 0 && count < limit {
                let bit = mask.trailing_zeros() as usize;
                let index = word_idx * 64 + bit;
                if index >= max_bindings {
                    break;
                }

                let cur = &current[index];
                let com = &committed[index];

                if !buffers_equal(cur, com) {
                    out_indices[count] = index as u32;
                    out_infos[count] = *cur;
                    committed[index] = *cur;
                    count += 1;
                }

                mask &= !(1u64 << bit);
            }
            *mask_ref = 0;
        }

        count
    }

    pub fn filter_redundant_images(
        &mut self,
        set_type: u32,
        out_indices: &mut [u32],
        out_infos: &mut [RyuDescriptorImageInfo],
    ) -> usize {
        let (current, committed, dirty, max_bindings) = match set_type {
            SET_TEXTURE => (
                &self.textures[..],
                &mut self.committed_textures[..],
                &mut self.dirty_textures[..],
                MAX_TEXTURE_BINDINGS,
            ),
            SET_IMAGE => (
                &self.images[..],
                &mut self.committed_images[..],
                &mut self.dirty_images[..],
                MAX_IMAGE_BINDINGS,
            ),
            _ => return 0,
        };

        let mut count = 0;
        let limit = out_indices.len().min(out_infos.len());

        for (word_idx, mask_ref) in dirty.iter_mut().enumerate() {
            let mut mask = *mask_ref;
            while mask != 0 && count < limit {
                let bit = mask.trailing_zeros() as usize;
                let index = word_idx * 64 + bit;
                if index >= max_bindings {
                    break;
                }

                let cur = &current[index];
                let com = &committed[index];

                if !images_equal(cur, com) {
                    out_indices[count] = index as u32;
                    out_infos[count] = *cur;
                    committed[index] = *cur;
                    count += 1;
                }

                mask &= !(1u64 << bit);
            }
            *mask_ref = 0;
        }

        count
    }

    /// Direct zero-copy copy of contiguous buffer descriptor slice into a destination byte buffer (e.g., template writer memory).
    pub fn pack_template_buffer_slice(
        &self,
        set_type: u32,
        binding: usize,
        count: usize,
        out_buf: &mut [u8],
    ) -> usize {
        let source_slice = match set_type {
            SET_UNIFORM => {
                if binding + count > MAX_UNIFORM_BUFFER_BINDINGS {
                    return 0;
                }
                &self.uniform_buffers[binding..binding + count]
            }
            SET_STORAGE => {
                if binding + count > MAX_STORAGE_BUFFER_BINDINGS {
                    return 0;
                }
                &self.storage_buffers[binding..binding + count]
            }
            _ => return 0,
        };

        let bytes_needed = count * std::mem::size_of::<RyuDescriptorBufferInfo>();
        if out_buf.len() < bytes_needed {
            return 0;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                source_slice.as_ptr() as *const u8,
                out_buf.as_mut_ptr(),
                bytes_needed,
            );
        }

        bytes_needed
    }

    pub fn commit_all(&mut self, set_type: u32) {
        match set_type {
            SET_UNIFORM => {
                self.committed_uniform
                    .copy_from_slice(&self.uniform_buffers);
                self.dirty_uniform = [0; 2];
            }
            SET_STORAGE => {
                self.committed_storage
                    .copy_from_slice(&self.storage_buffers);
                self.dirty_storage = [0; 2];
            }
            SET_TEXTURE => {
                self.committed_textures.copy_from_slice(&self.textures);
                self.dirty_textures = [0; 5];
            }
            SET_IMAGE => {
                self.committed_images.copy_from_slice(&self.images);
                self.dirty_images = [0; 2];
            }
            _ => {}
        }
    }
}
