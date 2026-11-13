using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.HLE.FileSystem
{
    public static unsafe class NativeRomFsIndexer
    {
        private const string LibraryName = "ryu_romfs_indexer";

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn(byte* data, nuint len);
        private delegate void DestroyFn(IntPtr handle);
        private delegate int FindFileFn(IntPtr handle, byte* path, ulong* outOffset, ulong* outSize);
        private delegate uint HashFn(uint parent, byte* name, nuint nameLen);
        private delegate nuint GetFileCountFn(IntPtr handle);
        private delegate nuint GetDirCountFn(IntPtr handle);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly FindFileFn _findFile;
        private static readonly HashFn _hash;
        private static readonly GetFileCountFn _getFileCount;
        private static readonly GetDirCountFn _getDirCount;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeRomFsIndexer()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_romfs_indexer_version");
                    _create = GetExport<CreateFn>(handle, "ryu_romfs_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_romfs_destroy");
                    _findFile = GetExport<FindFileFn>(handle, "ryu_romfs_find_file");
                    _hash = GetExport<HashFn>(handle, "ryu_romfs_hash");
                    _getFileCount = GetExport<GetFileCountFn>(handle, "ryu_romfs_get_file_count");
                    _getDirCount = GetExport<GetDirCountFn>(handle, "ryu_romfs_get_dir_count");

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

        public static IntPtr Create(ReadOnlySpan<byte> metadata)
        {
            if (_create == null || metadata.Length < 0x50)
            {
                return IntPtr.Zero;
            }

            fixed (byte* p = metadata)
            {
                return _create(p, (nuint)metadata.Length);
            }
        }

        public static void Destroy(IntPtr handle) => _destroy?.Invoke(handle);

        public static bool FindFile(IntPtr handle, string path, out ulong offset, out ulong size)
        {
            offset = 0;
            size = 0;
            if (_findFile == null || handle == IntPtr.Zero || string.IsNullOrEmpty(path))
            {
                return false;
            }

            byte[] utf8 = System.Text.Encoding.UTF8.GetBytes(path + '\0');
            fixed (byte* pPath = utf8)
            fixed (ulong* pOff = &offset)
            fixed (ulong* pSz = &size)
            {
                return _findFile(handle, pPath, pOff, pSz) == 1;
            }
        }

        public static uint Hash(uint parent, ReadOnlySpan<byte> name)
        {
            if (_hash != null && !name.IsEmpty)
            {
                fixed (byte* p = name)
                {
                    return _hash(parent, p, (nuint)name.Length);
                }
            }

            // Fallback Switch RomFS polynomial hash
            uint h = parent ^ 12345678u;
            for (int i = 0; i < name.Length; i++)
            {
                h = ((h >> 5) | (h << 27)) ^ name[i];
            }
            return h;
        }

        public static int GetFileCount(IntPtr handle)
        {
            return _getFileCount != null && handle != IntPtr.Zero ? (int)_getFileCount(handle) : 0;
        }

        public static int GetDirCount(IntPtr handle)
        {
            return _getDirCount != null && handle != IntPtr.Zero ? (int)_getDirCount(handle) : 0;
        }
    }
}
