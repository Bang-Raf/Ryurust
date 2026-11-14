using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Graphics.Texture.Astc
{
    /// <summary>
    /// ASTC to RGBA8 decoding in the native <c>ryu_texture_astc</c> library. Its output is
    /// byte-identical to <see cref="AstcDecoder"/> (checked by golden tests against this C#
    /// decoder) and it is about 1.6-2x faster.
    /// </summary>
    public static unsafe class NativeTextureAstc
    {
        private const string LibraryName = "ryu_texture_astc";

        private delegate uint VersionFn();
        private delegate int DecodeFn(byte* input, nuint inputLen, byte* output, nuint outputLen,
            int blockWidth, int blockHeight, int width, int height, int depth, int levels, int layers);

        private static readonly VersionFn _version;
        private static readonly DecodeFn _decodeParallel;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeTextureAstc()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_texture_astc_version");
                    _decodeParallel = GetExport<DecodeFn>(handle, "ryu_astc_decode_to_rgba8_parallel");

                    _isAvailable = _version != null && _decodeParallel != null && _version() >= 0x00010000;
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

        /// <summary>Returned by <see cref="DecodeToRgba8"/> when the native library is not usable.</summary>
        public const int StatusUnavailable = -100;

        /// <summary>
        /// Decodes a whole texture (all levels and layers) on all cores. Returns the native
        /// status: 0 when every block decoded; 1 when the output was fully written but some
        /// block was invalid (texels zero, as in C#) or, with nothing written, when the input
        /// or output is too short; -1 / -2 for bad arguments; <see cref="StatusUnavailable"/>
        /// when the library is missing.
        /// </summary>
        public static int DecodeToRgba8(
            ReadOnlySpan<byte> data,
            Span<byte> output,
            int blockWidth,
            int blockHeight,
            int width,
            int height,
            int depth,
            int levels,
            int layers)
        {
            if (!_isAvailable || data.IsEmpty || output.IsEmpty)
            {
                return StatusUnavailable;
            }

            fixed (byte* pIn = data)
            fixed (byte* pOut = output)
            {
                return _decodeParallel(pIn, (nuint)data.Length, pOut, (nuint)output.Length,
                    blockWidth, blockHeight, width, height, depth, levels, layers);
            }
        }
    }
}
