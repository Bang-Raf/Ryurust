//! Balanced interval tree used for texture / buffer cache overlap queries.
//!
//! The tree is an AVL tree keyed by `(start, end, value)` where every node also
//! stores the largest `end` found in its subtree (the classic "max-end
//! augmentation", same idea as the red-black `IntervalTree` in
//! `Ryujinx.Common.Collections`). Nodes live in one `Vec` and refer to each
//! other through `u32` indices; removed slots are recycled through a free list,
//! so there is no per-node allocation and no `unsafe`.

use std::cmp::Ordering;

/// Index value meaning "no node".
const NIL: u32 = u32::MAX;

/// Upper bound on the height of the tree, used to size the fixed traversal
/// stacks. An AVL tree of height `h` holds at least `Fib(h + 2) - 1` nodes, and
/// the arena can hold fewer than `2^32` nodes, so the height never exceeds 46.
const MAX_HEIGHT: usize = 48;

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

#[derive(Debug, Clone, Copy)]
struct Node {
    start: u64,
    end: u64,
    value: u64,
    /// Largest `end` of any interval in the subtree rooted at this node.
    max_end: u64,
    /// Left child, or the next free slot while this node is on the free list.
    left: u32,
    right: u32,
    /// Height of the subtree rooted here (a leaf has height 1).
    height: u8,
}

impl Node {
    #[inline(always)]
    fn key(&self) -> (u64, u64, u64) {
        (self.start, self.end, self.value)
    }
}

/// Interval multiset with overlap queries over half-open ranges `[start, end)`.
///
/// Despite the name, this is no longer a flat sorted array: it is a balanced
/// (AVL) interval tree stored in an index-based arena. The name is kept because
/// renaming the type would break existing users and the C ABI wrappers.
///
/// # Complexity
///
/// * `insert` / `remove`: `O(log n)`.
/// * `find_overlaps*`: `O(log n + k)` for the usual cache workloads, where `k`
///   is the number of reported intervals. As with any max-end-augmented search
///   tree the worst case is `O(min(n, (k + 1) log n))`, reached when the
///   reported intervals are scattered between many non-overlapping ones.
///
/// # Semantics
///
/// * An interval is reported when `start < q_end && q_start < end`; queries
///   with `q_start >= q_end` report nothing.
/// * The same `(start, end, value)` triple may be inserted more than once; each
///   `remove` takes out one copy.
/// * Results are reported in ascending `start` order. Intervals sharing a
///   `start` are ordered by `end`, then by `value`.
#[derive(Debug, Clone)]
pub struct FlatIntervalTree {
    nodes: Vec<Node>,
    root: u32,
    free_head: u32,
    len: usize,
}

impl Default for FlatIntervalTree {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl FlatIntervalTree {
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            root: NIL,
            free_head: NIL,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root = NIL;
        self.free_head = NIL;
        self.len = 0;
    }

    pub fn insert(&mut self, start: u64, end: u64, value: u64) {
        let new = self.alloc(start, end, value);
        self.len += 1;

        if self.root == NIL {
            self.root = new;
            return;
        }

        let key = (start, end, value);
        let mut path = [NIL; MAX_HEIGHT];
        let mut depth = 0;
        let mut cur = self.root;
        loop {
            let node = &mut self.nodes[cur as usize];
            // Every node on the way down gains the new interval in its subtree.
            if node.max_end < end {
                node.max_end = end;
            }
            path[depth] = cur;
            depth += 1;

            // Equal keys go right; duplicates are indistinguishable anyway.
            let link = if key < node.key() {
                &mut node.left
            } else {
                &mut node.right
            };
            if *link == NIL {
                *link = new;
                break;
            }
            cur = *link;
        }

        // `max_end` is already correct above, so the walk back up can stop as
        // soon as a subtree keeps the height it had before the insertion.
        self.retrace(&path[..depth], true);
    }

