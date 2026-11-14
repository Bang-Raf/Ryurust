# Ryubing + Ryurust

This is a build of the [Ryubing](https://git.ryujinx.app/projects/Ryubing)
Nintendo Switch emulator (Canary 1.3.352) with the
[Ryurust](https://github.com/Bang-Raf/Ryurust) native libraries enabled.

It is an **unofficial, experimental** build. Report problems to the Ryurust
issue tracker, not to the Ryubing project.

## What's different from upstream Ryubing

- `lib/` contains the 32 Ryurust libraries (`libryu_*.so` on Linux,
  `ryu_*.dll` on Windows).
- 6 subsystems call the Rust libraries: ASTC texture decoding (when the GPU
  can't sample ASTC; about 2x faster than C#), JIT register-allocator bitsets
  for large functions, uniform-buffer diffing, the SurfaceFlinger queue,
  motion fusion, and RomFS lookups when RomFS mods are applied.
- The log shows which libraries were used: each wrapper writes one
  `Ryurust:` line the first time the emulator calls it.
- Every wrapper falls back to the original C# code if its library is missing
  or fails to load. Delete a library from `lib/` to turn that subsystem off.
- The auto-updater is compiled out, so this build never replaces itself with
  upstream Ryubing.

Known limitations are listed in Ryurust's `docs/KNOWN_ISSUES.md`.

## Run

Linux x64:

```bash
tar -xJf ryurust-*-linux_x64.tar.xz
cd ryurust-*-linux_x64
./Ryujinx.sh
```

Windows x64: extract `ryurust-*-win_x64.zip` and run `Ryujinx.exe`. The
build is self-contained; it needs no .NET or Visual C++ runtime install.

You need your own legally dumped firmware, `prod.keys` and games. None are
included.

## Licenses

- Ryubing / Ryujinx: MIT, see `LICENSE-Ryujinx.txt`.
- Ryurust: MIT, see `LICENSE-Ryurust.txt`.
- Credits and third-party components: `CREDITS.md`.

Not affiliated with or endorsed by Nintendo.
