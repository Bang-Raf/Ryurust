# Ryurust

Rust implementations of performance-critical subsystems from the
[Ryujinx / Ryubing](https://git.ryujinx.app/projects/Ryubing) Nintendo Switch
emulator.

Ryubing is written in C#. Ryurust takes individual hot paths (texture
decoding, GPU command parsing, audio mixing, memory tracking, kernel
scheduling, and so on) and rewrites each one as a small Rust crate with a C
ABI. The emulator loads them through P/Invoke and falls back to its own C# code
when a library isn't present.

> **Status: experimental.** This is not a standalone emulator and not a
> drop-in replacement for Ryubing. Several crates are incomplete or have known
> bugs; read [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) before relying on
> any of them.

## At a glance

| | |
| :--- | :--- |
| Crates | 32 |
| Unit tests | 262, all passing (`cargo test --workspace`) |
| Benchmarks | 121 Criterion benchmarks, one suite per crate ([results](docs/BENCHMARKS.md)) |
| Wired into Ryubing | 13 crates have C# wrappers; 10 of those are called from emulator code |
| Platforms | Built and tested on x86_64 Linux. AVX2 paths are chosen at runtime with scalar fallbacks. |
| License | MIT |

## Crates

**C#** column: ● called from Ryubing, ○ wrapper exists but isn't called yet,
blank = no wrapper.

| Area | Crate | What it does | C# |
| :--- | :--- | :--- | :---: |
| Textures | `ryu-texture-astc` | ASTC block decompression to RGBA8 | |
| | `ryu-texture-bcn` | BC1–BC7 (incl. BC6H) surface decoding | |
| | `ryu-tegra-swizzle` | Tegra block-linear ↔ linear layout conversion | |
| | `ryu-pixel-converter` | Pixel format conversion (R5G6B5, R4G4B4A4, BGRA8 → RGBA8, …) | |
| | `ryu-texture-matcher` | Packed texture-format / sampler compatibility checks for the texture cache | ● |
| GPU | `ryu-gpu-gpfifo` | GPFIFO push-buffer method parser | |
| | `ryu-tegra-mme` | Maxwell Macro Execution Engine interpreter | |
| | `ryu-maxwell-state` | Dirty-register scanning (AVX2) and pipeline-state hashing | ● |
| | `ryu-gal-queue` | SPSC command queue and span pool for the render thread | |
| | `ryu-host1x-syncpoint` | Host1x syncpoint / fence sequencing | |
| | `ryu-buffer-diff` | Dirty-range detection for uniform buffer updates (AVX2) | ● |
| | `ryu-interval-tree` | Range-overlap queries for texture/buffer caches (sorted array) | ○ |
| | `ryu-vulkan-descriptors` | Descriptor-set state tracking and redundancy filtering | ● |
| Shaders | `ryu-shader-transpiler` | Maxwell shader → SPIR-V (**prototype, output not valid yet**) | |
| | `ryu-shader-cache` | Shader disk-cache TOC parsing, LZ4, CRC32 | ○ |
| Video | `ryu-video-nvdec` | VP9 IDCT and loop-filter kernels (**not a full VP9 decoder**) | |
| | `ryu-video-vic` | VIC surface blending, colour-space conversion, deinterlacing | |
| Audio | `ryu-audio-dsp` | Audio renderer command processing (mix, volume, biquad) | |
| | `ryu-audio-fx` | Biquad filter effects | |
| | `ryu-audio-mixer` | Multi-voice mixing and volume ramps (AVX2) | ● |
| | `ryu-audio-resampler` | Polyphase resampling and ADPCM decoding | |
| Memory | `ryu-memory-pagetable` | 48-bit guest virtual address page table | |
| | `ryu-memory-tracking` | Atomic page bitmap for dirty-memory tracking | |
| Kernel / OS | `ryu-kernel-sync` | Address-arbiter wait tables | |
| | `ryu-kernel-scheduler` | Priority-bitmask thread scheduler | ● |
| | `ryu-ipc-parser` | Horizon OS CMIF/TIPC IPC message parse and serialize | ○ |
| | `ryu-surfaceflinger-queue` | SurfaceFlinger buffer queue and binder parcels | ● |
| Storage | `ryu-storage-fs` | NCA AES-CTR decryption, LZ4/zstd, PFS0 lookup | |
| | `ryu-romfs-indexer` | RomFS path lookup | ● |
| JIT | `ryu-jit-regalloc` | Linear-scan register allocator and AVX2 bitset helpers for ARMeilleure | ● |
| Input | `ryu-motion-fusion` | Mahony IMU sensor fusion for motion controls | ● |
| Common | `ryu-common-hash` | xxHash3-128 (via `twox-hash`) with batching and a C ABI | |

## Benchmarks

