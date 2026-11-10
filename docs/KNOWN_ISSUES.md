# Known Issues

These problems were found while writing the benchmark suite (November 2026).
None are fixed yet. Each is a good first issue; please reference the crate
name in the PR title.

Severity: **crash** = reachable panic or undefined behavior from guest data;
**wrong** = incorrect output; **limited** = works but does less than its
description says; **perf** = correct but needlessly slow.

## Crashes and undefined behavior

| Crate | Severity | Problem |
| :--- | :--- | :--- |
| `ryu-texture-astc` | crash | `decode_color_values` writes into `color_values: [i32; 32]`, but `decode_integer_sequence` can return more than 32 values (up to 4 partitions × 8, plus trit/quint rounding). Malformed blocks panic with an out-of-bounds index at `src/decoder.rs:778`. Verified: 8 of 10,000 random 16-byte blocks panic (e.g. `0f bb 16 3c 20 3a 7a 94 71 75 fd 56 96 97 be 4f`, 4×4). The release profile uses `panic = "abort"`, so through the C ABI this kills the emulator process. |
| `ryu-gal-queue` | UB | `GalSpscQueue::get_slot_ptr` and `GalSpanPool::try_insert` write through `as_ptr() as *mut u8` on a `Box<[u8]>` reached via `&self`, with no `UnsafeCell`. |
| `ryu-audio-dsp` | UB | `process_command_batch` creates a `&[f32]` and a `&mut [f32]` to the same buffer when a command's input and output index are equal (in-place Volume, Mix, biquad). |
| `ryu-pixel-converter`, `ryu-texture-bcn` | UB | `&[u8]` is reinterpreted as `&[u16]`/`&[u32]`/`&[u64]` with `from_raw_parts` without checking alignment. |
| `ryu-video-vic` | UB risk | The default `parallel` path of `blend_surface` writes through raw pointers without the length checks the serial path has. A short slice would write out of bounds. |
| `ryu-jit-regalloc` | UB | `BumpArena::alloc_slice` returns `&'static mut`, which dangles after `reset()`. `with_capacity` doesn't check the allocation for null. |
| `ryu-surfaceflinger-queue` | race | When the ring is full, `queue_buffer` advances `tail` from the producer side, racing the consumer in an SPSC design. The dropped frame's slot stays in `queued_mask` and never becomes free again. |
| most FFI layers | UB risk | About 160 `unsafe` blocks have no `// SAFETY:` comment, and several FFI functions don't null-check pointers. |

## Incorrect results

