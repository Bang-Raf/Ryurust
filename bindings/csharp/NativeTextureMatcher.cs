using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Graphics.Gpu.Image
{
    internal static class NativeTextureMatcher
    {
        private const string LibraryName = "ryu_texture_matcher";

        private delegate uint VersionFn();
        private delegate ulong PackKeyFn(
            ushort format,
            byte target,
            byte swizzleR,
            byte swizzleG,
            byte swizzleB,
            byte swizzleA,
            byte depthStencilMode,
            byte samplesX,
            byte samplesY,
            int isLinear,
            byte bpp,
            int isCompressed,
            int isDepthStencil);

        private delegate int SamplerMatchesFn(ulong keyA, ulong keyB);
        private delegate int TargetSamplesCompatibleFn(ulong keyA, ulong keyB);
        private delegate int FormatMatchesFn(uint formatA, uint formatB, int forSampler, int depthAlias);
        private delegate int ViewTargetCompatibleFn(uint targetA, uint targetB, int supportsCubemap);
        private delegate uint GetFormatClassFn(uint format);
        private delegate int ViewFormatCompatibleFn(ulong keyA, ulong keyB, int capsIncompatA, int capsIncompatB, int forSampler, int depthAlias);

        private static readonly VersionFn _version;
        private static readonly PackKeyFn _packKey;
        private static readonly SamplerMatchesFn _samplerMatches;
        private static readonly TargetSamplesCompatibleFn _targetSamplesCompatible;
        private static readonly FormatMatchesFn _formatMatches;
        private static readonly ViewTargetCompatibleFn _viewTargetCompatible;
        private static readonly GetFormatClassFn _getFormatClass;
        private static readonly ViewFormatCompatibleFn _viewFormatCompatible;

        private static readonly bool _isAvailable;

        public static bool IsAvailable => _isAvailable;

        static NativeTextureMatcher()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_texture_matcher_version");
                    _packKey = GetExport<PackKeyFn>(handle, "ryu_texture_pack_key");
                    _samplerMatches = GetExport<SamplerMatchesFn>(handle, "ryu_texture_sampler_matches");
                    _targetSamplesCompatible = GetExport<TargetSamplesCompatibleFn>(handle, "ryu_texture_target_samples_compatible");
                    _formatMatches = GetExport<FormatMatchesFn>(handle, "ryu_texture_format_matches");
                    _viewTargetCompatible = GetExport<ViewTargetCompatibleFn>(handle, "ryu_texture_view_target_compatible");
                    _getFormatClass = GetExport<GetFormatClassFn>(handle, "ryu_texture_get_format_class");
                    _viewFormatCompatible = GetExport<ViewFormatCompatibleFn>(handle, "ryu_texture_view_format_compatible");

                    _isAvailable = _version != null && _version() >= 0x00010000;
                }
            }
            catch
            {
                _isAvailable = false;
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

            if (NativeLibrary.TryLoad(LibraryName, Assembly.GetExecutingAssembly(), null, out IntPtr resolved))
            {
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
            return null;
        }

        public static ulong PackKey(
            ushort format,
            byte target,
            byte swizzleR,
            byte swizzleG,
            byte swizzleB,
            byte swizzleA,
            byte depthStencilMode,
            byte samplesX,
            byte samplesY,
            bool isLinear,
            byte bpp,
            bool isCompressed,
            bool isDepthStencil)
        {
            if (_packKey != null)
            {
                return _packKey(
                    format,
                    target,
                    swizzleR,
                    swizzleG,
                    swizzleB,
                    swizzleA,
                    depthStencilMode,
                    samplesX,
                    samplesY,
                    isLinear ? 1 : 0,
                    bpp,
                    isCompressed ? 1 : 0,
                    isDepthStencil ? 1 : 0);
            }

            // Fallback bit-packing in C#
            ulong v = (ulong)format & 0xFFFF;
            v |= ((ulong)target & 0xFF) << 16;
            v |= ((ulong)swizzleR & 0x07) << 24;
            v |= ((ulong)swizzleG & 0x07) << 27;
            v |= ((ulong)swizzleB & 0x07) << 30;
            v |= ((ulong)swizzleA & 0x07) << 33;
            v |= ((ulong)depthStencilMode & 0x01) << 36;
            v |= ((ulong)samplesX & 0x0F) << 37;
            v |= ((ulong)samplesY & 0x0F) << 41;
            if (isLinear) v |= 1UL << 45;
            v |= ((ulong)bpp & 0x3F) << 46;
            if (isCompressed) v |= 1UL << 58;
            if (isDepthStencil) v |= 1UL << 59;
            return v;
        }

        public static bool SamplerMatches(ulong keyA, ulong keyB)
        {
            if (_samplerMatches != null)
            {
                return _samplerMatches(keyA, keyB) != 0;
            }
            const ulong samplerMask = 0x0000_001F_FF00_0000;
            return ((keyA ^ keyB) & samplerMask) == 0;
        }

        public static bool TargetSamplesCompatible(ulong keyA, ulong keyB)
        {
            if (_targetSamplesCompatible != null)
            {
                return _targetSamplesCompatible(keyA, keyB) != 0;
            }
            const ulong targetMask = 0x0000_0000_00FF_0000;
            const ulong samplesMask = 0x0000_1FE0_0000_0000;
            const ulong targetAndSamplesMask = targetMask | samplesMask;
            return ((keyA ^ keyB) & targetAndSamplesMask) == 0;
        }

        public static TextureMatchQuality FormatMatches(uint formatA, uint formatB, bool forSampler, bool depthAlias)
        {
            if (_formatMatches != null)
            {
                return (TextureMatchQuality)_formatMatches(formatA, formatB, forSampler ? 1 : 0, depthAlias ? 1 : 0);
            }
            return formatA == formatB ? TextureMatchQuality.Perfect : TextureMatchQuality.NoMatch;
        }

        public static TextureViewCompatibility ViewTargetCompatible(uint targetA, uint targetB, bool supportsCubemap)
        {
            if (_viewTargetCompatible != null)
            {
                return (TextureViewCompatibility)_viewTargetCompatible(targetA, targetB, supportsCubemap ? 1 : 0);
            }
            return targetA == targetB ? TextureViewCompatibility.Full : TextureViewCompatibility.Incompatible;
        }

        public static byte GetFormatClass(uint format)
        {
            if (_getFormatClass != null)
            {
                return (byte)_getFormatClass(format);
            }
            return 0;
        }

        public static TextureViewCompatibility ViewFormatCompatible(
            ulong keyA,
            ulong keyB,
            bool capsIncompatA,
            bool capsIncompatB,
            bool forSampler,
            bool depthAlias)
        {
            if (_viewFormatCompatible != null)
            {
                return (TextureViewCompatibility)_viewFormatCompatible(
                    keyA,
                    keyB,
                    capsIncompatA ? 1 : 0,
                    capsIncompatB ? 1 : 0,
                    forSampler ? 1 : 0,
                    depthAlias ? 1 : 0);
            }
            return TextureViewCompatibility.Incompatible;
        }
    }
}
