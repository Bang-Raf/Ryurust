# Credits

Ryurust is a fork. Most of what it does exists because other people built it
first.

## Upstream project

**Ryujinx / Ryubing** — <https://git.ryujinx.app/projects/Ryubing>

- Ryujinx is an open-source Nintendo Switch emulator written in C#, created by
  **gdkchan** in 2017 and developed by the Ryujinx Team and Contributors.
- After the original project was discontinued in October 2024, it continues as
  **Ryubing**, maintained by the Ryubing community.
- License: MIT. Copyright (c) Ryujinx Team and Contributors.

Every crate in this repository reimplements, in Rust, a subsystem that already
exists in the Ryujinx C# codebase: texture decoders, GPU command parsing, audio
rendering, memory management, kernel primitives, file systems and the JIT
register allocator. The C# source defined the behavior these crates have to
match. The C# wrappers in `bindings/csharp/` are written against Ryujinx's
namespaces and are meant to be dropped into a Ryubing checkout.

The Ryujinx copyright notice is kept in [`LICENSE`](LICENSE) as the MIT
license requires. If you redistribute this code, keep it there.

## Rust rewrite

- **R. Abdullah Hammami** ([@Bang-Raf](https://github.com/Bang-Raf)) — Rust
  crates, C# integration wrappers, benchmark suite.

## Third-party crates

Runtime dependencies (see each crate's `Cargo.toml`):

| Crate | Used by | License |
| :--- | :--- | :--- |
| [`twox-hash`](https://crates.io/crates/twox-hash) | `ryu-common-hash` (the xxHash3-128 implementation itself) | MIT |
| [`rayon`](https://crates.io/crates/rayon) | parallel paths in `ryu-audio-resampler`, `ryu-common-hash`, `ryu-texture-astc`, `ryu-tegra-swizzle`, `ryu-video-vic` | MIT / Apache-2.0 |
| [`bumpalo`](https://crates.io/crates/bumpalo) | arena allocation in `ryu-shader-transpiler` | MIT / Apache-2.0 |
| [`lz4_flex`](https://crates.io/crates/lz4_flex) | LZ4 decompression in `ryu-storage-fs` | MIT |
| [`zstd`](https://crates.io/crates/zstd) (bindings to Meta's zstd) | Zstandard decompression in `ryu-storage-fs` | MIT / BSD-3-Clause (libzstd) |
| [`aes`](https://crates.io/crates/aes), [`ctr`](https://crates.io/crates/ctr), [`cipher`](https://crates.io/crates/cipher) (RustCrypto) | AES-CTR in `ryu-storage-fs` | MIT / Apache-2.0 |

Development dependency: [`criterion`](https://crates.io/crates/criterion)
(MIT / Apache-2.0) for the benchmark suite.

## Algorithms and references

- **xxHash** — Yann Collet.
- **wyhash** — Wang Yi (public domain / Unlicense). `ryu-maxwell-state`'s
  pipeline-state hash uses wyhash's `mum` mixing function and constants.
- **ASTC** and **BCn (BC1–BC7)** texture formats — Khronos / Microsoft
  specifications.
- **Mahony** and **Madgwick** orientation filters — Robert Mahony et al.;
  Sebastian Madgwick.
- **Linear-scan register allocation** — Poletto & Sarkar (1999).
- NVIDIA Tegra X1 hardware documentation and the reverse-engineering work of
  the Switch emulation community (Ryujinx, Yuzu, envytools / nouveau) for
  GPFIFO, Maxwell, MME, Host1x, NVDEC and VIC behavior.

If you think something here is credited wrongly or not at all, open an issue.