| Crate | Severity | Problem |
| :--- | :--- | :--- |
| `ryu-jit-regalloc` | wrong | `run_pass` assigns the same register to intervals that are live at the same time when fixed intervals are present: a fixed interval takes its register without evicting the current holder. With 3% fixed intervals, 38 conflicting pairs out of 512 intervals. (Ryubing currently only calls this crate's bitset helpers, not `run_pass`.) |
| `ryu-vulkan-descriptors` | wrong | `filter_redundant_buffers` / `filter_redundant_images` stop when the output buffer is full but still clear the whole dirty word and all later words, so dirty bindings beyond capacity are lost. |
| `ryu-kernel-sync` | wrong | `wake_waiters` removes every woken waiter but only copies out `out_woken.len()` of them; the rest are lost. |
| `ryu-ipc-parser` | wrong | `serialize_cmif` / `serialize_tipc` write send/receive/exchange descriptor counts into the header but never write the descriptors. |
| `ryu-romfs-indexer` | likely wrong | `romfs_hash` seeds with `parent ^ 12345678`. The Switch RomFS hash (as implemented in LibHac) is believed to use `123456789`. The crate's tests build their own images with the same function, so they pass either way. Needs checking against a real RomFS image. |
| `ryu-tegra-mme` | wrong | The runaway-macro guard in `MmeInterpreter::execute` compares the program counter with `code.len() + 1000`, not an instruction count, so an infinitely looping macro never stops. |
| `ryu-host1x-syncpoint` | wrong (edge) | Entries from plain `increment()` use id 0, which collides with real handle 0 after the handle counter wraps. |
| `ryu-video-nvdec` | wrong | `idct32x32_1024_add` tests for all-zero rows with an `as i16` cast, so coefficients that are multiples of 65536 count as zero. `decode_frame` writes U and V from the same DC value. |
| `ryu-gal-queue` | wrong (edge) | `GalSpanPool::try_insert` wraps when `prod + size >= pool_size`, so a payload that fits exactly is moved to offset 0. |
| `ryu-audio-mixer` | inconsistent | Rounds with `ceil`, while `ryu-audio-dsp` / `ryu-audio-fx` use round-half-to-even for the same operation. |

## Does less than described

| Crate | Problem |
| :--- | :--- |
| `ryu-video-nvdec` | **Not a VP9 decoder.** The header parser reads only the frame marker, type and size; other fields are hard-coded. `decode_frame` reads one skip bit, one has-coefficients bit and one DC byte per 16×16 block, then runs a DC-only IDCT and the loop filter. No intra/inter prediction, no coefficient token decoding. The IDCT and loop-filter kernels themselves are real. |
| `ryu-shader-transpiler` | **Output is not valid SPIR-V.** Operands are fresh IDs that are never defined, `FFMA` is emitted as `FMul`, `BRA` ignores its target, and MOV/LDC/TEX and other opcodes emit nothing. Unknown opcodes are guessed from bits 20–23. Translation stops at the first `EXIT`. |
| `ryu-audio-fx` | Crate docs mention Delay and Reverb; only biquad filters exist. |
| `ryu-interval-tree` | `FlatIntervalTree` is a sorted `Vec` with linear-scan queries and O(n) insert/remove, not a tree. |
| `ryu-common-hash` | The xxHash3 implementation is the `twox-hash` crate; this crate adds batching and the C ABI. |
| most crates | Descriptions say "SIMD", but only `ryu-audio-mixer`, `ryu-buffer-diff`, `ryu-interval-tree`, `ryu-jit-regalloc` and `ryu-maxwell-state` contain explicit SIMD intrinsics. |

## Performance

| Crate | Problem |
| :--- | :--- |
| `ryu-host1x-syncpoint` | `signal_done` searches the queue linearly: O(n²) for a frame completed in reverse order. |
| `ryu-romfs-indexer` | `RyuRomFs::find_file` rebuilds a `RomFsReader` on every call. Compare `find_file` with `reader_find_file` in the benchmarks. |
| `ryu-storage-fs` | `decompress_zstd_block` creates a new zstd context per call. `find_pfs0_file` re-parses the header twice per entry. |
| `ryu-shader-cache` | CRC32 is a byte-at-a-time table loop. The LZ4 decoder copies overlapping matches one byte at a time. |
| `ryu-jit-regalloc` | Spilling ignores weight and next use; `sort_by_key` allocates despite the zero-allocation goal. |
| `ryu-audio-mixer` | Calls `is_x86_feature_detected!` on every mix call. |

## Other notes

- `ryu-maxwell-state`: `scan_changes` ORs into the dirty mask and never clears it; callers must clear it.
- `ryu-audio-dsp` / `ryu-audio-fx`: biquad feedback coefficients (`a1`, `a2`) must be stored **negated**. This is undocumented.
- `ryu-audio-resampler`: in `resample_low`, the `frac > 1.0` branch can't be reached.
- `ryu-pixel-converter`: `convert_bgra8_to_rgba8` silently ignores trailing bytes when the length isn't a multiple of 4.
- `ryu-shader-cache`: `parse_toc_entries` doesn't validate the header, and truncated literals are reported as `OutputBufferTooSmall` rather than as corrupt input.
- `ryu-surfaceflinger-queue`: `queue_buffer` doesn't check that the slot was dequeued; `find_oldest_free_slot` is O(free slots), not O(1) as its comment says.