Measured on an Intel Core Ultra 7 256V, Fedora 44, rustc 1.99.1, Criterion
0.5.1, on 2026-11-10. These are a sample; all 121 results, the method and the
caveats are in [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).

| Benchmark | Input | Time | Throughput |
| :--- | :--- | ---: | ---: |
| `texture_bcn/bc1` | 1920×1080 | 1.69 ms | 4.58 GiB/s |
| `texture_bcn/bc7` | 1920×1080 | 13.3 ms | 593 MiB/s |
| `tegra_swizzle/deswizzle_seq` | RGBA8 1920×1080 | 521 µs | 14.8 GiB/s |
| `memory_pagetable/read/random_256MiB` | 4096 random lookups | 4.24 µs | 967 M lookups/s |
| `gpu_gpfifo/parse_pushbuffer_chunk` | ~50k methods | 44.5 µs | 1.12 G methods/s |
| `audio_mixer/mix_batch` | 128 voices × 240 samples | 2.47 µs | 12.5 G samples/s |
| `maxwell_state/scan_changes` (AVX2) | 14 KiB register file | 132 ns | 101 GiB/s |
| `maxwell_state/scan_changes_scalar` | 14 KiB register file | 1.14 µs | 11.7 GiB/s |
| `shader_cache/crc32` | 64 KiB | 29.5 µs | 2.07 GiB/s |
| `storage_fs/aes_ctr_decrypt` | 1 MiB | 422 µs | 2.32 GiB/s |

These numbers measure the Rust code by itself. **There is no measured
comparison with the C# implementation yet**, so they say nothing about
speedup. Contributions that add a same-input C# vs Rust comparison are very
welcome.

## Quick start

Requirements: Rust stable (tested with 1.99.1), x86_64 Linux.

```bash
git clone git@github.com:Bang-Raf/Ryurust.git
cd Ryurust
cargo build --workspace --release      # target/release/libryu_*.so
cargo test --workspace
cargo bench -p ryurust-bench           # full benchmark suite
```

To use a library from Ryubing, copy `target/release/libryu_<name>.so` next to
the Ryubing executable and add the matching wrapper from
[`bindings/csharp/`](bindings/csharp). See
[`bindings/README.md`](bindings/README.md).

## Documentation

| Document | Contents |
| :--- | :--- |
| [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) | Layout, crate conventions, SIMD and `unsafe` rules, testing, benchmarking, testing inside Ryubing |
| [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) | All benchmark results, test machine, method, how to reproduce |
| [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) | Crashes, incorrect results and unfinished crates |
| [`bindings/README.md`](bindings/README.md) | C# wrappers and how they load |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to contribute and the pull-request process |
| [`CREDITS.md`](CREDITS.md) | Upstream project, authors, dependencies |

## Contributing

Contributions are welcome. In short:

1. Open or pick an issue.
2. Fork, then branch from `main` as `feat/…`, `fix/…`, `perf/…`, `refactor/…`,
   `docs/…`, `test/…` or `ci/…`.
3. Commit using [Conventional Commits](https://www.conventionalcommits.org/),
   scoped by crate: `fix(texture-astc): bound color value count`.
4. Before pushing, run `cargo fmt`, `cargo clippy -- -D warnings`,
   `cargo test` and `cargo bench -- --test`.
5. Open a PR against `main` and fill in the template. Performance PRs must
   include before/after Criterion output.
6. One maintainer approval and green CI are required; PRs are squash-merged.

The full process is in [`CONTRIBUTING.md`](CONTRIBUTING.md).
[`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) is a good place to find a first
task.

## Credits

Ryurust is a fork of work from the open-source Ryujinx project.

- **[Ryujinx](https://git.ryujinx.app/projects/Ryubing)**, created by
  **gdkchan** and developed by the Ryujinx Team and Contributors, and its
  continuation **Ryubing**, maintained by the Ryubing community. Every crate
  here reimplements a subsystem from the Ryujinx C# source, which defines the
  behavior these crates must match.
- **Rust rewrite**: R. Abdullah Hammami ([@Bang-Raf](https://github.com/Bang-Raf)).
- Third-party crates: `twox-hash`, `rayon`, `bumpalo`, `lz4_flex`, `zstd`,
  RustCrypto `aes`/`ctr`/`cipher`, and `criterion` for benchmarks.

Full credits are in [`CREDITS.md`](CREDITS.md).

## License

[MIT](LICENSE). Copyright (c) 2026 Raden Abdullah Hammami, and copyright (c)
Ryujinx Team and Contributors for the upstream work this is derived from.

Ryurust is not affiliated with or endorsed by Nintendo. It contains no
Nintendo code, keys or firmware; you must supply your own legally dumped
games and keys to use Ryubing.
