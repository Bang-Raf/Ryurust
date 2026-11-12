use crate::matcher;
use crate::types::PackedTextureKey;

pub const RYU_TEXTURE_MATCHER_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_texture_matcher_version() -> u32 {
    RYU_TEXTURE_MATCHER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_texture_pack_key(
    format: u16,
    target: u8,
    swizzle_r: u8,
    swizzle_g: u8,
    swizzle_b: u8,
    swizzle_a: u8,
    depth_stencil_mode: u8,
    samples_x: u8,
    samples_y: u8,
    is_linear: i32,
    bpp: u8,
    is_compressed: i32,
    is_depth_stencil: i32,
) -> u64 {
    let format_class = matcher::get_format_class(format as u32);
    let key = PackedTextureKey::new(
        format,
        target,
        swizzle_r,
        swizzle_g,
        swizzle_b,
        swizzle_a,
        depth_stencil_mode,
        samples_x,
        samples_y,
        is_linear != 0,
        bpp,
        format_class,
        is_compressed != 0,
        is_depth_stencil != 0,
    );
    key.0
}

#[no_mangle]
pub extern "C" fn ryu_texture_sampler_matches(key_a: u64, key_b: u64) -> i32 {
    let k_a = PackedTextureKey(key_a);
    let k_b = PackedTextureKey(key_b);
    if k_a.sampler_params_matches(&k_b) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_texture_target_samples_compatible(key_a: u64, key_b: u64) -> i32 {
    let k_a = PackedTextureKey(key_a);
    let k_b = PackedTextureKey(key_b);
    if k_a.target_and_samples_compatible(&k_b) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_texture_format_matches(
    format_a: u32,
    format_b: u32,
    for_sampler: i32,
    depth_alias: i32,
) -> i32 {
    matcher::format_matches(format_a, format_b, for_sampler != 0, depth_alias != 0) as i32
}

#[no_mangle]
pub extern "C" fn ryu_texture_view_target_compatible(
    target_a: u32,
    target_b: u32,
    supports_cubemap: i32,
) -> i32 {
    matcher::view_target_compatible(target_a as u8, target_b as u8, supports_cubemap != 0) as i32
}

#[no_mangle]
pub extern "C" fn ryu_texture_get_format_class(format: u32) -> u32 {
    matcher::get_format_class(format) as u32
}

#[no_mangle]
pub extern "C" fn ryu_texture_view_format_compatible(
    key_a: u64,
    key_b: u64,
    caps_incompat_a: i32,
    caps_incompat_b: i32,
    for_sampler: i32,
    depth_alias: i32,
) -> i32 {
    let k_a = PackedTextureKey(key_a);
    let k_b = PackedTextureKey(key_b);
    matcher::view_format_compatible(
        k_a,
        k_b,
        caps_incompat_a != 0,
        caps_incompat_b != 0,
        for_sampler != 0,
        depth_alias != 0,
    ) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // No export of this crate takes a pointer, so there is no null-pointer test.
    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_texture_matcher_version(), RYU_TEXTURE_MATCHER_VERSION);
        assert_eq!(RYU_TEXTURE_MATCHER_VERSION, 0x0001_0000);
    }
}