    pub fn remove(&mut self, start: u64, end: u64, value: u64) -> bool {
        let key = (start, end, value);
        let mut path = [NIL; MAX_HEIGHT];
        let mut depth = 0;
        let mut cur = self.root;
        loop {
            if cur == NIL {
                return false;
            }
            let node = &self.nodes[cur as usize];
            let next = match key.cmp(&node.key()) {
                Ordering::Less => node.left,
                Ordering::Greater => node.right,
                Ordering::Equal => break,
            };
            path[depth] = cur;
            depth += 1;
            cur = next;
        }

        let target = cur;
        let (left, right) = {
            let node = &self.nodes[target as usize];
            (node.left, node.right)
        };

        // `victim` is the slot that actually leaves the tree; it has at most
        // one child, which takes its place.
        let (victim, child) = if left != NIL && right != NIL {
            // Two children: move the in-order successor's interval into
            // `target` and unlink the successor instead.
            path[depth] = target;
            depth += 1;
            let mut succ = right;
            loop {
                let next = self.nodes[succ as usize].left;
                if next == NIL {
                    break;
                }
                path[depth] = succ;
                depth += 1;
                succ = next;
            }
            let s = self.nodes[succ as usize];
            let node = &mut self.nodes[target as usize];
            node.start = s.start;
            node.end = s.end;
            node.value = s.value;
            (succ, s.right)
        } else if left != NIL {
            (target, left)
        } else {
            (target, right)
        };

        if depth == 0 {
            self.root = child;
        } else {
            let parent = &mut self.nodes[path[depth - 1] as usize];
            if parent.left == victim {
                parent.left = child;
            } else {
                parent.right = child;
            }
        }

        self.release(victim);
        self.len -= 1;

        // `max_end` can shrink anywhere on the path, so walk all the way up.
        self.retrace(&path[..depth], false);
        true
    }

    /// Appends the values of all intervals overlapping `[q_start, q_end)` to
    /// `out`, in ascending `(start, end, value)` order.
    pub fn find_overlaps(&self, q_start: u64, q_end: u64, out: &mut Vec<u64>) {
        self.for_each_overlap(q_start, q_end, |value| out.push(value));
    }

    /// Writes the values of the overlapping intervals to `out` (same order as
    /// [`find_overlaps`](Self::find_overlaps)) and returns the total number of
    /// overlaps, which can exceed `out.len()`; the excess is counted but not
    /// stored.
    pub fn find_overlaps_slice(&self, q_start: u64, q_end: u64, out: &mut [u64]) -> usize {
        let mut count = 0;
        self.for_each_overlap(q_start, q_end, |value| {
            if let Some(slot) = out.get_mut(count) {
                *slot = value;
            }
            count += 1;
        });
        count
    }

    /// Same as [`find_overlaps`](Self::find_overlaps). Kept for API
    /// compatibility: the sorted-array implementation had separate scalar and
    /// AVX2 scans, the tree has a single query path.
    #[inline(always)]
    pub fn find_overlaps_scalar(&self, q_start: u64, q_end: u64, out: &mut Vec<u64>) {
        self.for_each_overlap(q_start, q_end, |value| out.push(value));
    }

    /// In-order walk that skips every subtree whose `max_end` shows it cannot
    /// reach the query, and stops at the first node starting at or after
    /// `q_end`.
    #[inline(always)]
    fn for_each_overlap(&self, q_start: u64, q_end: u64, mut report: impl FnMut(u64)) {
        if q_start >= q_end {
            return;
        }

        let nodes = self.nodes.as_slice();
        let mut stack = [NIL; MAX_HEIGHT];
        let mut sp = 0;
        let mut cur = self.root;
        loop {
            while cur != NIL {
                let node = &nodes[cur as usize];
                if node.max_end <= q_start {
                    // Nothing below ends after the query starts.
                    break;
                }
                if node.start < q_end {
                    stack[sp] = cur;
                    sp += 1;
                }
                // Otherwise this node and its right subtree start too late;
                // only the left subtree can still hold overlaps.
                cur = node.left;
            }

            if sp == 0 {
                break;
            }
            sp -= 1;
            let node = &nodes[stack[sp] as usize];
            if node.end > q_start {
                report(node.value);
            }
            cur = node.right;
        }
    }

    fn alloc(&mut self, start: u64, end: u64, value: u64) -> u32 {
        let node = Node {
            start,
            end,
            value,
            max_end: end,
            left: NIL,
            right: NIL,
            height: 1,
        };
        if self.free_head != NIL {
            let idx = self.free_head;
            self.free_head = self.nodes[idx as usize].left;
            self.nodes[idx as usize] = node;
            idx
        } else {
            assert!(
                self.nodes.len() < NIL as usize,
                "interval tree is limited to u32::MAX - 1 entries"
            );
            self.nodes.push(node);
            (self.nodes.len() - 1) as u32
        }
    }

