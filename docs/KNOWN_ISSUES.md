# Known Issues

Last reviewed: 2026-11-11.

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
| `ryu-texture-astc` | Partitioned blocks now follow the C# `SelectPartition` port, but there is no golden-output test against Ryujinx's C# decoder yet (needs a .NET environment to generate reference data). `decode_astc_to_rgba8` returns success even when some blocks were invalid (they decode to magenta); the C# decoder reports failure. |
| `ryu-jit-regalloc` | Two fixed intervals that overlap and demand the same register cannot both be satisfied; both keep it. Spilling picks the current interval and ignores weight / next use. Sorting unsorted input allocates. |
| `ryu-ipc-parser` | TIPC serialization doesn't encode pointer-buffer (`X`) descriptors, matching the C# implementation; a TIPC message with pointer buffers silently drops them. |
| `ryu-interval-tree` | `FlatIntervalTree` is a sorted `Vec`: linear-scan queries and O(n) insert/remove. Fine for small caches, not a balanced tree. |
| `ryu-romfs-indexer` | The hash now matches LibHac, but it hasn't been tested against a real RomFS image. |

### Code-quality debt

- Many existing `unsafe` blocks (mostly in FFI layers) still have no
  `// SAFETY:` comment; new and recently fixed code does.
- Several FFI functions don't null-check their pointer arguments.
- Version exports exist in only 16 of 32 crates and aren't uniformly named
  (see [`DEVELOPMENT.md`](DEVELOPMENT.md)).
- 26 clippy lints are allowed workspace-wide as tracked debt (see the root
  `Cargo.toml`).

## Gameplay testing

The v0.1.0 Linux package was installed from
`ghcr.io/bang-raf/ryurust:0.1.0` and played on 2026-11-11 (Fedora 44,
Intel Core Ultra 7 256V with integrated Arc graphics, firmware 22.5.0).

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

## Fixed before release

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
