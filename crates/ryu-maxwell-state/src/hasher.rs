//! Fast SIMD/WyHash-inspired 64-bit pipeline state key hasher.
//! Designed to replace multi-loop scalar `* 23` hashing in `PipelineUid.GetHashCode()`.

const SECRET1: u64 = 0xa076_1d64_78bd_642f;
const SECRET2: u64 = 0xe703_7ed1_a0b4_28db;
const SECRET3: u64 = 0x8ebc_6af0_9c88_c6e3;

#[inline(always)]
fn mum(a: u64, b: u64) -> u64 {
    let r = (a as u128).wrapping_mul(b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

#[inline(always)]
fn read_u64_le(p: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([
        p[off],
        p[off + 1],
        p[off + 2],
        p[off + 3],
        p[off + 4],
        p[off + 5],
        p[off + 6],
        p[off + 7],
    ])
}

#[inline(always)]
fn read_u32_le(p: &[u8], off: usize) -> u64 {
    u32::from_le_bytes([p[off], p[off + 1], p[off + 2], p[off + 3]]) as u64
}

/// Computes a high-dispersion 64-bit hash over pipeline state bytes.
#[inline]
pub fn pipeline_state_hash64(data: &[u8]) -> u64 {
    let len = data.len();
    let mut seed = SECRET1 ^ (len as u64);

    if len == 0 {
        return mum(seed, SECRET2);
    }

    let mut i = 0;
    // Process 32-byte blocks using 4 independent lanes (instruction-level parallelism)
    let mut s0 = seed;
    let mut s1 = seed ^ SECRET2;
    let mut s2 = seed ^ SECRET3;
    let mut s3 = seed.wrapping_add(SECRET1);

    while i + 32 <= len {
        let v0 = read_u64_le(data, i);
        let v1 = read_u64_le(data, i + 8);
        let v2 = read_u64_le(data, i + 16);
        let v3 = read_u64_le(data, i + 24);

        s0 = mum(v0 ^ s0, SECRET1);
        s1 = mum(v1 ^ s1, SECRET2);
        s2 = mum(v2 ^ s2, SECRET3);
        s3 = mum(v3 ^ s3, SECRET1);
        i += 32;
    }

    seed = s0 ^ s1 ^ s2 ^ s3;

    while i + 8 <= len {
        let v = read_u64_le(data, i);
        seed = mum(v ^ seed, SECRET2);
        i += 8;
    }

    if i + 4 <= len {
        let v = read_u32_le(data, i);
        seed = mum(v ^ seed, SECRET3);
        i += 4;
    }

    let mut tail = 0u64;
    let mut shift = 0;
    while i < len {
        tail |= (data[i] as u64) << shift;
        shift += 8;
        i += 1;
    }
    if shift > 0 {
        seed = mum(tail ^ seed, SECRET1);
    }

    mum(seed ^ (len as u64), SECRET3)
}

#[inline(always)]
pub fn pipeline_state_hash32(data: &[u8]) -> i32 {
    let h = pipeline_state_hash64(data);
    ((h as u32) ^ ((h >> 32) as u32)) as i32
}

/// Fast byte-slice equality using AVX2 when available.
#[inline]
pub fn pipeline_state_equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 32 {
            // SAFETY: AVX2 was just detected and the slices have equal length.
            return unsafe { pipeline_state_equals_avx2(a, b) };
        }
    }

    a == b
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn pipeline_state_equals_avx2(a: &[u8], b: &[u8]) -> bool {
    use std::arch::x86_64::*;

    let len = a.len();
    let mut i = 0;
    let pa = a.as_ptr();
    let pb = b.as_ptr();

    while i + 32 <= len {
        let va = _mm256_loadu_si256(pa.add(i) as *const __m256i);
        let vb = _mm256_loadu_si256(pb.add(i) as *const __m256i);
        let eq = _mm256_cmpeq_epi8(va, vb);
        if _mm256_movemask_epi8(eq) != -1 {
            return false;
        }
        i += 32;
    }

    while i < len {
        if *pa.add(i) != *pb.add(i) {
            return false;
        }
        i += 1;
    }

    true
}
