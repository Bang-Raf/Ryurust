#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRange {
    pub offset: usize,
    pub length: usize,
}

impl DirtyRange {
    #[inline(always)]
    pub fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }

    #[inline(always)]
    pub fn end(&self) -> usize {
        self.offset + self.length
    }
}

pub fn is_equal_fast(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let len = a.len();
    if len < 32 {
        return a == b;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 was just detected and both slices are `len` bytes long.
            unsafe {
                return is_equal_avx2(a.as_ptr(), b.as_ptr(), len);
            }
        }
    }

    a == b
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn is_equal_avx2(a_ptr: *const u8, b_ptr: *const u8, len: usize) -> bool {
    let chunks = len / 32;
    for i in 0..chunks {
        let offset = i * 32;
        let va = _mm256_loadu_si256(a_ptr.add(offset) as *const __m256i);
        let vb = _mm256_loadu_si256(b_ptr.add(offset) as *const __m256i);
        let eq = _mm256_cmpeq_epi8(va, vb);
        let mask = _mm256_movemask_epi8(eq) as u32;
        if mask != 0xFFFF_FFFF {
            return false;
        }
    }

    let remainder_start = chunks * 32;
    for i in remainder_start..len {
        if *a_ptr.add(i) != *b_ptr.add(i) {
            return false;
        }
    }

    true
}

pub fn find_dirty_range(src: &[u8], dst: &[u8]) -> Option<DirtyRange> {
    let len = src.len().min(dst.len());
    if len == 0 {
        return None;
    }

    if len < 32 {
        return find_dirty_range_scalar(src, dst, len);
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 was just detected and both slices are at least `len` bytes long.
            unsafe {
                return find_dirty_range_avx2(src.as_ptr(), dst.as_ptr(), len);
            }
        }
    }

    find_dirty_range_scalar(src, dst, len)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_dirty_range_avx2(
    src_ptr: *const u8,
    dst_ptr: *const u8,
    len: usize,
) -> Option<DirtyRange> {
    let chunks = len / 32;
    let mut first_diff: Option<usize> = None;

    // 1. Forward scan to find first modified byte
    for i in 0..chunks {
        let offset = i * 32;
        let va = _mm256_loadu_si256(src_ptr.add(offset) as *const __m256i);
        let vb = _mm256_loadu_si256(dst_ptr.add(offset) as *const __m256i);
        let eq = _mm256_cmpeq_epi8(va, vb);
        let mask = _mm256_movemask_epi8(eq) as u32;

        if mask != 0xFFFF_FFFF {
            let diff = !mask;
            let byte_idx = diff.trailing_zeros() as usize;
            first_diff = Some(offset + byte_idx);
            break;
        }
    }

    if first_diff.is_none() {
        let remainder_start = chunks * 32;
        for i in remainder_start..len {
            if *src_ptr.add(i) != *dst_ptr.add(i) {
                first_diff = Some(i);
                break;
            }
        }
    }

    let first = first_diff?;

    // 2. Backward scan to find last modified byte
    let remainder_start = chunks * 32;
    if remainder_start < len {
        for i in (remainder_start..len).rev() {
            if *src_ptr.add(i) != *dst_ptr.add(i) {
                return Some(DirtyRange::new(first, i - first + 1));
            }
        }
    }

    let first_chunk = first / 32;
    for chunk_idx in (first_chunk..chunks).rev() {
        let offset = chunk_idx * 32;
        let va = _mm256_loadu_si256(src_ptr.add(offset) as *const __m256i);
        let vb = _mm256_loadu_si256(dst_ptr.add(offset) as *const __m256i);
        let eq = _mm256_cmpeq_epi8(va, vb);
        let mask = _mm256_movemask_epi8(eq) as u32;

        if mask != 0xFFFF_FFFF {
            let diff = !mask;
            let bit_idx = 31 - diff.leading_zeros() as usize;
            let last = offset + bit_idx;
            return Some(DirtyRange::new(first, last - first + 1));
        }
    }

    Some(DirtyRange::new(first, 1))
}

pub fn find_dirty_range_scalar(src: &[u8], dst: &[u8], len: usize) -> Option<DirtyRange> {
    let mut first = None;
    for i in 0..len {
        if src[i] != dst[i] {
            first = Some(i);
            break;
        }
    }

    let first = first?;

    let mut last = first;
    for i in (first..len).rev() {
        if src[i] != dst[i] {
            last = i;
            break;
        }
    }

    Some(DirtyRange::new(first, last - first + 1))
}

pub fn diff_and_copy(src: &[u8], dst: &mut [u8]) -> Option<DirtyRange> {
    let dirty = find_dirty_range(src, dst)?;
    dst[dirty.offset..dirty.end()].copy_from_slice(&src[dirty.offset..dirty.end()]);
    Some(dirty)
}
