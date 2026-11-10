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

This is the shape crates should have. Where existing crates don't match yet,
it's noted below; fixing that is welcome work.

1. **Safe API first.** Public Rust functions take slices and return
   `Result`/values. The safe API is what tests and benchmarks call.
2. **Thin FFI layer.** An `ffi` module contains `#[no_mangle] pub unsafe
   extern "C"` functions that check pointers and lengths, then call the safe
   API. Handles are `Box::into_raw` pointers with a matching `*_destroy`
   function. *Today:* all 32 crates have an `ffi` module, but several (mostly
   the audio crates, `ryu-buffer-diff`, `ryu-gal-queue`,
   `ryu-host1x-syncpoint`) don't null-check pointers.
3. **Version export.** The C ABI exposes a `*_version() -> u32` function
   (`0xMMMMmmmm`). C# wrappers refuse to load a library older than the one
   they were written for. Bump it when you change an exported signature.
   *Today:* 16 crates export one (the 13 with C# wrappers, plus
   `ryu-pixel-converter`, `ryu-texture-bcn` and `ryu-video-nvdec`);
   names aren't uniform yet (`ryu_ipc_version`, `ryu_scheduler_version`,
   `ryu_nvdec_get_version`). New crates should use
   `ryu_<crate_name>_version`.
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

### Lint debt

CI runs `cargo clippy --workspace --all-targets -- -D warnings`. The existing
crates trip about 26 clippy lints (mostly `excessive_precision` in filter
tables, `missing_safety_doc` and `not_unsafe_ptr_arg_deref` in FFI layers).
Those lints are allowed in `[workspace.lints.clippy]` in the root
`Cargo.toml` so that every *other* lint is enforced. To pay some down: delete
one `allow` line, fix what `cargo clippy` reports, and open a `refactor/` PR.
If the fix touches a hot path, include benchmark results.

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
