# Known Issues

Last reviewed: 2026-11-13.

This page lists what is still wrong or unfinished, followed by the issues
fixed before the first public release (so reviewers know what changed and
why). Anything still open is a good first issue; reference the crate name in
the PR title.

## Open

### Incomplete crates

| Crate | Problem |
| :--- | :--- |
| `ryu-video-nvdec` | **Not a VP9 decoder.** It provides VP9 inverse transforms (IDCT/IADST) and the loop filter, plus a synthetic DC-only reconstruction used for testing and benchmarking. There is no bitstream/coefficient token decoding and no intra/inter prediction. `idct32` also truncates its inputs to `i16` (fine for 8-bit content, not for high bit depth). |
| `ryu-shader-transpiler` | **Output is not valid SPIR-V.** Operands are fresh IDs that are never defined, `FFMA` is emitted as `FMul`, `BRA` ignores its target, and MOV/LDC/TEX and other opcodes emit nothing. Unknown opcodes are guessed from bits 20–23 and translation stops at the first `EXIT`. Treat it as a prototype. |

### Correctness gaps and edge cases

| Crate | Problem |
| :--- | :--- |
| `ryu-texture-astc` | Invalid blocks are written as opaque magenta; C# leaves those texels as zeros. The status code is the same (failure), so only the texel values differ. Deliberate, but a C# caller that switches to the native decoder will see magenta instead of black. |
| `ryu-jit-regalloc` | Spill choice uses the furthest interval end (Poletto/Sarkar), not C#'s next-use position plus interval splitting: this crate's intervals carry no use lists and are never split. Fixed intervals with `fixed_reg` outside `0..32` are not tracked (the register masks are `u32`), so conflicts between them go undetected. |
| `ryu-texture-astc` | Matching C# made decoding slower: about 3–8% per valid block, and 12–35% per invalid block, because blocks that used to be rejected early now go through the same steps as in C#. Real textures are almost all valid blocks; the Criterion benchmark uses random blocks (~94% invalid), so its numbers overstate the cost. See [BENCHMARKS.md](BENCHMARKS.md). |
| `ryu-interval-tree` | At n = 128 the balanced tree is no faster than the old sorted array, and the slice query may be 5–15% slower (within benchmark noise). From a few hundred entries up it is clearly faster. |
| `ryu-video-nvdec` | `stride * n` / `height * y_stride` aren't overflow-checked. A wrapped product gives a too-short slice and a panic (the process aborts at the C boundary), not memory corruption. |
| `ryu-memory-pagetable` | `map_range` / `unmap_range` with a `size` near `u64::MAX` loop over that many pages. No overflow, but effectively a hang; callers must pass real sizes. |

### Code-quality debt

- Inside `pub unsafe extern "C" fn` bodies (edition 2021) unsafe operations
  aren't in their own blocks, so they're covered by the function's
  `# Safety` doc rather than a per-operation `// SAFETY:` comment. Moving to
  edition 2024 (`unsafe_op_in_unsafe_fn`) would make that explicit.
- The C# wrappers don't log whether a native library was loaded, so a log
  can't show whether the Rust path or the C# fallback ran.
- `NativeJitRegalloc.cs` has no binding for the new
  `ryu_regalloc_get_fixed_conflict` export (nothing in Ryubing calls the
  allocator part of the wrapper yet).

## Gameplay testing

All sessions so far ran on one machine: Fedora 44, Intel Core Ultra 7 256V
with integrated Arc graphics (Mesa 26.2.5, Vulkan), firmware 22.5.0.

### v0.1.1 (2026-11-13)

The package was installed from `ghcr.io/bang-raf/ryurust:0.1.1`. Both games
were played in one emulator session (9.5 minutes).

| Game | Session length | Result |
| :--- | :--- | :--- |
| NARUTO X BORUTO Ultimate Ninja STORM CONNECTIONS v1.7.0 | 6 min | No crash, clean stop |
| The Legend of Zelda: Tears of the Kingdom v1.4.3 | 3 min | No crash, clean stop |

What the log showed:

- The only error lines are the same `SurfaceFlinger DequeueBuffer` pair as in
  v0.1.0, once per game at the moment it is stopped. Nothing is logged as an
  error while a game runs.
- Warnings: `GPU processing thread is too slow` 6 times (the NARUTO shader
  cache started empty), JIT cache growth twice in TOTK, and the
  `TerminationRequested` lines of guest threads exiting at stop. The same
  kinds of warning appear in the v0.1.0 log, and no new kind.
- NARUTO logs `ServiceAcc CheckAvailability` and `GetAccountId` stubs about
  30 times a second (about 20,000 lines in 6 minutes). This is upstream
  Ryubing's stub for the online account service, which Ryurust doesn't touch.

