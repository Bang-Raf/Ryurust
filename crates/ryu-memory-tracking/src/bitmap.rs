use std::sync::atomic::{AtomicU64, Ordering};

pub const WORD_SIZE: usize = 64;
pub const WORD_SHIFT: usize = 6;
pub const WORD_MASK: usize = WORD_SIZE - 1;

pub struct AtomicPageBitmap {
    words: Box<[AtomicU64]>,
    bit_count: usize,
}

// SAFETY: the only mutable state is the `AtomicU64` words; `bit_count` never changes
// after construction.
unsafe impl Send for AtomicPageBitmap {}
// SAFETY: see above; every `&self` method goes through atomic operations.
unsafe impl Sync for AtomicPageBitmap {}

impl AtomicPageBitmap {
    pub fn new(bit_count: usize, initial_set: bool) -> Self {
        let word_count = (bit_count + WORD_MASK) / WORD_SIZE;
        let initial_val = if initial_set { !0u64 } else { 0u64 };
        let mut words_vec = Vec::with_capacity(word_count);
        for _ in 0..word_count {
            words_vec.push(AtomicU64::new(initial_val));
        }
        Self {
            words: words_vec.into_boxed_slice(),
            bit_count,
        }
    }

    #[inline(always)]
    pub fn bit_count(&self) -> usize {
        self.bit_count
    }

    #[inline(always)]
    pub fn is_set(&self, bit: usize) -> bool {
        if bit >= self.bit_count {
            return false;
        }
        let word_idx = bit >> WORD_SHIFT;
        let bit_in_word = bit & WORD_MASK;
        let mask = 1u64 << bit_in_word;
        (self.words[word_idx].load(Ordering::Acquire) & mask) != 0
    }

    pub fn is_set_range(&self, start: usize, end: usize) -> bool {
        if start >= self.bit_count || start > end {
            return false;
        }
        let clamped_end = end.min(self.bit_count - 1);
        let start_idx = start >> WORD_SHIFT;
        let start_bit = start & WORD_MASK;
        let start_mask = !0u64 << start_bit;

        let end_idx = clamped_end >> WORD_SHIFT;
        let end_bit = clamped_end & WORD_MASK;
        let end_mask = if end_bit == 63 {
            !0u64
        } else {
            (1u64 << (end_bit + 1)) - 1
        };

        if start_idx == end_idx {
            let mask = start_mask & end_mask;
            return (self.words[start_idx].load(Ordering::Acquire) & mask) != 0;
        }

        if (self.words[start_idx].load(Ordering::Acquire) & start_mask) != 0 {
            return true;
        }

        for i in (start_idx + 1)..end_idx {
            if self.words[i].load(Ordering::Acquire) != 0 {
                return true;
            }
        }

        (self.words[end_idx].load(Ordering::Acquire) & end_mask) != 0
    }

    #[inline(always)]
    pub fn set(&self, bit: usize, value: bool) {
        if bit >= self.bit_count {
            return;
        }
        let word_idx = bit >> WORD_SHIFT;
        let bit_in_word = bit & WORD_MASK;
        let mask = 1u64 << bit_in_word;

        if value {
            self.words[word_idx].fetch_or(mask, Ordering::Release);
        } else {
            self.words[word_idx].fetch_and(!mask, Ordering::Release);
        }
    }

    pub fn set_range(&self, start: usize, end: usize, value: bool) {
        if start >= self.bit_count || start > end {
            return;
        }
        let clamped_end = end.min(self.bit_count - 1);
        let start_idx = start >> WORD_SHIFT;
        let start_bit = start & WORD_MASK;
        let start_mask = !0u64 << start_bit;

        let end_idx = clamped_end >> WORD_SHIFT;
        let end_bit = clamped_end & WORD_MASK;
        let end_mask = if end_bit == 63 {
            !0u64
        } else {
            (1u64 << (end_bit + 1)) - 1
        };

        if start_idx == end_idx {
            let mask = start_mask & end_mask;
            if value {
                self.words[start_idx].fetch_or(mask, Ordering::Release);
            } else {
                self.words[start_idx].fetch_and(!mask, Ordering::Release);
            }
            return;
        }

        if value {
            self.words[start_idx].fetch_or(start_mask, Ordering::Release);
            for i in (start_idx + 1)..end_idx {
                self.words[i].store(!0u64, Ordering::Release);
            }
            self.words[end_idx].fetch_or(end_mask, Ordering::Release);
        } else {
            self.words[start_idx].fetch_and(!start_mask, Ordering::Release);
            for i in (start_idx + 1)..end_idx {
                self.words[i].store(0u64, Ordering::Release);
            }
            self.words[end_idx].fetch_and(!end_mask, Ordering::Release);
        }
    }

    pub fn any_set(&self) -> bool {
        self.words.iter().any(|w| w.load(Ordering::Acquire) != 0)
    }

    pub fn clear(&self) {
        for w in self.words.iter() {
            w.store(0, Ordering::Release);
        }
    }

    pub fn count_set_bits(&self) -> usize {
        self.words
            .iter()
            .map(|w| w.load(Ordering::Acquire).count_ones() as usize)
            .sum()
    }

    pub fn find_first_set(&self) -> Option<usize> {
        self.find_next_set(0)
    }

    pub fn find_next_set(&self, from_bit: usize) -> Option<usize> {
        if from_bit >= self.bit_count {
            return None;
        }
        let mut word_idx = from_bit >> WORD_SHIFT;
        let bit_in_word = from_bit & WORD_MASK;

        let first_word = self.words[word_idx].load(Ordering::Acquire) & (!0u64 << bit_in_word);
        if first_word != 0 {
            let bit = (word_idx << WORD_SHIFT) + (first_word.trailing_zeros() as usize);
            return if bit < self.bit_count {
                Some(bit)
            } else {
                None
            };
        }

        word_idx += 1;
        while word_idx < self.words.len() {
            let word = self.words[word_idx].load(Ordering::Acquire);
            if word != 0 {
                let bit = (word_idx << WORD_SHIFT) + (word.trailing_zeros() as usize);
                return if bit < self.bit_count {
                    Some(bit)
                } else {
                    None
                };
            }
            word_idx += 1;
        }
        None
    }
}
