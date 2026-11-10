# Bindings

## C# (`csharp/`)

P/Invoke wrappers for the 13 crates that Ryubing calls. They are copied from
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

### Integration status

In the Ryubing integration branch, 10 of the 13 wrappers are called from
emulator code. `NativeIntervalTree`, `NativeShaderCache` and `NativeIpcParser`
are defined but not called yet. That a wrapper is called does not mean it runs
on every frame; that depends on the call site and on `IsAvailable`.

The other 19 crates have a C ABI (`ffi` module) but no C# wrapper.
