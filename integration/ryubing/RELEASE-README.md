# Ryubing + Ryurust (Linux x64)

This is a build of the [Ryubing](https://git.ryujinx.app/projects/Ryubing)
Nintendo Switch emulator (Canary 1.3.352) with the
[Ryurust](https://github.com/Bang-Raf/Ryurust) native libraries enabled.

It is an **unofficial, experimental** build. Report problems to the Ryurust
issue tracker, not to the Ryubing project.

## What's different from upstream Ryubing

- `lib/libryu_*.so` contains the 32 Ryurust libraries.
- 13 C# subsystems have native wrappers; 10 are called by the emulator: JIT
  register-allocator bitsets, audio mixing, Maxwell dirty-state scanning,
  uniform-buffer diffing, texture compatibility checks, Vulkan descriptor
  updates, RomFS lookup, the kernel scheduler, the SurfaceFlinger queue and
  motion fusion.
- Every wrapper falls back to the original C# code if its library is missing
  or fails to load. Delete a `.so` from `lib/` to turn that subsystem off.
- The auto-updater is compiled out, so this build never replaces itself with
  upstream Ryubing.

Known limitations are listed in Ryurust's `docs/KNOWN_ISSUES.md`.

## Run

```bash
tar -xJf ryurust-*-linux_x64.tar.xz
cd ryurust-*-linux_x64
./Ryujinx.sh
```

You need your own legally dumped firmware, `prod.keys` and games. None are
included.

## Licenses

- Ryubing / Ryujinx: MIT, see `LICENSE-Ryujinx.txt`.
- Ryurust: MIT, see `LICENSE-Ryurust.txt`.
- Credits and third-party components: `CREDITS.md`.

Not affiliated with or endorsed by Nintendo.
