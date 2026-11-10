const INLINE_CAP: usize = 8;

#[derive(Clone, Debug)]
pub struct ThreadQueue {
    inline: [u64; INLINE_CAP],
    len: usize,
    overflow: Vec<u64>,
}

impl Default for ThreadQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadQueue {
    pub const fn new() -> Self {
        Self {
            inline: [0; INLINE_CAP],
            len: 0,
            overflow: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn push_back(&mut self, thread_id: u64) {
        if self.len < INLINE_CAP {
            self.inline[self.len] = thread_id;
            self.len += 1;
        } else {
            self.overflow.push(thread_id);
            self.len += 1;
        }
    }

    #[inline]
    pub fn push_front(&mut self, thread_id: u64) {
        if self.len < INLINE_CAP {
            if self.len > 0 {
                self.inline.copy_within(0..self.len, 1);
            }
            self.inline[0] = thread_id;
            self.len += 1;
        } else {
            let last_inline = self.inline[INLINE_CAP - 1];
            self.inline.copy_within(0..INLINE_CAP - 1, 1);
            self.inline[0] = thread_id;
            self.overflow.insert(0, last_inline);
            self.len += 1;
        }
    }

    #[inline]
    pub fn remove(&mut self, thread_id: u64) -> bool {
        let inline_count = self.len.min(INLINE_CAP);
        for i in 0..inline_count {
            if self.inline[i] == thread_id {
                if self.len <= INLINE_CAP {
                    self.inline.copy_within(i + 1..self.len, i);
                    self.len -= 1;
                    return true;
                } else {
                    self.inline.copy_within(i + 1..INLINE_CAP, i);
                    self.inline[INLINE_CAP - 1] = self.overflow.remove(0);
                    self.len -= 1;
                    return true;
                }
            }
        }

        if self.len > INLINE_CAP {
            if let Some(pos) = self.overflow.iter().position(|&x| x == thread_id) {
                self.overflow.remove(pos);
                self.len -= 1;
                return true;
            }
        }

        false
    }

    #[inline]
    pub fn reschedule(&mut self, thread_id: u64) -> Option<u64> {
        if self.len == 0 {
            return None;
        }
        if self.len == 1 {
            return if self.inline[0] == thread_id {
                Some(thread_id)
            } else {
                None
            };
        }

        if self.remove(thread_id) {
            self.push_back(thread_id);
        }
        self.first()
    }

    #[inline(always)]
    pub fn first(&self) -> Option<u64> {
        if self.len > 0 {
            Some(self.inline[0])
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<u64> {
        if index >= self.len {
            None
        } else if index < INLINE_CAP {
            Some(self.inline[index])
        } else {
            Some(self.overflow[index - INLINE_CAP])
        }
    }

    pub fn export(&self, out: &mut [u64]) -> usize {
        let count = self.len.min(out.len());
        let inline_count = count.min(INLINE_CAP);
        out[..inline_count].copy_from_slice(&self.inline[..inline_count]);
        if count > INLINE_CAP {
            let overflow_count = count - INLINE_CAP;
            out[INLINE_CAP..count].copy_from_slice(&self.overflow[..overflow_count]);
        }
        count
    }
}
