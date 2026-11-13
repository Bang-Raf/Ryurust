using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Graphics.Gpu.Shader.DiskCache
{
    [StructLayout(LayoutKind.Sequential)]
    public struct NativeTocHeader
    {
        public uint Magic;
        public uint FormatVersion;
        public uint CodeGenVersion;
        public uint Padding;
        public ulong Timestamp;
        public ulong Reserved;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct NativeOffsetAndSize
    {
        public ulong Offset;
        public uint UncompressedSize;
        public uint CompressedSize;
    }

    public static unsafe class NativeShaderCache
    {
        private const string LibraryName = "ryu_shader_cache";

        private delegate uint VersionFn();
        private delegate int ValidateTocFn(byte* data, nuint len, uint expectedMagic, NativeTocHeader* outHeader);
        private delegate int ParseEntriesFn(byte* data, nuint len, NativeOffsetAndSize* outEntries, nuint maxEntries);
        private delegate int DecompressLz4Fn(byte* src, nuint srcLen, byte* dst, nuint dstCap, nuint* outLen);
        private delegate uint Crc32Fn(byte* data, nuint len);

        private static readonly VersionFn _version;
        private static readonly ValidateTocFn _validateToc;
        private static readonly ParseEntriesFn _parseEntries;
        private static readonly DecompressLz4Fn _decompressLz4;
        private static readonly Crc32Fn _crc32;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeShaderCache()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_shader_cache_version");
                    _validateToc = GetExport<ValidateTocFn>(handle, "ryu_shader_cache_validate_toc");
                    _parseEntries = GetExport<ParseEntriesFn>(handle, "ryu_shader_cache_parse_entries");
                    _decompressLz4 = GetExport<DecompressLz4Fn>(handle, "ryu_shader_cache_decompress_lz4");
                    _crc32 = GetExport<Crc32Fn>(handle, "ryu_shader_cache_crc32");

                    _isAvailable = _version != null && _validateToc != null && _version() >= 0x00010000;
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

        public static bool TryValidateToc(ReadOnlySpan<byte> data, uint expectedMagic, out NativeTocHeader header)
        {
            header = default;
            if (!_isAvailable || data.Length < sizeof(NativeTocHeader))
            {
                return false;
            }

            fixed (byte* pData = data)
            fixed (NativeTocHeader* pHdr = &header)
            {
                return _validateToc(pData, (nuint)data.Length, expectedMagic, pHdr) == 0;
            }
        }

        public static bool TryParseEntries(ReadOnlySpan<byte> data, Span<NativeOffsetAndSize> destination, out int count)
        {
            count = 0;
            if (!_isAvailable || data.Length < sizeof(NativeTocHeader) || destination.IsEmpty)
            {
                return false;
            }

            fixed (byte* pData = data)
            fixed (NativeOffsetAndSize* pDst = destination)
            {
                int res = _parseEntries(pData, (nuint)data.Length, pDst, (nuint)destination.Length);
                if (res >= 0)
                {
                    count = res;
                    return true;
                }
            }

            return false;
        }

        public static bool TryDecompressLz4(ReadOnlySpan<byte> src, Span<byte> dst, out int uncompressedLength)
        {
            uncompressedLength = 0;
            if (!_isAvailable || src.IsEmpty || dst.IsEmpty)
            {
                return false;
            }

            nuint outLen = 0;
            fixed (byte* pSrc = src)
            fixed (byte* pDst = dst)
            {
                if (_decompressLz4(pSrc, (nuint)src.Length, pDst, (nuint)dst.Length, &outLen) == 0)
                {
                    uncompressedLength = (int)outLen;
                    return true;
                }
            }

            return false;
        }

        public static uint ComputeCrc32(ReadOnlySpan<byte> data)
        {
            if (!_isAvailable || data.IsEmpty)
            {
                return 0;
            }

            fixed (byte* pData = data)
            {
                return _crc32(pData, (nuint)data.Length);
            }
        }
    }
}