Limits: this is the first build in which the native RomFS index can be
created, and both games loaded and streamed RomFS data without file-system
errors. But the C# wrappers still don't log which path ran (see "Code-quality
debt"), so the log can't show whether the Rust index or the C# fallback
served those lookups. No frame-rate measurements were taken.

### v0.1.0 (2026-11-11)

The package was installed from `ghcr.io/bang-raf/ryurust:0.1.0`.

| Game | Session length | Result |
| :--- | :--- | :--- |
| The Legend of Zelda: Tears of the Kingdom v1.4.3 | 4.5 min | No crash, clean stop |
| The Legend of Zelda: Tears of the Kingdom v1.4.3 | 22 min | No crash, clean stop |
| Super Mario Odyssey v1.4.1 | 8 min | No crash, clean stop |

What the logs showed:

- The only error lines are
  `SurfaceFlinger DequeueBuffer: No available buffer slots` followed by
  `OnTransact: ... DequeueBuffer: Busy`, once per session at the moment the
  game is stopped. A Ryubing build without Ryurust logs the same pair at
  shutdown on the same machine, so this is not caused by the Ryurust
  SurfaceFlinger queue.
- `ServiceNv Wait: GPU processing thread is too slow` appeared 18 times
  across the three sessions, all with a cold shader cache.

Limits of this test:

- One machine, two games, 35 minutes in total. No frame-rate or frame-time
  measurements were taken, and there is no comparison against a build without
  Ryurust.
- The C# wrappers don't log whether a native library was loaded, so the logs
  don't prove that the ten wired subsystems ran the Rust code rather than the
  C# fallback. The libraries were present in `lib/`, one of the paths the
  wrappers probe.
- Found afterwards: in this build the native RomFS index could never be
  created (see "Fixed in v0.1.1"), so RomFS lookups in these sessions all
  used the C# path.

## Fixed in v0.1.1

Found while closing the issues listed as open in v0.1.0. All fixes come with
regression tests. No exported function was removed or changed its signature
(checked against the symbols in the v0.1.0 libraries: all 252 still present,
21 added).

### Crashes and undefined behavior

| Crate | Fix |
| :--- | :--- |
| `ryu-tegra-swizzle` | All four `convert_*` functions wrote past the end of the destination for 3-, 6- and 12-byte pixels (the row-tail loop restarted from a rounded-down pixel index). The tail is now copied as one block; output for 1/2/4/8/16-byte pixels is unchanged. |
| `ryu-gal-queue` | `ryu_spsc_create`: `capacity * element_size` could overflow and allocate a buffer smaller than the slots it hands out; now returns null. `ryu_span_pool_get`: `offset + size` could wrap past the bounds check; now uses `checked_add`. |
| `ryu-jit-regalloc` | `ryu_regalloc_bitset_or` / `_and_not` created overlapping `&mut` and `&` slices when C# passes the same buffer as `dst` and `src`. That case is now handled explicitly (same results). |
| `ryu-audio-dsp` | `ryu_dsp_process_command_batch` used `saturating_mul` for `buffer_count * sample_count`, so an overflow built a `usize::MAX`-length slice. Now returns `DSP_STATUS_INVALID_ARGUMENT`. |
| `ryu-texture-astc` | Panicked on 3D textures whose height isn't a multiple of the block height (integer underflow), on block sizes above 12x12, and on a too-short output buffer. The first now writes nothing for those blocks, like C#; the others return failure. |
| `ryu-romfs-indexer` | A corrupted image with a cyclic hash chain looped forever. Chain walks are now bounded by the table size. |
| `ryu-surfaceflinger-queue` | `ryu_sf_parcel_write_header` overflowed `u32` for huge sizes (panic in debug, wrong size in release). Now returns `0`. |
| `ryu-memory-pagetable` | `map_range` / `unmap_range` overflowed computing the page count for `size` near `u64::MAX`. |
| `ryu-vulkan-descriptors`, `ryu-texture-bcn`, `ryu-pixel-converter` (tests) | Tests reinterpreted a `Vec<u8>` as a slice of 8-byte-aligned structs, or passed one buffer as both source and destination. Fixed. |

### Incorrect results

