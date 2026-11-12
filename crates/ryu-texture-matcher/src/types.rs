#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FormatClass {
    Unclassified = 0,
    Bc1Rgba = 1,
    Bc2 = 2,
    Bc3 = 3,
    Bc4 = 4,
    Bc5 = 5,
    Bc6 = 6,
    Bc7 = 7,
    Etc2Rgb = 8,
    Etc2Rgba = 9,
    Astc4x4 = 10,
    Astc5x4 = 11,
    Astc5x5 = 12,
    Astc6x5 = 13,
    Astc6x6 = 14,
    Astc8x5 = 15,
    Astc8x6 = 16,
    Astc8x8 = 17,
    Astc10x5 = 18,
    Astc10x6 = 19,
    Astc10x8 = 20,
    Astc10x10 = 21,
    Astc12x10 = 22,
    Astc12x12 = 23,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureViewCompatibility {
    Incompatible = 0,
    LayoutIncompatible = 1,
    CopyOnly = 2,
    FormatAlias = 3,
    Full = 4,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureMatchQuality {
    NoMatch = 0,
    FormatAlias = 1,
    Perfect = 2,
}

/// Bit-packed 64-bit Texture Key for instant zero-allocation matching:
/// - Bits 0..15  (16 bits): Format
/// - Bits 16..23 (8 bits) : Target
/// - Bits 24..26 (3 bits) : SwizzleR (0..5)
/// - Bits 27..29 (3 bits) : SwizzleG (0..5)
/// - Bits 30..32 (3 bits) : SwizzleB (0..5)
/// - Bits 33..35 (3 bits) : SwizzleA (0..5)
/// - Bit 36      (1 bit)  : DepthStencilMode (0: Depth, 1: Stencil)
/// - Bits 37..40 (4 bits) : SamplesInX (1..16)
/// - Bits 41..44 (4 bits) : SamplesInY (1..16)
/// - Bit 45      (1 bit)  : IsLinear (0: BlockLinear, 1: Linear)
/// - Bits 46..51 (6 bits) : BytesPerPixel (1..64)
/// - Bits 52..57 (6 bits) : FormatClass (0..23)
/// - Bit 58      (1 bit)  : IsCompressed (0: No, 1: Yes)
/// - Bit 59      (1 bit)  : IsDepthOrStencil (0: No, 1: Yes)
/// - Bits 60..63 (4 bits) : Reserved
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedTextureKey(pub u64);

impl PackedTextureKey {
    // Mask for SwizzleR..A (12 bits: 24..35) + DepthStencilMode (1 bit: 36) = 13 bits (24..36)
    pub const SAMPLER_MASK: u64 = 0x0000_001F_FF00_0000;

    // Mask for Target (bits 16..23) + SamplesInX/Y (bits 37..44)
    pub const TARGET_MASK: u64 = 0x0000_0000_00FF_0000;
    pub const SAMPLES_MASK: u64 = 0x0000_1FE0_0000_0000;
    pub const TARGET_AND_SAMPLES_MASK: u64 = Self::TARGET_MASK | Self::SAMPLES_MASK;

    pub const FORMAT_MASK: u64 = 0x0000_0000_0000_FFFF;
    pub const BPP_MASK: u64 = 0x000F_C000_0000_0000;
    pub const FORMAT_CLASS_MASK: u64 = 0x03F0_0000_0000_0000;

    #[allow(clippy::too_many_arguments)] // one parameter per packed key field, matching the `ryu_texture_pack_key` C ABI
    #[inline(always)]
    pub fn new(
        format: u16,
        target: u8,
        swizzle_r: u8,
        swizzle_g: u8,
        swizzle_b: u8,
        swizzle_a: u8,
        depth_stencil_mode: u8,
        samples_x: u8,
        samples_y: u8,
        is_linear: bool,
        bpp: u8,
        format_class: u8,
        is_compressed: bool,
        is_depth_or_stencil: bool,
    ) -> Self {
        let mut v: u64 = (format as u64) & 0xFFFF;
        v |= ((target as u64) & 0xFF) << 16;
        v |= ((swizzle_r as u64) & 0x07) << 24;
        v |= ((swizzle_g as u64) & 0x07) << 27;
        v |= ((swizzle_b as u64) & 0x07) << 30;
        v |= ((swizzle_a as u64) & 0x07) << 33;
        v |= ((depth_stencil_mode as u64) & 0x01) << 36;
        v |= ((samples_x as u64) & 0x0F) << 37;
        v |= ((samples_y as u64) & 0x0F) << 41;
        if is_linear {
            v |= 1u64 << 45;
        }
        v |= ((bpp as u64) & 0x3F) << 46;
        v |= ((format_class as u64) & 0x3F) << 52;
        if is_compressed {
            v |= 1u64 << 58;
        }
        if is_depth_or_stencil {
            v |= 1u64 << 59;
        }
        Self(v)
    }

    #[inline(always)]
    pub fn format(&self) -> u16 {
        (self.0 & Self::FORMAT_MASK) as u16
    }

    #[inline(always)]
    pub fn target(&self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn swizzle_r(&self) -> u8 {
        ((self.0 >> 24) & 0x07) as u8
    }

    #[inline(always)]
    pub fn swizzle_g(&self) -> u8 {
        ((self.0 >> 27) & 0x07) as u8
    }

    #[inline(always)]
    pub fn swizzle_b(&self) -> u8 {
        ((self.0 >> 30) & 0x07) as u8
    }

    #[inline(always)]
    pub fn swizzle_a(&self) -> u8 {
        ((self.0 >> 33) & 0x07) as u8
    }

    #[inline(always)]
    pub fn depth_stencil_mode(&self) -> u8 {
        ((self.0 >> 36) & 0x01) as u8
    }

    #[inline(always)]
    pub fn samples_x(&self) -> u8 {
        ((self.0 >> 37) & 0x0F) as u8
    }

    #[inline(always)]
    pub fn samples_y(&self) -> u8 {
        ((self.0 >> 41) & 0x0F) as u8
    }

    #[inline(always)]
    pub fn is_linear(&self) -> bool {
        ((self.0 >> 45) & 0x01) != 0
    }

    #[inline(always)]
    pub fn bytes_per_pixel(&self) -> u8 {
        ((self.0 >> 46) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn format_class(&self) -> u8 {
        ((self.0 >> 52) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn is_compressed(&self) -> bool {
        ((self.0 >> 58) & 0x01) != 0
    }

    #[inline(always)]
    pub fn is_depth_or_stencil(&self) -> bool {
        ((self.0 >> 59) & 0x01) != 0
    }

    #[inline(always)]
    pub fn sampler_params_matches(&self, other: &Self) -> bool {
        ((self.0 ^ other.0) & Self::SAMPLER_MASK) == 0
    }

    #[inline(always)]
    pub fn target_and_samples_compatible(&self, other: &Self) -> bool {
        ((self.0 ^ other.0) & Self::TARGET_AND_SAMPLES_MASK) == 0
    }
}
