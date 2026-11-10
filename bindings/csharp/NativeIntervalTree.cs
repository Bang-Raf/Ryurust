using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Common.Collections
{
    public sealed unsafe class NativeIntervalTree : IDisposable
    {
        private const string LibraryName = "ryu_interval_tree";

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn();
        private delegate void DestroyFn(IntPtr tree);
        private delegate void ClearFn(IntPtr tree);
        private delegate void InsertFn(IntPtr tree, ulong start, ulong end, ulong value);
        private delegate int RemoveFn(IntPtr tree, ulong start, ulong end, ulong value);
        private delegate nuint FindOverlapsFn(IntPtr tree, ulong start, ulong end, ulong* outBuf, nuint maxOut);
        private delegate nuint LenFn(IntPtr tree);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly ClearFn _clear;
        private static readonly InsertFn _insert;
        private static readonly RemoveFn _remove;
        private static readonly FindOverlapsFn _findOverlaps;
        private static readonly LenFn _len;

        private static readonly bool _isAvailable;

        public static bool IsAvailable => _isAvailable;

        private IntPtr _handle;

        public int Count => _handle != IntPtr.Zero && _isAvailable ? (int)_len(_handle) : 0;

        static NativeIntervalTree()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_interval_tree_version");
                    _create = GetExport<CreateFn>(handle, "ryu_interval_tree_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_interval_tree_destroy");
                    _clear = GetExport<ClearFn>(handle, "ryu_interval_tree_clear");
                    _insert = GetExport<InsertFn>(handle, "ryu_interval_tree_insert");
                    _remove = GetExport<RemoveFn>(handle, "ryu_interval_tree_remove");
                    _findOverlaps = GetExport<FindOverlapsFn>(handle, "ryu_interval_tree_find_overlaps");
                    _len = GetExport<LenFn>(handle, "ryu_interval_tree_len");

                    _isAvailable = _version != null && _create != null && _findOverlaps != null && _version() >= 0x00010000;
                }
            }
            catch
            {
                _isAvailable = false;
            }
        }

        public NativeIntervalTree()
        {
            if (_isAvailable)
            {
                _handle = _create();
            }
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
                    return h;
                }
            }

            if (NativeLibrary.TryLoad(LibraryName, Assembly.GetExecutingAssembly(), null, out IntPtr defaultHandle))
            {
                return defaultHandle;
            }

            return IntPtr.Zero;
        }

        private static T GetExport<T>(IntPtr handle, string name) where T : Delegate
        {
            if (NativeLibrary.TryGetExport(handle, name, out IntPtr address))
            {
                return Marshal.GetDelegateForFunctionPointer<T>(address);
            }
            return null;
        }

        public void Insert(ulong start, ulong end, ulong value)
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                _insert(_handle, start, end, value);
            }
        }

        public bool Remove(ulong start, ulong end, ulong value)
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                return _remove(_handle, start, end, value) == 1;
            }
            return false;
        }

        public void Clear()
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                _clear(_handle);
            }
        }

        public int FindOverlaps(ulong start, ulong end, Span<ulong> destination)
        {
            if (_handle != IntPtr.Zero && _isAvailable && !destination.IsEmpty)
            {
                fixed (ulong* pDst = destination)
                {
                    return (int)_findOverlaps(_handle, start, end, pDst, (nuint)destination.Length);
                }
            }
            return 0;
        }

        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                _destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }
    }
}