    fn release(&mut self, idx: u32) {
        self.nodes[idx as usize].left = self.free_head;
        self.free_head = idx;
    }

    /// Rebalances the nodes in `path` (root first) from the deepest one up,
    /// re-linking rotated subtrees into their parents.
    fn retrace(&mut self, path: &[u32], stop_when_height_unchanged: bool) {
        let mut depth = path.len();
        while depth > 0 {
            depth -= 1;
            let idx = path[depth];
            let old_height = self.nodes[idx as usize].height;
            let new_root = self.rebalance(idx);
            if new_root != idx {
                if depth == 0 {
                    self.root = new_root;
                } else {
                    let parent = &mut self.nodes[path[depth - 1] as usize];
                    if parent.left == idx {
                        parent.left = new_root;
                    } else {
                        parent.right = new_root;
                    }
                }
            }
            if stop_when_height_unchanged && self.nodes[new_root as usize].height == old_height {
                break;
            }
        }
    }

    #[inline(always)]
    fn height(&self, idx: u32) -> u8 {
        if idx == NIL {
            0
        } else {
            self.nodes[idx as usize].height
        }
    }

    /// Recomputes `height` and `max_end` of `idx` from its children.
    #[inline(always)]
    fn update(&mut self, idx: u32) {
        let node = self.nodes[idx as usize];
        let mut height = 0;
        let mut max_end = node.end;
        if node.left != NIL {
            let left = &self.nodes[node.left as usize];
            height = left.height;
            max_end = max_end.max(left.max_end);
        }
        if node.right != NIL {
            let right = &self.nodes[node.right as usize];
            height = height.max(right.height);
            max_end = max_end.max(right.max_end);
        }
        let node = &mut self.nodes[idx as usize];
        node.height = height + 1;
        node.max_end = max_end;
    }

    /// Refreshes `idx` and restores the AVL balance of its subtree, returning
    /// the index of the subtree's (possibly new) root.
    fn rebalance(&mut self, idx: u32) -> u32 {
        self.update(idx);
        let (left, right) = {
            let node = &self.nodes[idx as usize];
            (node.left, node.right)
        };
        let left_height = self.height(left);
        let right_height = self.height(right);

        if left_height > right_height + 1 {
            let l = self.nodes[left as usize];
            if self.height(l.left) < self.height(l.right) {
                let new_left = self.rotate_left(left);
                self.nodes[idx as usize].left = new_left;
            }
            self.rotate_right(idx)
        } else if right_height > left_height + 1 {
            let r = self.nodes[right as usize];
            if self.height(r.right) < self.height(r.left) {
                let new_right = self.rotate_right(right);
                self.nodes[idx as usize].right = new_right;
            }
            self.rotate_left(idx)
        } else {
            idx
        }
    }

    fn rotate_left(&mut self, idx: u32) -> u32 {
        let pivot = self.nodes[idx as usize].right;
        self.nodes[idx as usize].right = self.nodes[pivot as usize].left;
        self.nodes[pivot as usize].left = idx;
        self.update(idx);
        self.update(pivot);
        pivot
    }

