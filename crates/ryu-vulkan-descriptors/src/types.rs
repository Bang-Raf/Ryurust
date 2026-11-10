pub const MAX_SHADER_STAGES: usize = 5;
pub const MAX_UNIFORM_BUFFERS_PER_STAGE: usize = 18;
pub const MAX_STORAGE_BUFFERS_PER_STAGE: usize = 16;
pub const MAX_TEXTURES_PER_STAGE: usize = 64;
pub const MAX_IMAGES_PER_STAGE: usize = 16;

pub const MAX_UNIFORM_BUFFER_BINDINGS: usize = MAX_UNIFORM_BUFFERS_PER_STAGE * MAX_SHADER_STAGES; // 90
pub const MAX_STORAGE_BUFFER_BINDINGS: usize = MAX_STORAGE_BUFFERS_PER_STAGE * MAX_SHADER_STAGES; // 80
pub const MAX_TEXTURE_BINDINGS: usize = MAX_TEXTURES_PER_STAGE * MAX_SHADER_STAGES; // 320
pub const MAX_IMAGE_BINDINGS: usize = MAX_IMAGES_PER_STAGE * MAX_SHADER_STAGES; // 80

pub const SET_UNIFORM: u32 = 0;
pub const SET_STORAGE: u32 = 1;
pub const SET_TEXTURE: u32 = 2;
pub const SET_IMAGE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct RyuDescriptorBufferInfo {
    pub buffer_handle: u64,
    pub offset: u64,
    pub range: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct RyuDescriptorImageInfo {
    pub sampler_handle: u64,
    pub image_view_handle: u64,
    pub image_layout: u32,
    pub _pad: u32,
}
