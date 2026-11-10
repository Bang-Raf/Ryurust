#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start: u64,
    pub end: u64,
    pub value: u64,
}

impl Interval {
    #[inline(always)]
    pub fn new(start: u64, end: u64, value: u64) -> Self {
        Self { start, end, value }
    }

    #[inline(always)]
    pub fn overlaps(&self, q_start: u64, q_end: u64) -> bool {
        self.start < q_end && q_start < self.end
    }
}

#[derive(Debug, Clone, Default)]
pub struct FlatIntervalTree {
    intervals: Vec<Interval>,
}

impl FlatIntervalTree {
    pub fn new() -> Self {
        Self {
            intervals: Vec::with_capacity(64),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            intervals: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    pub fn clear(&mut self) {
        self.intervals.clear();
    }

    pub fn insert(&mut self, start: u64, end: u64, value: u64) {
        let item = Interval::new(start, end, value);
        let idx = match self.intervals.binary_search_by_key(&start, |i| i.start) {
            Ok(pos) => pos,
            Err(pos) => pos,
        };
        self.intervals.insert(idx, item);
    }

    pub fn remove(&mut self, start: u64, end: u64, value: u64) -> bool {
        if let Some(pos) = self
            .intervals
            .iter()
            .position(|i| i.start == start && i.end == end && i.value == value)
        {
            self.intervals.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn find_overlaps(&self, q_start: u64, q_end: u64, out: &mut Vec<u64>) {
        if self.intervals.is_empty() || q_start >= q_end {
            return;
        }

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") && self.intervals.len() >= 8 {
                unsafe {
                    self.find_overlaps_avx2(q_start, q_end, out);
                    return;
                }
            }
        }

        self.find_overlaps_scalar(q_start, q_end, out);
    }

    pub fn find_overlaps_slice(&self, q_start: u64, q_end: u64, out: &mut [u64]) -> usize {
        if self.intervals.is_empty() || q_start >= q_end {
            return 0;
        }

        let mut count = 0;
        for item in &self.intervals {
            if item.start >= q_end {
                break;
            }
            if item.end > q_start {
                if count < out.len() {
                    out[count] = item.value;
                }
                count += 1;
            }
        }
        count
    }

    #[inline(always)]
    pub fn find_overlaps_scalar(&self, q_start: u64, q_end: u64, out: &mut Vec<u64>) {
        for item in &self.intervals {
            if item.start >= q_end {
                break;
            }
            if item.end > q_start {
                out.push(item.value);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_overlaps_avx2(&self, q_start: u64, q_end: u64, out: &mut Vec<u64>) {
        let count = self.intervals.len();
        let chunks = count / 4;
        let bias = _mm256_set1_epi64x(i64::MIN); // 0x8000_0000_0000_0000 for unsigned comparison

        let v_q_start = _mm256_xor_si256(_mm256_set1_epi64x(q_start as i64), bias);
        let v_q_end = _mm256_xor_si256(_mm256_set1_epi64x(q_end as i64), bias);

        let ptr = self.intervals.as_ptr();

        for i in 0..chunks {
            let offset = i * 4;
            let item0 = *ptr.add(offset);
            let item1 = *ptr.add(offset + 1);
            let item2 = *ptr.add(offset + 2);
            let item3 = *ptr.add(offset + 3);

            // If the first interval in this sorted block starts after q_end, then all remaining do too
            if item0.start >= q_end {
                return;
            }

            let starts = _mm256_set_epi64x(
                item3.start as i64,
                item2.start as i64,
                item1.start as i64,
                item0.start as i64,
            );
            let ends = _mm256_set_epi64x(
                item3.end as i64,
                item2.end as i64,
                item1.end as i64,
                item0.end as i64,
            );

            let v_s = _mm256_xor_si256(starts, bias);
            let v_e = _mm256_xor_si256(ends, bias);

            // s < q_end <=> q_end > s
            let cmp1 = _mm256_cmpgt_epi64(v_q_end, v_s);
            // q_start < e <=> e > q_start
            let cmp2 = _mm256_cmpgt_epi64(v_e, v_q_start);

            let overlap = _mm256_and_si256(cmp1, cmp2);
            let mask = _mm256_movemask_pd(_mm256_castsi256_pd(overlap)) as u32;

            if (mask & 1) != 0 {
                out.push(item0.value);
            }
            if (mask & 2) != 0 {
                out.push(item1.value);
            }
            if (mask & 4) != 0 {
                out.push(item2.value);
            }
            if (mask & 8) != 0 {
                out.push(item3.value);
            }
        }

        // Remainder
        for i in (chunks * 4)..count {
            let item = *ptr.add(i);
            if item.start >= q_end {
                break;
            }
            if item.end > q_start {
                out.push(item.value);
            }
        }
    }
}
