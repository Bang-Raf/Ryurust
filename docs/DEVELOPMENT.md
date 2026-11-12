# Development Guide

## Repository layout

```text
Ryurust/
├── Cargo.toml              workspace manifest and release/bench profiles
├── crates/ryu-*/           32 library crates, one per emulator subsystem
│   ├── src/lib.rs          safe Rust API + unit tests
│   └── src/ffi.rs or mod ffi   extern "C" exports used by C#
├── benches/                ryurust-bench: Criterion suite, one file per crate
│   └── benches/<crate>.rs
├── bindings/csharp/        P/Invoke wrappers dropped into a Ryubing checkout
├── docs/                   this guide, architecture, benchmark results
└── .github/                CI, PR template, issue templates
```

## Toolchain

- Rust stable. Tested with 1.99.1 on x86_64 Linux.
- `rustfmt` and `clippy` components.
- Optional: .NET SDK and a Ryubing checkout, to test the C# side.

```bash
cargo build --workspace            # debug
cargo build --workspace --release  # builds lib<crate>.so for every crate
cargo test --workspace
```

Release `.so` files land in `target/release/`. Copy the ones you need next to
the Ryubing executable (or into its `lib/` directory) to enable them.

## Crate conventions

Every crate follows these; new crates must too.

1. **Safe API first.** Public Rust functions take slices and return
   `Result`/values. The safe API is what tests and benchmarks call.
2. **Thin FFI layer.** An `ffi` module contains `#[no_mangle] pub unsafe
   extern "C"` functions that null-check pointers and check lengths, then
   call the safe API. Each has a `# Safety` doc saying what the caller must
   guarantee, and each `unsafe` block has a `// SAFETY:` comment. Handles are
   `Box::into_raw` pointers with a matching `*_destroy` function. Every
   crate's `ffi` tests call each pointer-taking export with null pointers.
3. **Version export.** Every crate exports `ryu_<crate_name>_version() -> u32`
   (`0xMMMMmmmm`, e.g. `ryu_audio_fx_version`). C# wrappers refuse to load a
   library older than the one they were written for. Bump it when you change
   an exported signature. Four older names are kept as aliases because
   existing C# code calls them: `ryu_ipc_version`, `ryu_scheduler_version`,
   `ryu_surfaceflinger_version` and `ryu_nvdec_get_version`.
4. **`crate-type = ["cdylib", "rlib"]`** so the crate is both a Rust library
   and a loadable shared object.

### SIMD

Five crates use explicit x86 intrinsics today: `ryu-audio-mixer`,
`ryu-buffer-diff`, `ryu-interval-tree`, `ryu-jit-regalloc` and
`ryu-maxwell-state`. All five pick the AVX2 path at runtime with
`is_x86_feature_detected!` and keep a scalar fallback. The other crates rely
on the compiler's auto-vectorization; the word "SIMD" in some of their
descriptions refers to that, not to hand-written intrinsics.

New SIMD code must:

- live in a `#[target_feature(enable = "...")] unsafe fn`;
- be selected at runtime, never by `-C target-cpu=native` alone;
- have a scalar implementation and a test that checks both give identical
  output.

### `unsafe`

The codebase has about 160 `unsafe` blocks/functions, mostly in FFI layers
and SIMD paths, and none have `// SAFETY:` comments yet. New or changed
`unsafe` code must have one. Adding them to existing code is a welcome
`refactor/` PR.

### Lints

CI runs `cargo clippy --workspace --all-targets -- -D warnings` with no
workspace-wide allows. Where a lint is wrong for a specific piece of code
(coefficient tables copied from C#, functions whose argument list mirrors a C#
or C ABI signature, a NaN-preserving clamp), it is allowed locally with
`#[allow(clippy::...)] // <reason>` on that item or file. Add new exceptions the
same way, never in `Cargo.toml`.

## Testing

- Unit tests live next to the code (`#[cfg(test)] mod tests`).
- Prefer known-answer tests: feed the same input to Ryujinx's C# code, record
  the output, and assert the Rust crate matches byte for byte.
- Cover edge cases: empty input, non-multiple-of-block sizes, maximum
  dimensions, unmapped addresses.

```bash
cargo test -p ryu-texture-bcn          # one crate
cargo test --workspace -- --nocapture  # everything, with output
```

## Benchmarking

The suite is in `benches/` and uses [Criterion](https://bheisler.github.io/criterion.rs/book/).

```bash
cargo bench -p ryurust-bench                       # everything (~several minutes)
cargo bench -p ryurust-bench --bench texture_bcn   # one crate
cargo bench -p ryurust-bench -- --test             # smoke test: run each once
```

HTML reports go to `target/criterion/report/index.html`.

Rules for benchmark code:

- Build inputs outside the timed closure.
- `black_box` both inputs and outputs.
- Set `Throughput` so results are comparable across input sizes.
- No hard-coded comparison numbers. To compare two implementations, benchmark
  both in the same file.

For trustworthy numbers: plug in the laptop, close other apps, and run the
same benchmark twice; if the two runs disagree by more than a few percent the
machine is too noisy. Published results and how they were produced are in
[`BENCHMARKS.md`](BENCHMARKS.md).

## Testing inside Ryubing

1. Build: `cargo build --workspace --release`.
2. Copy `target/release/libryu_*.so` into the Ryubing build output directory
   (next to the `Ryujinx` executable, or its `lib/` subdirectory).
3. Copy the matching wrapper from `bindings/csharp/` into the Ryubing source
   tree (paths in [`bindings/README.md`](../bindings/README.md)) and wire the
   call site with an `IsAvailable` check and the original C# code as fallback.
4. Build Ryubing and run a game. Compare rendering/audio output and frame time
   with and without the `.so` present.
