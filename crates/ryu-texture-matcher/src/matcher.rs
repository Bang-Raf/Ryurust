use crate::types::{FormatClass, PackedTextureKey, TextureMatchQuality, TextureViewCompatibility};

// Precomputed LUT for fast O(1) format class lookup
pub static FORMAT_CLASS_LUT: [u8; 150] = {
    let mut lut = [0u8; 150];
    lut[65] = FormatClass::Bc1Rgba as u8;
    lut[68] = FormatClass::Bc1Rgba as u8;
    lut[66] = FormatClass::Bc2 as u8;
    lut[69] = FormatClass::Bc2 as u8;
    lut[67] = FormatClass::Bc3 as u8;
    lut[70] = FormatClass::Bc3 as u8;
    lut[71] = FormatClass::Bc4 as u8;
    lut[72] = FormatClass::Bc4 as u8;
    lut[73] = FormatClass::Bc5 as u8;
    lut[74] = FormatClass::Bc5 as u8;
    lut[77] = FormatClass::Bc6 as u8;
    lut[78] = FormatClass::Bc6 as u8;
    lut[75] = FormatClass::Bc7 as u8;
    lut[76] = FormatClass::Bc7 as u8;
    lut[79] = FormatClass::Etc2Rgb as u8;
    lut[82] = FormatClass::Etc2Rgb as u8;
    lut[80] = FormatClass::Etc2Rgba as u8;
    lut[83] = FormatClass::Etc2Rgba as u8;

    lut[109] = FormatClass::Astc4x4 as u8;
    lut[123] = FormatClass::Astc4x4 as u8;
    lut[110] = FormatClass::Astc5x4 as u8;
    lut[124] = FormatClass::Astc5x4 as u8;
    lut[111] = FormatClass::Astc5x5 as u8;
    lut[125] = FormatClass::Astc5x5 as u8;
    lut[112] = FormatClass::Astc6x5 as u8;
    lut[126] = FormatClass::Astc6x5 as u8;
    lut[113] = FormatClass::Astc6x6 as u8;
    lut[127] = FormatClass::Astc6x6 as u8;
    lut[114] = FormatClass::Astc8x5 as u8;
    lut[128] = FormatClass::Astc8x5 as u8;
    lut[115] = FormatClass::Astc8x6 as u8;
    lut[129] = FormatClass::Astc8x6 as u8;
    lut[116] = FormatClass::Astc8x8 as u8;
    lut[130] = FormatClass::Astc8x8 as u8;
    lut[117] = FormatClass::Astc10x5 as u8;
    lut[131] = FormatClass::Astc10x5 as u8;
    lut[118] = FormatClass::Astc10x6 as u8;
    lut[132] = FormatClass::Astc10x6 as u8;
    lut[119] = FormatClass::Astc10x8 as u8;
    lut[133] = FormatClass::Astc10x8 as u8;
    lut[120] = FormatClass::Astc10x10 as u8;
    lut[134] = FormatClass::Astc10x10 as u8;
    lut[121] = FormatClass::Astc12x10 as u8;
    lut[135] = FormatClass::Astc12x10 as u8;
    lut[122] = FormatClass::Astc12x12 as u8;
    lut[136] = FormatClass::Astc12x12 as u8;
    lut
};

#[inline(always)]
pub fn get_format_class(format: u32) -> u8 {
    if (format as usize) < FORMAT_CLASS_LUT.len() {
        FORMAT_CLASS_LUT[format as usize]
    } else {
        FormatClass::Unclassified as u8
    }
}

#[inline(always)]
pub fn is_valid_depth_as_color_alias(lhs: u32, rhs: u32) -> bool {
    (lhs == 52 && rhs == 9) || (lhs == 50 && rhs == 5)
}

#[inline(always)]
pub fn is_valid_color_as_depth_alias(lhs: u32, rhs: u32) -> bool {
    (lhs == 9 && rhs == 52) || (lhs == 5 && rhs == 50)
}

#[inline]
pub fn format_matches(
    lhs_format: u32,
    rhs_format: u32,
    for_sampler: bool,
    depth_alias: bool,
) -> TextureMatchQuality {
    if is_valid_depth_as_color_alias(lhs_format, rhs_format) && (for_sampler || depth_alias) {
        return TextureMatchQuality::FormatAlias;
    }

    if depth_alias {
        if lhs_format == 49 && rhs_format == 0 {
            // S8Uint == R8Unorm
            return TextureMatchQuality::FormatAlias;
        } else if (lhs_format == 53 || lhs_format == 51 || lhs_format == 143) && rhs_format == 140 {
            // D24UnormS8Uint / S8UintD24Unorm / X8UintD24Unorm == B8G8R8A8Unorm
            return TextureMatchQuality::FormatAlias;
        } else if lhs_format == 54 && rhs_format == 21 {
            // D32FloatS8Uint == R32G32Float
            return TextureMatchQuality::FormatAlias;
        }
    }

    if lhs_format == rhs_format {
        TextureMatchQuality::Perfect
    } else {
        TextureMatchQuality::NoMatch
    }
}

