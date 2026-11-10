//! Sharded High-Performance Kernel Address Arbiter Registry.

use std::collections::HashMap;
use std::sync::Mutex;

const SHARD_COUNT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waiter {
    pub thread_id: u64,
    pub priority: i32,
}

#[derive(Default)]
struct ShardData {
    waiters: HashMap<u64, Vec<Waiter>>,
}

impl ShardData {
    fn insert_waiter(&mut self, address: u64, thread_id: u64, priority: i32) {
        let list = self
            .waiters
            .entry(address)
            .or_insert_with(|| Vec::with_capacity(4));
        let waiter = Waiter {
            thread_id,
            priority,
        };

        // Binary search insertion preserving priority order (ascending: lower value = higher priority)
        let idx = match list.binary_search_by_key(&priority, |w| w.priority) {
            Ok(i) => i,
            Err(i) => i,
        };
        list.insert(idx, waiter);
    }

    fn remove_waiter(&mut self, address: u64, thread_id: u64) -> bool {
        if let Some(list) = self.waiters.get_mut(&address) {
            if let Some(pos) = list.iter().position(|w| w.thread_id == thread_id) {
                list.remove(pos);
                if list.is_empty() {
                    self.waiters.remove(&address);
                }
                return true;
            }
        }
        false
    }

    fn wake_waiters(&mut self, address: u64, count: i32, out_woken: &mut [u64]) -> usize {
        if let Some(list) = self.waiters.get_mut(&address) {
            let valid_count = if count <= 0 {
                list.len()
            } else {
                list.len().min(count as usize)
            };

            let to_copy = valid_count.min(out_woken.len());
            for i in 0..to_copy {
                out_woken[i] = list[i].thread_id;
            }

            list.drain(0..valid_count);

            if list.is_empty() {
                self.waiters.remove(&address);
            }

            to_copy
        } else {
            0
        }
    }

    fn get_waiter_count(&self, address: u64) -> usize {
        self.waiters.get(&address).map(|l| l.len()).unwrap_or(0)
    }

    fn clear(&mut self) {
        self.waiters.clear();
    }
}

/// Sharded Kernel Address Arbiter Registry for zero lock contention.
pub struct ArbiterRegistry {
    shards: [Mutex<ShardData>; SHARD_COUNT],
}

impl ArbiterRegistry {
    pub fn new() -> Self {
        // Initialize 64 independent shards
        let shards = std::array::from_fn(|_| Mutex::new(ShardData::default()));
        Self { shards }
    }

    #[inline(always)]
    fn get_shard_index(address: u64) -> usize {
        // High-quality hash mixing for 64-bit guest addresses
        let mut h = address;
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        (h as usize) % SHARD_COUNT
    }

    pub fn insert_waiter(&self, address: u64, thread_id: u64, priority: i32) {
        let shard_idx = Self::get_shard_index(address);
        let mut shard = self.shards[shard_idx].lock().unwrap();
        shard.insert_waiter(address, thread_id, priority);
    }

    pub fn remove_waiter(&self, address: u64, thread_id: u64) -> bool {
        let shard_idx = Self::get_shard_index(address);
        let mut shard = self.shards[shard_idx].lock().unwrap();
        shard.remove_waiter(address, thread_id)
    }

    pub fn wake_waiters(&self, address: u64, count: i32, out_woken: &mut [u64]) -> usize {
        let shard_idx = Self::get_shard_index(address);
        let mut shard = self.shards[shard_idx].lock().unwrap();
        shard.wake_waiters(address, count, out_woken)
    }

    pub fn get_waiter_count(&self, address: u64) -> usize {
        let shard_idx = Self::get_shard_index(address);
        let shard = self.shards[shard_idx].lock().unwrap();
        shard.get_waiter_count(address)
    }

    pub fn compute_signal_modify_addend(&self, address: u64, count: i32) -> i32 {
        let shard_idx = Self::get_shard_index(address);
        let shard = self.shards[shard_idx].lock().unwrap();
        let waiting_count = shard.get_waiter_count(address);

        if waiting_count > 0 {
            if count <= 0 {
                -2
            } else if waiting_count < (count as usize) {
                -1
            } else {
                0
            }
        } else {
            1
        }
    }

    pub fn clear(&self) {
        for shard in &self.shards {
            shard.lock().unwrap().clear();
        }
    }
}

impl Default for ArbiterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