    fn rotate_right(&mut self, idx: u32) -> u32 {
        let pivot = self.nodes[idx as usize].left;
        self.nodes[idx as usize].left = self.nodes[pivot as usize].right;
        self.nodes[pivot as usize].right = idx;
        self.update(idx);
        self.update(pivot);
        pivot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Subtree {
        height: u8,
        max_end: u64,
        count: usize,
    }

    impl FlatIntervalTree {
        /// Checks ordering, AVL balance, stored heights, the max-end
        /// augmentation and the arena / free-list bookkeeping.
        fn validate(&self) {
            let mut in_order = Vec::with_capacity(self.len);
            let count = if self.root == NIL {
                0
            } else {
                self.validate_subtree(self.root, &mut in_order).count
            };
            assert_eq!(count, self.len, "reachable node count != len");
            assert!(
                in_order.windows(2).all(|w| w[0] <= w[1]),
                "in-order keys are not sorted"
            );

            let mut free = 0;
            let mut idx = self.free_head;
            while idx != NIL {
                free += 1;
                assert!(free <= self.nodes.len(), "free list has a cycle");
                idx = self.nodes[idx as usize].left;
            }
            assert_eq!(self.nodes.len(), self.len + free, "leaked arena slots");

            // AVL height bound: h <= 1.4405 * log2(n + 2).
            let bound = (1.4405 * ((self.len + 2) as f64).log2()).floor() as u8;
            assert!(self.height(self.root) <= bound, "tree is too tall");
        }

        fn validate_subtree(&self, idx: u32, in_order: &mut Vec<(u64, u64, u64)>) -> Subtree {
            let node = self.nodes[idx as usize];
            let mut height = 0;
            let mut max_end = node.end;
            let mut count = 1;
            let mut balance = 0i32;

            if node.left != NIL {
                let left = self.validate_subtree(node.left, in_order);
                height = left.height;
                max_end = max_end.max(left.max_end);
                count += left.count;
                balance += i32::from(left.height);
            }
            in_order.push(node.key());
            if node.right != NIL {
                let right = self.validate_subtree(node.right, in_order);
                height = height.max(right.height);
                max_end = max_end.max(right.max_end);
                count += right.count;
                balance -= i32::from(right.height);
            }

            assert!(balance.abs() <= 1, "node {idx} is unbalanced ({balance})");
            assert_eq!(node.height, height + 1, "node {idx} has a stale height");
            assert_eq!(node.max_end, max_end, "node {idx} has a stale max_end");
            Subtree {
                height: height + 1,
                max_end,
                count,
            }
        }
    }

    fn xorshift(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    #[test]
    fn invariants_hold_after_every_random_operation() {
        let mut rng = 0x9E37_79B9_7F4A_7C15u64;
        let mut tree = FlatIntervalTree::default();
        let mut live: Vec<(u64, u64, u64)> = Vec::new();
        tree.validate();

        for step in 0..4000u32 {
            // Grow for a while, then shrink, so both directions get deep trees.
            let insert_bias = if (step / 500) % 2 == 0 { 70 } else { 30 };
            if live.is_empty() || xorshift(&mut rng) % 100 < insert_bias {
                // Small coordinate space: plenty of equal starts and exact duplicates.
                let start = xorshift(&mut rng) % 64;
                let end = start + xorshift(&mut rng) % 16;
                let value = xorshift(&mut rng) % 4;
                tree.insert(start, end, value);
                live.push((start, end, value));
            } else {
                let pick = (xorshift(&mut rng) % live.len() as u64) as usize;
                let (start, end, value) = live.swap_remove(pick);
                assert!(tree.remove(start, end, value));
            }
            tree.validate();
            assert_eq!(tree.len(), live.len());
        }

        for (start, end, value) in live.drain(..) {
            assert!(tree.remove(start, end, value));
            tree.validate();
        }
        assert!(tree.is_empty());
        assert_eq!(tree.root, NIL);
    }

    #[test]
    fn invariants_hold_for_sorted_insertion_and_removal() {
        // Monotonic keys are the worst case for an unbalanced tree.
        let mut tree = FlatIntervalTree::new();
        for i in 0..600u64 {
            tree.insert(i * 0x1000, i * 0x1000 + 0x4000, i);
            tree.validate();
        }
        for i in (0..600u64).rev() {
            tree.insert(i * 0x1000, i * 0x1000 + 0x100, i);
            tree.validate();
        }
        for i in 0..600u64 {
            assert!(tree.remove(i * 0x1000, i * 0x1000 + 0x4000, i));
            tree.validate();
        }
        for i in (0..600u64).rev() {
            assert!(tree.remove(i * 0x1000, i * 0x1000 + 0x100, i));
            tree.validate();
        }
        assert!(tree.is_empty());
    }

    #[test]
    fn removed_slots_are_reused() {
        let mut tree = FlatIntervalTree::new();
        for i in 0..100u64 {
            tree.insert(i, i + 10, i);
        }
        let slots = tree.nodes.len();
        for round in 0..20u64 {
            for i in 0..50u64 {
                assert!(tree.remove(i * 2, i * 2 + 10, i * 2));
            }
            for i in 0..50u64 {
                tree.insert(i * 2, i * 2 + 10, i * 2);
            }
            tree.validate();
            assert_eq!(tree.nodes.len(), slots, "arena grew in round {round}");
        }
    }
}
