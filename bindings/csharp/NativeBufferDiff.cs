using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Graphics.Gpu.Memory
{
    public static unsafe class NativeBufferDiff
    {
        private const string LibraryName = "ryu_buffer_diff";

        private delegate uint VersionFn();
        private delegate int FindDirtyRangeFn(byte* src, byte* dst, nuint len, nuint* outOffset, nuint* outLength);
        private delegate int DiffAndCopyFn(byte* src, byte* dst, nuint len, nuint* outOffset, nuint* outLength);
        private delegate int IsEqualFn(byte* a, byte* b, nuint len);

        private static readonly VersionFn _version;
        private static readonly FindDirtyRangeFn _findDirtyRange;
        private static readonly DiffAndCopyFn _diffAndCopy;
        private static readonly IsEqualFn _isEqual;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeBufferDiff()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_buffer_diff_version");
                    _findDirtyRange = GetExport<FindDirtyRangeFn>(handle, "ryu_buffer_find_dirty_range");
                    _diffAndCopy = GetExport<DiffAndCopyFn>(handle, "ryu_buffer_diff_and_copy");
                    _isEqual = GetExport<IsEqualFn>(handle, "ryu_buffer_is_equal");

                    _isAvailable = _version != null && _findDirtyRange != null && _version() >= 0x00010000;
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

            if (NativeLibrary.TryLoad(LibraryName, Assembly.GetExecutingAssembly(), null, out IntPtr defaultHandle))
            {
                _loadedFrom = $"{LibraryName} (system library path)";
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

        public static bool TryFindDirtyRange(ReadOnlySpan<byte> src, ReadOnlySpan<byte> dst, out int offset, out int length)
        {
            offset = 0;
            length = 0;

            if (!_isAvailable || src.IsEmpty || dst.IsEmpty)
            {
                return false;
            }

            int len = Math.Min(src.Length, dst.Length);
            nuint outOff = 0;
            nuint outLen = 0;

            fixed (byte* pSrc = src)
            fixed (byte* pDst = dst)
            {
                int res = _findDirtyRange(pSrc, pDst, (nuint)len, &outOff, &outLen);
                if (res >= 0)
                {
                    offset = (int)outOff;
                    length = (int)outLen;
                    return true;
                }
            }

            return false;
        }

        public static bool TryDiffAndCopy(ReadOnlySpan<byte> src, Span<byte> dst, out int offset, out int length)
        {
            offset = 0;
            length = 0;

            if (!_isAvailable || src.IsEmpty || dst.IsEmpty)
            {
                return false;
            }

            int len = Math.Min(src.Length, dst.Length);
            nuint outOff = 0;
            nuint outLen = 0;

            fixed (byte* pSrc = src)
            fixed (byte* pDst = dst)
            {
                int res = _diffAndCopy(pSrc, pDst, (nuint)len, &outOff, &outLen);
                if (res >= 0)
                {
                    offset = (int)outOff;
                    length = (int)outLen;
                    return true;
                }
            }

            return false;
        }

        public static bool TryIsEqual(ReadOnlySpan<byte> a, ReadOnlySpan<byte> b, out bool isEqual)
        {
            isEqual = false;

            if (!_isAvailable || a.Length != b.Length)
            {
                return false;
            }

            if (a.IsEmpty)
            {
                isEqual = true;
                return true;
            }

            fixed (byte* pA = a)
            fixed (byte* pB = b)
            {
                int res = _isEqual(pA, pB, (nuint)a.Length);
                if (res >= 0)
                {
                    isEqual = res == 1;
                    return true;
                }
            }

            return false;
        }
    }
}