#[inline]
pub fn is_incompatible_format_aliasing_allowed(
    mut lhs_format: u32,
    mut rhs_format: u32,
    lhs_bpp: u8,
    rhs_bpp: u8,
) -> bool {
    if rhs_bpp < lhs_bpp {
        std::mem::swap(&mut lhs_format, &mut rhs_format);
    }

    (lhs_format == 36 && rhs_format == 45) // R8G8B8A8Unorm && R32G32B32A32Float
        || (lhs_format == 0 && rhs_format == 36) // R8Unorm && R8G8B8A8Unorm
        || (lhs_format == 0 && rhs_format == 10) // R8Unorm && R32Uint
}

#[inline]
pub fn view_target_compatible(
    lhs: u8,
    rhs: u8,
    supports_cubemap_view: bool,
) -> TextureViewCompatibility {
    match lhs {
        0 | 3 => {
            // Texture1D, Texture1DArray
            if rhs == 0 || rhs == 3 {
                TextureViewCompatibility::Full
            } else {
                TextureViewCompatibility::Incompatible
            }
        }
        1 | 4 => {
            // Texture2D, Texture2DArray
            if rhs == 1 || rhs == 4 {
                TextureViewCompatibility::Full
            } else if supports_cubemap_view && (rhs == 7 || rhs == 8) {
                TextureViewCompatibility::Full
            } else {
                TextureViewCompatibility::Incompatible
            }
        }
        7 | 8 => {
            // Cubemap, CubemapArray
            if rhs == 7 || rhs == 8 {
                TextureViewCompatibility::Full
            } else if rhs == 1 || rhs == 4 {
                if supports_cubemap_view {
                    TextureViewCompatibility::Full
                } else {
                    TextureViewCompatibility::CopyOnly
                }
            } else {
                TextureViewCompatibility::Incompatible
            }
        }
        5 | 6 => {
            // Texture2DMultisample, Texture2DMultisampleArray
            if rhs == 1 || rhs == 4 {
                TextureViewCompatibility::CopyOnly
            } else if rhs == 5 || rhs == 6 {
                TextureViewCompatibility::Full
            } else {
                TextureViewCompatibility::Incompatible
            }
        }
        2 => {
            // Texture3D
            if rhs == 1 {
                TextureViewCompatibility::CopyOnly
            } else if rhs == 2 {
                TextureViewCompatibility::Full
            } else {
                TextureViewCompatibility::Incompatible
            }
        }
        _ => TextureViewCompatibility::Incompatible,
    }
}

#[inline]
pub fn view_format_compatible(
    lhs: PackedTextureKey,
    rhs: PackedTextureKey,
    caps_host_incompatible_lhs: bool,
    caps_host_incompatible_rhs: bool,
    for_sampler: bool,
    depth_alias: bool,
) -> TextureViewCompatibility {
    if lhs.is_depth_or_stencil() || rhs.is_depth_or_stencil() {
        let quality = format_matches(
            lhs.format() as u32,
            rhs.format() as u32,
            for_sampler,
            depth_alias,
        );
        return match quality {
            TextureMatchQuality::Perfect => TextureViewCompatibility::Full,
            TextureMatchQuality::FormatAlias => TextureViewCompatibility::FormatAlias,
            TextureMatchQuality::NoMatch => {
                if is_valid_color_as_depth_alias(lhs.format() as u32, rhs.format() as u32)
                    || is_valid_depth_as_color_alias(lhs.format() as u32, rhs.format() as u32)
                {
                    TextureViewCompatibility::CopyOnly
                } else {
                    TextureViewCompatibility::Incompatible
                }
            }
        };
    }

    if caps_host_incompatible_lhs || caps_host_incompatible_rhs {
        return if lhs.format() == rhs.format() {
            TextureViewCompatibility::Full
        } else {
            TextureViewCompatibility::Incompatible
        };
    }

    if lhs.is_compressed() && rhs.is_compressed() {
        let lhs_class = lhs.format_class();
        let rhs_class = rhs.format_class();
        if lhs_class == rhs_class && lhs_class != 0 {
            TextureViewCompatibility::Full
        } else {
            TextureViewCompatibility::Incompatible
        }
    } else if lhs.bytes_per_pixel() == rhs.bytes_per_pixel() {
        if lhs.is_compressed() == rhs.is_compressed() {
            TextureViewCompatibility::Full
        } else {
            TextureViewCompatibility::CopyOnly
        }
    } else if is_incompatible_format_aliasing_allowed(
        lhs.format() as u32,
        rhs.format() as u32,
        lhs.bytes_per_pixel(),
        rhs.bytes_per_pixel(),
    ) {
        TextureViewCompatibility::CopyOnly
    } else {
        TextureViewCompatibility::Incompatible
    }
}
