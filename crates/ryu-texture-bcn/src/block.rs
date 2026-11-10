//! 128-bit compressed block representation and color types.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub low: u64,
    pub high: u64,
}

impl Block {
    #[inline(always)]
    pub fn new(low: u64, high: u64) -> Self {
        Self { low, high }
    }

    #[inline(always)]
    pub fn from_slice(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= 16);
        let low = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let high = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        Self { low, high }
    }

    #[inline(always)]
    pub fn decode(&self, offset: &mut usize, bits: usize) -> u64 {
        if bits == 0 {
            return 0;
        }

        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };

        let value = if *offset >= 64 {
            (self.high >> (*offset - 64)) & mask
        } else {
            let mut val = self.low >> *offset;
            if *offset + bits > 64 {
                let remainder = 64 - *offset;
                val |= self.high << remainder;
            }
            val & mask
        };

        *offset += bits;
        value
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RgbaColor32 {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

impl RgbaColor32 {
    #[inline(always)]
    pub const fn new(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self { r, g, b, a }
    }

    #[inline(always)]
    pub fn get_color8(&self) -> RgbaColor8 {
        RgbaColor8 {
            r: self.r.clamp(0, 255) as u8,
            g: self.g.clamp(0, 255) as u8,
            b: self.b.clamp(0, 255) as u8,
            a: self.a.clamp(0, 255) as u8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RgbaColor8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor8 {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline(always)]
    pub fn to_u32(&self) -> u32 {
        (self.r as u32) | ((self.g as u32) << 8) | ((self.b as u32) << 16) | ((self.a as u32) << 24)
    }

    #[inline(always)]
    pub fn from_u32(val: u32) -> Self {
        Self {
            r: (val & 0xFF) as u8,
            g: ((val >> 8) & 0xFF) as u8,
            b: ((val >> 16) & 0xFF) as u8,
            a: ((val >> 24) & 0xFF) as u8,
        }
    }
}

/// A decoded texel that can be stored little-endian into a byte buffer.
pub(crate) trait TexelLe: Copy {
    const SIZE: usize;
    fn store(self, out: &mut [u8]);
}

macro_rules! impl_texel_le {
    ($($t:ty),*) => {$(
        impl TexelLe for $t {
            const SIZE: usize = std::mem::size_of::<$t>();
            #[inline(always)]
            fn store(self, out: &mut [u8]) {
                out.copy_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}
impl_texel_le!(u16, u32, u64);

/// Copies the visible `bw` x `bh` region of a decoded 4x4 `tile` into `dst`, a byte
/// buffer of texels with a row pitch of `stride_texels`, at texel (`x_base`, `y_base`).
///
/// `dst` has no alignment requirement: texels are written as little-endian bytes,
/// which compiles to plain (unaligned) stores.
#[inline(always)]
pub(crate) fn store_tile<T: TexelLe>(
    dst: &mut [u8],
    tile: &[T; 16],
    stride_texels: usize,
    x_base: usize,
    y_base: usize,
    bw: usize,
    bh: usize,
) {
    for ty in 0..bh {
        let start = ((y_base + ty) * stride_texels + x_base) * T::SIZE;
        let row = &mut dst[start..start + bw * T::SIZE];
        let tile_row = &tile[ty * 4..ty * 4 + bw];
        for (out, &texel) in row.chunks_exact_mut(T::SIZE).zip(tile_row) {
            texel.store(out);
        }
    }
}
