# Bindings

## C# (`csharp/`)

P/Invoke wrappers for 14 crates. They are copied from
the Ryubing integration branch; each file belongs at the path shown below in a
Ryubing checkout.

| Wrapper | Path in Ryubing `src/` | Crate |
| :--- | :--- | :--- |
| `NativeJitRegAlloc.cs` | `ARMeilleure/CodeGen/RegisterAllocators/` | `ryu-jit-regalloc` |
| `NativeAudioMixer.cs` | `Ryujinx.Audio/Renderer/Dsp/` | `ryu-audio-mixer` |
| `NativeMaxwellState.cs` | `Ryujinx.Common/` | `ryu-maxwell-state` |
| `NativeIntervalTree.cs` | `Ryujinx.Common/Collections/` | `ryu-interval-tree` |
| `NativeTextureMatcher.cs` | `Ryujinx.Graphics.Gpu/Image/` | `ryu-texture-matcher` |
| `NativeBufferDiff.cs` | `Ryujinx.Graphics.Gpu/Memory/` | `ryu-buffer-diff` |
| `NativeShaderCache.cs` | `Ryujinx.Graphics.Gpu/Shader/DiskCache/` | `ryu-shader-cache` |
| `NativeTextureAstc.cs` | `Ryujinx.Graphics.Texture/Astc/` | `ryu-texture-astc` |
| `NativeVulkanDescriptors.cs` | `Ryujinx.Graphics.Vulkan/` | `ryu-vulkan-descriptors` |
| `NativeRomFsIndexer.cs` | `Ryujinx.HLE/FileSystem/` | `ryu-romfs-indexer` |
| `NativeIpcParser.cs` | `Ryujinx.HLE/HOS/Ipc/` | `ryu-ipc-parser` |
| `NativeKernelScheduler.cs` | `Ryujinx.HLE/HOS/Kernel/Threading/` | `ryu-kernel-scheduler` |
| `NativeSurfaceFlingerQueue.cs` | `Ryujinx.HLE/HOS/Services/SurfaceFlinger/` | `ryu-surfaceflinger-queue` |
| `NativeMotionFusion.cs` | `Ryujinx.Input/Motion/` | `ryu-motion-fusion` |

### How loading works

Each wrapper resolves `lib<crate_name>.so` (`.dll` / `.dylib` on other OSes)
at static-constructor time, probing the app directory, `lib/`,
`runtimes/linux-x64/native/`, then the system loader. It looks up every export
with `NativeLibrary.TryGetExport`, checks the crate's `*_version()` export, and
sets `IsAvailable`. Callers in Ryubing check `IsAvailable` and fall back to
the original C# code when the library is missing or too old, so the emulator
keeps working without the native libraries.

The static constructor then logs one line through Ryujinx's `Logger`
(`LogClass.Application`): `Ryurust: using native <library> <version> from
<path>` (Info), `<library> not found, using the C# implementation` (Info), or
`<library> found at <path> but not usable (<reason>)` (Warning). A wrapper
the emulator never touches logs nothing.

### Integration status

In the Ryubing integration branch, 6 of the 14 wrappers are called from
emulator code:

- `NativeTextureAstc`: ASTC decoding in `AstcDecoder.TryDecodeToRgba8P`, used
  when the GPU can't sample ASTC. Byte-identical to the C# decoder, about 2x
  faster; falls back to C# on short input.
- `NativeJitRegAlloc`: `BitMap.Set` / `Clear` for bitmaps of 32 words or more.
- `NativeBufferDiff`: `ConstantBufferUpdater`, which writes and invalidates
  only the changed range of a uniform buffer.
- `NativeSurfaceFlingerQueue`: the free-slot search when a buffer is dequeued.
- `NativeMotionFusion`: motion-control sensor fusion.
- `NativeRomFsIndexer`: only when RomFS mods are applied. `ModLoader` locates
  the base game's files with it, through
  `Ryujinx.HLE/FileSystem/NativeRomFsImage.cs` (in the integration patch only,
  because it depends on LibHac).

`NativeIntervalTree`, `NativeShaderCache` and `NativeIpcParser` were never
called. Since v0.1.2, `NativeMaxwellState`, `NativeTextureMatcher`,
`NativeVulkanDescriptors`, `NativeKernelScheduler` and `NativeAudioMixer` are
no longer called either: measured, each native call cost more than the C# it
replaced, or its result was unused (see
[`docs/PERFORMANCE_RESEARCH.md`](../docs/PERFORMANCE_RESEARCH.md)). That a wrapper is called does not
mean it runs on every frame; that depends on the call site and on
`IsAvailable`.

The other 18 crates have a C ABI (`ffi` module) but no C# wrapper.

### Behavior changes

The pre-release bug fixes kept every exported signature, but some return
values changed (for example, the SurfaceFlinger queue now rejects invalid state
transitions, and the audio mixer's rounding now matches C# exactly). The list
is in [`docs/KNOWN_ISSUES.md`](../docs/KNOWN_ISSUES.md#behavior-changes-visible-to-c).