| Crate | Fix |
| :--- | :--- |
| `ryu-romfs-indexer` | Rejected every image built by LibHac: it assumed the tables come before the file data, but LibHac (and real games) put them after. Found by a new test against an image built with LibHac's `RomFsBuilder` (synthetic content, `tests/fixtures/`). The C# side in Ryubing only passed the bytes before the data, so in v0.1.0 the native RomFS index was never used and lookups always fell back to C#. Both sides are fixed; the C# change is in `integration/ryubing/ryurust-integration.patch`. |
| `ryu-texture-astc` | Checked byte for byte against the C# `AstcDecoder` with new golden fixtures (`tests/golden/`, plus a 280,000-block run). That found: dual-plane blocks applied the second weight plane to the wrong channel; blocks whose weights use more bits than are left were rejected (C# decodes their colours as zero); two edge cases in the colour-precision search. All fixed. |
| `ryu-texture-astc` | `decode_astc_to_rgba8` returned success when some blocks were invalid. It now returns failure, like C# `TryDecodeToRgba8` (FFI: `1`). |
| `ryu-jit-regalloc` | Two overlapping fixed intervals demanding the same register both got it. This is now detected before allocation: `run_pass` returns `usize::MAX`, nothing is allocated, and `ryu_regalloc_get_fixed_conflict` reports the pair. Spilling now picks the non-fixed interval that ends furthest away instead of always the current one. |
| `ryu-ipc-parser` | `serialize_tipc` silently dropped pointer (X) buffers. TIPC has none (C# asserts there are none), so such a message is now rejected. |

### Performance

| Crate | Fix |
| :--- | :--- |
| `ryu-interval-tree` | Was a sorted `Vec` with linear queries and O(n) insert/remove. Now an AVL interval tree in an index arena: O(log n) insert/remove, O(log n + k) typical queries. Checked against a naive model over 50,000 random operations. See [BENCHMARKS.md](BENCHMARKS.md). |
| `ryu-jit-regalloc` | Sorting unsorted input no longer allocates. |

### FFI and code quality

- Every crate exports `ryu_<crate>_version()`. The older names
  `ryu_ipc_version`, `ryu_scheduler_version`, `ryu_surfaceflinger_version`
  and `ryu_nvdec_get_version` stay as aliases.
- Every pointer-taking export is `unsafe extern "C"`, has a `# Safety` doc,
  and is tested with null pointers. (All of them already null-checked; the
  tests now pin that.)
- `unsafe` blocks have `// SAFETY:` comments.
- The 26 clippy lints allowed workspace-wide are gone. Two intentional
  exceptions are allowed locally with a reason: `excessive_precision` in the
  resampler's coefficient table, and `too_many_arguments` on functions whose
  arguments mirror a C# or C ABI signature.

### Behavior changes visible to C#

- `ryu_astc_decode_to_rgba8[_parallel]`: returns `1` (was `0`) when any block
  is invalid. Not called from Ryubing yet.
- `ryu_regalloc_run_pass`: returns `usize::MAX` (`-1` after the wrapper's
  `int` cast) on a fixed-register conflict, and spill choices differ. The
  wrapped `RunPass` has no callers in Ryubing.
- `ryu_ipc_serialize_tipc`: returns `-2` for a message with pointer buffers.
- `ryu_romfs_create`: needs the header and all four tables. Passing only the
  bytes before `data_offset` no longer works for LibHac-layout images (it never
  did in practice; see above).
- `ryu_interval_tree_find_overlaps`: entries with the same start come out
  ordered by end, then value (was unspecified).

## Fixed before v0.1.0

All fixes come with regression tests. No exported `extern "C"` signature
changed (checked: 253 signatures identical before and after).

### Crashes and undefined behavior

| Crate | Fix |
| :--- | :--- |
| `ryu-texture-astc` | Malformed blocks could panic (out-of-bounds write in `decode_color_values`, plus a shift overflow in `select_partition` and an integer underflow in `read_bits`). Invalid blocks now decode to the ASTC error colour (`0xFFFF00FF`, magenta). Tested with 100,000 random blocks each for 4×4 and 8×8, and a one-off 4.2 M-block fuzz across all 14 block sizes. `select_partition` and `hash52` were also re-ported from C# (they decoded partitioned blocks incorrectly). |
| `ryu-gal-queue` | Writes through `&self` without `UnsafeCell` (undefined behavior). Storage is now `Box<[UnsafeCell<u8>]>`. |
| `ryu-audio-dsp` | In-place commands created aliasing `&` / `&mut` slices. In-place commands now run on a single `&mut` slice; the batch processor has no `unsafe`, and the FFI handles `input == output`. |
| `ryu-pixel-converter`, `ryu-texture-bcn` | Unaligned `&[u8]` → `&[u16/u32/u64]` casts. Replaced with little-endian byte reads/writes; tested with deliberately misaligned buffers. |
| `ryu-video-vic` | The parallel blend path wrote through raw pointers without length checks. `blend_surface` now validates buffer sizes (with overflow checks) and the parallel path uses safe `par_chunks_mut`. |
| `ryu-jit-regalloc` | `BumpArena::alloc_slice` returned `&'static mut` (dangling after `reset`); allocation wasn't null-checked. Slices now borrow the arena, `reset` takes `&mut self`, allocation failure calls `handle_alloc_error`. |
| `ryu-surfaceflinger-queue` | On a full ring the producer advanced the consumer's `tail` (data race) and leaked the dropped slot. A full ring is now rejected without touching consumer state; `queue_buffer` / `release_buffer` / `cancel_buffer` reject invalid state transitions. Two-thread stress test (200k frames) checks no slot leaks. |
| `ryu-tegra-mme` | Infinite-loop macros never terminated. Execution now stops after 2²⁴ instructions (`MME_MAX_STEPS`); `last_run_hit_step_limit()` reports it. |

### Incorrect results

| Crate | Fix |
| :--- | :--- |
| `ryu-jit-regalloc` | `run_pass` gave the same register to simultaneously live intervals when fixed intervals were present. Now looks ahead at fixed intervals per register and spills instead. Randomized test (16 seeds, 512 and 2048 intervals, ~3% fixed) asserts zero conflicts; the old code produced 67 on the first case. |
| `ryu-vulkan-descriptors` | Dirty bindings beyond the output capacity were cleared and lost. Unemitted bindings now stay dirty; callers drain by calling again. |
| `ryu-kernel-sync` | `wake_waiters` removed more waiters than it could report. It now removes at most `out_woken.len()`; the rest stay queued. |
| `ryu-ipc-parser` | Serializers wrote send/receive/exchange descriptor counts without the descriptors. They now write them (same layout as the parser and C# `IpcBuffDesc`) and reject counts above 15. TIPC raw data is padded to the advertised word count. |
| `ryu-romfs-indexer` | Hash seed was `12345678`; LibHac (`RomFsEntries.GetRomHashCode`) uses `123456789`. Fixed and cited in the code. Lookups on real RomFS images would have failed. |
| `ryu-audio-mixer` | Rounding didn't match C#. It now reproduces `MixCommand` exactly: `ceil` on full SIMD-width chunks (8 with AVX, 4 with SSE4.1/NEON), round-half-to-even on the tail and in `MixRamp`. Tests compare bit-for-bit with a port of the C# code. |
| `ryu-video-nvdec` | `idct32` used `step2[1] + step2[13]` where libvpx uses `step2[2]`; fixed (caught by a new float-reference test). The zero-row test truncated with `as i16`. `decode_frame` used one DC value for both U and V. |
| `ryu-host1x-syncpoint` | Untracked increments used id 0, colliding with handle 0 after wrap; they now use a reserved sentinel the handle counter skips. |
| `ryu-gal-queue` | `GalSpanPool::try_insert` wrapped an exactly fitting payload to offset 0. Fixed. (The C# `CircularSpanPool` has the same off-by-one; this crate now deliberately differs.) |
| `ryu-shader-cache` | `parse_toc_entries` didn't validate the header (now checks magic `TOCS`/`TOCH` and version); truncated LZ4 literals were reported as `OutputBufferTooSmall` instead of `CorruptedInput`. |
| `ryu-pixel-converter` | `convert_bgra8_to_rgba8` silently ignored trailing bytes; it now returns an error (FFI: `-3`). |

### Performance

| Crate | Fix |
| :--- | :--- |
| `ryu-host1x-syncpoint` | `signal_done` was O(n) per call (O(n²) per frame in reverse order); now O(1) via a handle index. |
| `ryu-romfs-indexer` | `find_file` re-parsed the header on every lookup; table layout is now validated once and cached. |
| `ryu-storage-fs` | zstd context is reused per thread instead of created per call; PFS0 lookup parses the header once instead of twice per entry. |
| `ryu-shader-cache` | CRC32 is slicing-by-8 instead of byte-at-a-time; LZ4 match copies use `copy_within`/`fill` instead of byte loops. Both checked against the old implementations on random input. |
| `ryu-audio-mixer` | CPU feature detection cached once instead of per call. |

### Behavior changes visible to C#

- `ryu_audio_mix` / `ryu_audio_mix_ramp`: output now matches the managed C#
  fallback bit for bit (previously differed in rounding on some samples).
- `ryu_sf_queue_queue_buffer`, `_release_buffer`, `_cancel_buffer`: return `0`
  (failure) for invalid state transitions or a full ring.
- `ryu_vulkan_*` redundancy filters: may leave bindings dirty when the output
  buffer is full; call again to drain.
- `ryu_shader_cache_parse_entries`: returns `-2` (bad magic) / `-3` (bad
  version) instead of parsing anything.
- `ryu_romfs_*`: hashes differ (now correct), so lookups built against the old
  hash won't match.

### Documentation corrections

- `ryu-audio-fx` described Delay and Reverb effects that don't exist; docs now
  say biquad filters only.
- Biquad feedback coefficients (`a1`, `a2`) must be stored negated; now
  documented in `ryu-audio-dsp`.
- `ryu-maxwell-state::scan_changes` accumulates into the dirty mask; now
  documented (call `clear_dirty`).
- Crate descriptions no longer claim SIMD or features that aren't
  implemented.
