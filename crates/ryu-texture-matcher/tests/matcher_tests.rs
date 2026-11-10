use ryu_texture_matcher::*;

#[test]
fn test_pack_and_unpack_key() {
    let key = PackedTextureKey::new(
        36,    // R8G8B8A8Unorm
        1,     // Texture2D
        0,     // Red
        1,     // Green
        2,     // Blue
        3,     // Alpha
        0,     // Depth
        1,     // SamplesInX
        1,     // SamplesInY
        false, // BlockLinear
        4,     // BPP
        0,     // Unclassified
        false, // Not compressed
        false, // Not depth
    );

    assert_eq!(key.format(), 36);
    assert_eq!(key.target(), 1);
    assert_eq!(key.swizzle_r(), 0);
    assert_eq!(key.swizzle_g(), 1);
    assert_eq!(key.swizzle_b(), 2);
    assert_eq!(key.swizzle_a(), 3);
    assert_eq!(key.depth_stencil_mode(), 0);
    assert_eq!(key.samples_x(), 1);
    assert_eq!(key.samples_y(), 1);
    assert!(!key.is_linear());
    assert_eq!(key.bytes_per_pixel(), 4);
    assert!(!key.is_compressed());
    assert!(!key.is_depth_or_stencil());
}

#[test]
fn test_sampler_params_matching() {
    let key_a = PackedTextureKey::new(36, 1, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false);
    let key_b = PackedTextureKey::new(55, 1, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false); // Different format (sRGB)
    let key_c = PackedTextureKey::new(36, 1, 1, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false); // Different SwizzleR
    let key_d = PackedTextureKey::new(36, 1, 0, 1, 2, 3, 1, 1, 1, false, 4, 0, false, false); // Different DepthStencilMode

    assert!(key_a.sampler_params_matches(&key_b));
    assert!(!key_a.sampler_params_matches(&key_c));
    assert!(!key_a.sampler_params_matches(&key_d));
}

#[test]
fn test_target_and_samples_compatible() {
    let key_a = PackedTextureKey::new(36, 1, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false);
    let key_b = PackedTextureKey::new(36, 1, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false);
    let key_diff_target =
        PackedTextureKey::new(36, 2, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false); // Texture3D
    let key_diff_samples =
        PackedTextureKey::new(36, 1, 0, 1, 2, 3, 0, 2, 2, false, 4, 0, false, false); // 2x2 Samples

    assert!(key_a.target_and_samples_compatible(&key_b));
    assert!(!key_a.target_and_samples_compatible(&key_diff_target));
    assert!(!key_a.target_and_samples_compatible(&key_diff_samples));
}

#[test]
fn test_format_classes() {
    assert_eq!(get_format_class(65), FormatClass::Bc1Rgba as u8); // Bc1RgbaUnorm
    assert_eq!(get_format_class(68), FormatClass::Bc1Rgba as u8); // Bc1RgbaSrgb
    assert_eq!(get_format_class(75), FormatClass::Bc7 as u8); // Bc7Unorm
    assert_eq!(get_format_class(76), FormatClass::Bc7 as u8); // Bc7Srgb
    assert_eq!(get_format_class(109), FormatClass::Astc4x4 as u8); // Astc4x4Unorm
    assert_eq!(get_format_class(123), FormatClass::Astc4x4 as u8); // Astc4x4Srgb
    assert_eq!(get_format_class(0), FormatClass::Unclassified as u8);
}

#[test]
fn test_format_matches() {
    // Exact match
    assert_eq!(
        format_matches(36, 36, false, false),
        TextureMatchQuality::Perfect
    );

    // Depth-as-color alias: D32Float (52) -> R32Float (9)
    assert_eq!(
        format_matches(52, 9, true, false),
        TextureMatchQuality::FormatAlias
    );
    assert_eq!(
        format_matches(52, 9, false, false),
        TextureMatchQuality::NoMatch
    );

    // 2D engine S8Uint (49) -> R8Unorm (0)
    assert_eq!(
        format_matches(49, 0, false, true),
        TextureMatchQuality::FormatAlias
    );
    assert_eq!(
        format_matches(49, 0, false, false),
        TextureMatchQuality::NoMatch
    );
}

#[test]
fn test_view_target_compatibility() {
    // 2D vs 2D
    assert_eq!(
        view_target_compatible(1, 1, false),
        TextureViewCompatibility::Full
    );

    // 2D vs Cubemap with and without caps
    assert_eq!(
        view_target_compatible(1, 7, false),
        TextureViewCompatibility::Incompatible
    );
    assert_eq!(
        view_target_compatible(1, 7, true),
        TextureViewCompatibility::Full
    );

    // Cubemap vs 2D with and without caps
    assert_eq!(
        view_target_compatible(7, 1, false),
        TextureViewCompatibility::CopyOnly
    );
    assert_eq!(
        view_target_compatible(7, 1, true),
        TextureViewCompatibility::Full
    );

    // 3D vs 2D
    assert_eq!(
        view_target_compatible(2, 1, false),
        TextureViewCompatibility::CopyOnly
    );
    assert_eq!(
        view_target_compatible(2, 2, false),
        TextureViewCompatibility::Full
    );
}

#[test]
fn test_view_format_compatibility() {
    // BC7Unorm vs BC7Srgb (both class Bc7)
    let k_bc7_u = PackedTextureKey::new(
        75,
        1,
        0,
        1,
        2,
        3,
        0,
        1,
        1,
        false,
        16,
        FormatClass::Bc7 as u8,
        true,
        false,
    );
    let k_bc7_s = PackedTextureKey::new(
        76,
        1,
        0,
        1,
        2,
        3,
        0,
        1,
        1,
        false,
        16,
        FormatClass::Bc7 as u8,
        true,
        false,
    );
    assert_eq!(
        view_format_compatible(k_bc7_u, k_bc7_s, false, false, false, false),
        TextureViewCompatibility::Full
    );

    // Format aliasing R8G8B8A8Unorm (36, 4 bpp) vs R32G32B32A32Float (45, 16 bpp)
    let k_rgba8 = PackedTextureKey::new(36, 1, 0, 1, 2, 3, 0, 1, 1, false, 4, 0, false, false);
    let k_rgba32f = PackedTextureKey::new(45, 1, 0, 1, 2, 3, 0, 1, 1, false, 16, 0, false, false);
    assert_eq!(
        view_format_compatible(k_rgba8, k_rgba32f, false, false, false, false),
        TextureViewCompatibility::CopyOnly
    );
}

#[test]
fn test_c_abi_lifecycle() {
    assert_eq!(ryu_texture_matcher_version(), 0x00010000);

    let k1 = ryu_texture_pack_key(36, 1, 0, 1, 2, 3, 0, 1, 1, 0, 4, 0, 0);
    let k2 = ryu_texture_pack_key(36, 1, 0, 1, 2, 3, 0, 1, 1, 0, 4, 0, 0);
    let k3 = ryu_texture_pack_key(36, 1, 1, 1, 2, 3, 0, 1, 1, 0, 4, 0, 0);

    assert_eq!(ryu_texture_sampler_matches(k1, k2), 1);
    assert_eq!(ryu_texture_sampler_matches(k1, k3), 0);
    assert_eq!(ryu_texture_target_samples_compatible(k1, k2), 1);

    assert_eq!(ryu_texture_format_matches(36, 36, 0, 0), 2); // Perfect
    assert_eq!(ryu_texture_get_format_class(75), FormatClass::Bc7 as u32);
}
