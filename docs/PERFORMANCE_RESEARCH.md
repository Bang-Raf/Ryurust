# Performance research (2026-11-14)

What actually limits speed and loading time in the Ryurust build of Ryubing
1.3.352, measured on the one test machine (Intel Core Ultra 7 256V, 4 P-cores +
4 E-cores, Fedora 44, .NET 10). Each finding says how it was measured and what
is still unverified. Nothing here has been confirmed with in-game frame times
yet; that is the main gap (see "What is not measured").

## Status

Done after this research (unreleased, see KNOWN_ISSUES.md "Since v0.1.2"):
the ASTC decoder is wired, measured end to end through `AstcDecoder` at 1.6-2.1x
faster with identical output; the slower and unused native calls from sections
3 and 4 are removed; the JIT bitmap threshold is 32 words. The audio mixer turned
out slower at real frame sizes (160-240 samples), including the ramp mix at
3.3x slower, so it is removed rather than given a threshold. Still to do:
GameMode pinning test, in-game frame-time measurements, and benchmarks for the
remaining native sites.

## Summary

| # | Finding | Evidence | Action |
| :- | :--- | :--- | :--- |
| 1 | The emulator runs on 4 of 8 cores | Log: "4 logical", PTC "Thread count: 4" | Disable GameMode core pinning and compare |
| 2 | The Rust ASTC decoder is 1.6–2.0x faster than C#, output identical, and not used | Benchmark below | Wire it into `Texture.cs` |
| 3 | Most wired native calls are slower than the C# they replace | Benchmark below | Remove them or raise their size thresholds |
| 4 | Three wired subsystems only mirror state or are never called | Code reading | Remove |
| 5 | Boot time is dominated by JIT re-translation, not by the caches | Session logs | Follows from 1; shrinks as the cache matures |

## 1. GameMode pins the emulator to the P-cores

Upstream's `Ryujinx.sh` starts the emulator through `gamemoderun` when it is
installed. GameMode 1.8 enables core pinning by default and, on Intel CPUs
with P- and E-cores, pins the process to the P-cores
(`/usr/share/gamemode/gamemode.ini`, `[cpu]` section). On this machine that is
CPUs 0–3 of 8.

Seen in every session log: `CPU: Intel(R) Core(TM) Ultra 7 256V ; 4 logical`
and `Ptc MakeAndSaveTranslations: ... Thread count: 4`. With 8 cores visible
the PTC formula (`Ptc.cs:857`) gives 7 threads. The shader-compile thread
count (`ParallelDiskCacheLoader.cs:223`) and parallel ASTC decoding also
follow the visible core count.

The emulator runs several busy threads at once (three or four guest cores, the
GPU thread, the Vulkan backend thread, audio, JIT), so four cores is tight;
the logs also show `GPU processing thread is too slow`. Whether all eight
cores are faster in a game has to be measured: pinning exists because P-cores
are faster per thread.

To test, create `~/.config/gamemode.ini` with:

```ini
[cpu]
pin_cores=no
```

or run `./Ryujinx` directly instead of `./Ryujinx.sh`. The log's CPU line
should then say `8 logical`.

## 2. ASTC decoding: Rust is faster and is not wired

When the GPU has no native ASTC support (most desktop GPUs), Ryubing decodes
ASTC textures on the CPU (`Texture.cs:795`, `AstcDecoder.TryDecodeToRgba8P`).
That work happens while textures stream in, so it is a plausible source of
stutter. Whether this machine's GPU needs it was not confirmed (`vulkaninfo`
is not installed).

2048x2048 texture of valid, non-void-extent blocks, best of 5, all 8 cores:

| Block size | C# sequential | Rust sequential | C# parallel | Rust parallel | Output |
| :--- | ---: | ---: | ---: | ---: | :--- |
| 4x4 | 233.1 ms | 137.2 ms (1.70x) | 48.3 ms | 29.4 ms (1.64x) | identical |
| 6x6 | 167.8 ms | 94.0 ms (1.79x) | 35.1 ms | 17.3 ms (2.03x) | identical |
| 8x8 | 138.0 ms | 78.4 ms (1.76x) | 25.6 ms | 15.0 ms (1.71x) | identical |

`ryu-texture-astc` has no C# wrapper. This is the one place found so far where
a crate does a large amount of work per call and beats the C# code.

## 3. Small native calls cost more than they save

A call from C# into a Rust library through the current wrappers
(`Marshal.GetDelegateForFunctionPointer` delegates) costs about 7–8 ns before
any work is done. Most wired call sites do 1–4 ns of work.

Per call, best of 7 runs. The C# column is the original code where it is a
one-liner (`SequenceEqual`) and an equivalent loop otherwise:

| Call site | Data | C# | Rust via P/Invoke | Rust / C# |
| :--- | :--- | ---: | ---: | ---: |
| JIT `BitMap.Set` | 4 words | 2.4 ns | 8.1 ns | 3.4x slower |
| | 8 words | 4.3 ns | 8.2 ns | 1.9x slower |
| | 32 words | 16.3 ns | 10.0 ns | 1.6x faster |
| `PipelineUid` equality | 72 B | 1.6 ns | 9.9 ns | 6.2x slower |
| `PipelineUid` hash | 72 B | 3.6 ns | 20.6 ns | 5.7x slower |
| Buffer equality (`PhysicalMemory`) | 32 B | 1.2 ns | 7.7 ns | 6.4x slower |
| | 256 B | 3.0 ns | 9.9 ns | 3.3x slower |
| | 4 KiB | 42.7 ns | 56.6 ns | 1.3x slower |
| | 64 KiB | 1261 ns | 1275 ns | same |
| Texture sampler match | 6 fields | 1.2 ns | 6.9 ns | 5.7x slower |
| Audio mix | 240 samples | 18.5 ns | 23.9 ns | 1.3x slower |
| | 480 samples | 40.2 ns | 31.8 ns | 1.3x faster |

.NET already vectorises `SequenceEqual`, so the Rust AVX2 compare has nothing
to win even on large buffers. The native path only pays off for the JIT bitmap
from about 32 words and for audio from about 480 samples; the current
thresholds are 4 words and none.

These are nanoseconds, but several of these sites run per draw call or per
pipeline lookup. The net effect in a game is unmeasured; the direction is
that the Ryurust build is slower than upstream in these paths.

## 4. Wired subsystems that do no useful work

- **Vulkan descriptors** (`DescriptorSetUpdater.cs`): `UpdateBuffer` is called
  for every uniform and storage buffer binding, but its result is ignored and
  the C# code carries on as before. Pure overhead per binding.
- **Kernel scheduler** (`KPriorityQueue.cs`): every schedule, unschedule and
  reschedule updates both the C# lists and the Rust state, and queries go
  through the Rust state plus a dictionary lookup. The dictionary
  (`_threadsByUid`) never removes entries, so it keeps every thread object
  alive for the life of the queue.
- **Maxwell state tracker** (`StateUpdateTracker.cs`): the tracker is created
  and its register map is filled, but `ScanChanges`, the only method that uses
  it, has no caller. Only the `PipelineUid` hash and equality calls from that
  crate run, and those are slower (section 3). The docs list "Maxwell
  dirty-state scanning" as used; it is not.

Two wired sites may be real wins and need measuring before any change:
`ConstantBufferUpdater` (finds the changed byte range of a uniform buffer and
writes and invalidates only that range, where upstream rewrites the whole
buffer) and the SurfaceFlinger free-slot search.

## 5. Where boot time goes

From the session logs (Zelda: Tears of the Kingdom v1.4.3, time from load to
the first display layer):

| Phase | v0.1.1 session | Source line |
| :--- | ---: | :--- |
| Load PTC file (236 MB, 64,508 functions) | 1.2 s | `Ptc Load: Loaded Translation Cache` |
| Load 1,150 cached shaders | 0.8 s | `Gpu LoadShaders` |
| JIT-translate 46,873 functions not yet in the cache, 4 threads | 12.6 s | `Ptc MakeAndSaveTranslations` |
| Total to first layer | 18.4 s | `SurfaceFlinger CreateLayerFromId` |

The caches load fast. Two thirds of the boot is the JIT translating functions
that earlier sessions profiled but that were not yet in the cache (16.5 s for
72,574 functions in the v0.1.0 session). That cost is CPU-bound and runs on
as many threads as the emulator can see, so finding 1 applies directly. It
also shrinks on its own as the same game is played more and the cache fills.

## What is not measured

- **In-game frame rate and frame times.** There is no comparison of this
  build against upstream Ryubing, or against this build with `lib/` removed.
  Deleting `lib/` makes every wrapper fall back to C#, which is the simplest
  A/B test.
- **Whether the GPU decodes ASTC natively** on the test machine.
- **Shader-compile stutter.** It happens in the Vulkan driver and none of the
  crates touch it.
- **Not yet examined:** texture de-swizzling (`ryu-tegra-swizzle` against
  `LayoutConverter`), BCn decoding, and the file-system read path (AES-CTR,
  integrity checks).
- The C# sides of the hash, sampler-match, bitmap and audio rows in section 3
  are equivalent loops written for the benchmark, not the exact upstream
  functions.

## Proposed order of work

1. Test GameMode pinning off (no code change).
2. Add a wrapper for `ryu-texture-astc` and use it in `Texture.cs`, with the
   C# decoder as fallback.
3. Remove the native calls from `PipelineUid`, `TextureCompatibility`,
   `PhysicalMemory`, `DescriptorSetUpdater`, `KPriorityQueue` and
   `StateUpdateTracker`; raise the JIT bitmap threshold to 32 words and the
   audio threshold to 480 samples.
4. Measure frame times for upstream, this build, and this build after 2 and 3.
5. Benchmark de-swizzling and BCn decoding against C# the same way as ASTC.
