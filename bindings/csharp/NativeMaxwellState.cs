using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Common
{
    public static unsafe class NativeMaxwellState
    {
        private const string LibraryName = "ryu_maxwell_state";

        private delegate uint VersionFn();
        private delegate IntPtr CreateTrackerFn();
        private delegate void DestroyTrackerFn(IntPtr tracker);
        private delegate void MapRegisterFn(IntPtr tracker, uint regOffset, byte groupIndex);
        private delegate void MapRangeFn(IntPtr tracker, uint startOffset, uint sizeBytes, byte groupIndex);
        private delegate void SetDirtyFn(IntPtr tracker, uint regOffset);
        private delegate void SetDirtyBatchFn(IntPtr tracker, uint* offsets, nuint count);
        private delegate void ForceDirtyFn(IntPtr tracker, byte groupIndex);
        private delegate void SetAllDirtyFn(IntPtr tracker);
        private delegate ulong GetDirtyMaskFn(IntPtr tracker);
        private delegate ulong ScanChangesFn(IntPtr tracker, byte* current, byte* shadow, nuint len);
        private delegate nuint CollectDirtyFn(IntPtr tracker, ulong checkMask, uint* outGroups, nuint maxGroups);
        private delegate void ClearDirtyFn(IntPtr tracker, ulong mask);
        private delegate ulong PipelineHash64Fn(byte* data, nuint len);
        private delegate int PipelineHash32Fn(byte* data, nuint len);
        private delegate int PipelineEqualsFn(byte* a, byte* b, nuint len);

        private static readonly VersionFn _version;
        private static readonly CreateTrackerFn _create;
        private static readonly DestroyTrackerFn _destroy;
        private static readonly MapRegisterFn _mapRegister;
        private static readonly MapRangeFn _mapRange;
        private static readonly SetDirtyFn _setDirty;
        private static readonly SetDirtyBatchFn _setDirtyBatch;
        private static readonly ForceDirtyFn _forceDirty;
        private static readonly SetAllDirtyFn _setAllDirty;
        private static readonly GetDirtyMaskFn _getDirtyMask;
        private static readonly ScanChangesFn _scanChanges;
        private static readonly CollectDirtyFn _collectDirty;
        private static readonly ClearDirtyFn _clearDirty;
        private static readonly PipelineHash64Fn _pipelineHash64;
        private static readonly PipelineHash32Fn _pipelineHash32;
        private static readonly PipelineEqualsFn _pipelineEquals;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeMaxwellState()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_maxwell_state_version");
                    _create = GetExport<CreateTrackerFn>(handle, "ryu_maxwell_tracker_create");
                    _destroy = GetExport<DestroyTrackerFn>(handle, "ryu_maxwell_tracker_destroy");
                    _mapRegister = GetExport<MapRegisterFn>(handle, "ryu_maxwell_tracker_map_register");
                    _mapRange = GetExport<MapRangeFn>(handle, "ryu_maxwell_tracker_map_range");
                    _setDirty = GetExport<SetDirtyFn>(handle, "ryu_maxwell_tracker_set_dirty");
                    _setDirtyBatch = GetExport<SetDirtyBatchFn>(handle, "ryu_maxwell_tracker_set_dirty_batch");
                    _forceDirty = GetExport<ForceDirtyFn>(handle, "ryu_maxwell_tracker_force_dirty");
                    _setAllDirty = GetExport<SetAllDirtyFn>(handle, "ryu_maxwell_tracker_set_all_dirty");
                    _getDirtyMask = GetExport<GetDirtyMaskFn>(handle, "ryu_maxwell_tracker_get_dirty_mask");
                    _scanChanges = GetExport<ScanChangesFn>(handle, "ryu_maxwell_tracker_scan_changes");
                    _collectDirty = GetExport<CollectDirtyFn>(handle, "ryu_maxwell_tracker_collect_dirty");
                    _clearDirty = GetExport<ClearDirtyFn>(handle, "ryu_maxwell_tracker_clear_dirty");
                    _pipelineHash64 = GetExport<PipelineHash64Fn>(handle, "ryu_maxwell_pipeline_hash64");
                    _pipelineHash32 = GetExport<PipelineHash32Fn>(handle, "ryu_maxwell_pipeline_hash32");
                    _pipelineEquals = GetExport<PipelineEqualsFn>(handle, "ryu_maxwell_pipeline_equals");

                    _isAvailable = _version != null && _version() >= 0x00010000;
                }
            }
            catch (Exception e)
            {
                _isAvailable = false;
                _loadFailure = e.Message;
            }

            LogLoadResult();
        }

        private static IntPtr LoadNativeLibrary()
        {
            string ext = OperatingSystem.IsWindows() ? ".dll" : (OperatingSystem.IsMacOS() ? ".dylib" : ".so");
            string prefix = OperatingSystem.IsWindows() ? string.Empty : "lib";
            string fileName = $"{prefix}{LibraryName}{ext}";

            string baseDir = AppDomain.CurrentDomain.BaseDirectory;
            string[] probePaths = [
                Path.Combine(baseDir, fileName),
                Path.Combine(baseDir, "lib", fileName),
                Path.Combine(baseDir, "runtimes", "linux-x64", "native", fileName),
                Path.Combine(baseDir, "..", "linux_test_suite", "lib", fileName),
                Path.Combine(baseDir, "..", "crates", "target", "release", fileName),
                fileName
            ];

            foreach (string path in probePaths)
            {
                if (File.Exists(path) && NativeLibrary.TryLoad(path, out IntPtr h))
                {
                    _loadedFrom = path;
                    return h;
                }
            }

            if (NativeLibrary.TryLoad(LibraryName, Assembly.GetExecutingAssembly(), null, out IntPtr resolved))
            {
                _loadedFrom = $"{LibraryName} (system library path)";
                return resolved;
            }

            return IntPtr.Zero;
        }

        private static T GetExport<T>(IntPtr handle, string name) where T : Delegate
        {
            if (NativeLibrary.TryGetExport(handle, name, out IntPtr address))
            {
                return Marshal.GetDelegateForFunctionPointer<T>(address);
            }
            _missingExports.Add(name);
            return null;
        }

        private static void LogLoadResult()
        {
            if (_isAvailable)
            {
                uint version = _version();
                string missing = _missingExports.Count == 0 ? string.Empty : $" (missing exports: {string.Join(", ", _missingExports)})";
                Logger.Info?.Print(LogClass.Application, $"Ryurust: using native {LibraryName} {version >> 16}.{version & 0xFFFF} from {_loadedFrom}{missing}");
            }
            else if (_loadedFrom == null && _loadFailure == null)
            {
                Logger.Info?.Print(LogClass.Application, $"Ryurust: {LibraryName} not found, using the C# implementation");
            }
            else
            {
                string reason = _loadFailure
                    ?? (_missingExports.Count != 0 ? $"missing exports: {string.Join(", ", _missingExports)}" : "version too old");
                Logger.Warning?.Print(LogClass.Application, $"Ryurust: {LibraryName} found at {_loadedFrom ?? "an unknown path"} but not usable ({reason}), using the C# implementation");
            }
        }

        public static IntPtr CreateTracker() => _create != null ? _create() : IntPtr.Zero;

        public static void DestroyTracker(IntPtr handle) => _destroy?.Invoke(handle);

        public static void MapRegister(IntPtr handle, uint regOffset, byte groupIndex)
        {
            if (handle != IntPtr.Zero) _mapRegister?.Invoke(handle, regOffset, groupIndex);
        }

        public static void MapRange(IntPtr handle, uint startOffset, uint sizeBytes, byte groupIndex)
        {
            if (handle != IntPtr.Zero) _mapRange?.Invoke(handle, startOffset, sizeBytes, groupIndex);
        }

        public static void SetDirty(IntPtr handle, uint regOffset)
        {
            if (handle != IntPtr.Zero) _setDirty?.Invoke(handle, regOffset);
        }

        public static void ForceDirty(IntPtr handle, byte groupIndex)
        {
            if (handle != IntPtr.Zero) _forceDirty?.Invoke(handle, groupIndex);
        }

        public static void SetAllDirty(IntPtr handle)
        {
            if (handle != IntPtr.Zero) _setAllDirty?.Invoke(handle);
        }

        public static ulong GetDirtyMask(IntPtr handle)
        {
            return handle != IntPtr.Zero && _getDirtyMask != null ? _getDirtyMask(handle) : 0;
        }

        public static ulong ScanChanges(IntPtr handle, ReadOnlySpan<byte> current, ReadOnlySpan<byte> shadow)
        {
            if (handle == IntPtr.Zero || _scanChanges == null) return 0;
            int len = Math.Min(current.Length, shadow.Length);
            if (len == 0) return 0;
            fixed (byte* pCurr = current)
            fixed (byte* pShad = shadow)
            {
                return _scanChanges(handle, pCurr, pShad, (nuint)len);
            }
        }

        public static int CollectDirty(IntPtr handle, ulong checkMask, Span<uint> outGroups)
        {
            if (handle == IntPtr.Zero || _collectDirty == null || outGroups.IsEmpty) return 0;
            fixed (uint* pOut = outGroups)
            {
                return (int)_collectDirty(handle, checkMask, pOut, (nuint)outGroups.Length);
            }
        }

        public static void ClearDirty(IntPtr handle, ulong mask)
        {
            if (handle != IntPtr.Zero) _clearDirty?.Invoke(handle, mask);
        }

        public static ulong PipelineHash64(ReadOnlySpan<byte> data)
        {
            if (_pipelineHash64 == null || data.IsEmpty) return 0;
            fixed (byte* p = data)
            {
                return _pipelineHash64(p, (nuint)data.Length);
            }
        }

        public static int PipelineHash32(ReadOnlySpan<byte> data)
        {
            if (_pipelineHash32 == null || data.IsEmpty) return 0;
            fixed (byte* p = data)
            {
                return _pipelineHash32(p, (nuint)data.Length);
            }
        }

        public static bool PipelineEquals(ReadOnlySpan<byte> a, ReadOnlySpan<byte> b)
        {
            if (a.Length != b.Length) return false;
            if (a.IsEmpty) return true;
            if (_pipelineEquals == null) return a.SequenceEqual(b);
            fixed (byte* pa = a)
            fixed (byte* pb = b)
            {
                return _pipelineEquals(pa, pb, (nuint)a.Length) == 1;
            }
        }
    }
}
