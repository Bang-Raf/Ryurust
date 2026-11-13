using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Graphics.Vulkan
{
    [StructLayout(LayoutKind.Sequential)]
    public struct NativeDescriptorBufferInfo
    {
        public ulong BufferHandle;
        public ulong Offset;
        public ulong Range;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct NativeDescriptorImageInfo
    {
        public ulong SamplerHandle;
        public ulong ImageViewHandle;
        public uint ImageLayout;
        public uint Pad;
    }

    public static unsafe class NativeVulkanDescriptors
    {
        private const string LibraryName = "ryu_vulkan_descriptors";

        public const uint SetUniform = 0;
        public const uint SetStorage = 1;
        public const uint SetTexture = 2;
        public const uint SetImage = 3;

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn();
        private delegate void DestroyFn(IntPtr state);
        private delegate void ResetFn(IntPtr state);
        private delegate int UpdateBufferFn(IntPtr state, uint setType, uint index, ulong bufferHandle, ulong offset, ulong range);
        private delegate int UpdateImageFn(IntPtr state, uint setType, uint index, ulong samplerHandle, ulong imageViewHandle, uint layout);
        private delegate int FilterRedundantBuffersFn(IntPtr state, uint setType, uint* outIndices, NativeDescriptorBufferInfo* outInfos, nuint capacity, nuint* outCount);
        private delegate int FilterRedundantImagesFn(IntPtr state, uint setType, uint* outIndices, NativeDescriptorImageInfo* outInfos, nuint capacity, nuint* outCount);
        private delegate int PackTemplateSliceFn(IntPtr state, uint setType, uint binding, uint count, byte* outBuf, nuint outBufLen, nuint* outWritten);
        private delegate int CommitAllFn(IntPtr state, uint setType);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly ResetFn _reset;
        private static readonly UpdateBufferFn _updateBuffer;
        private static readonly UpdateImageFn _updateImage;
        private static readonly FilterRedundantBuffersFn _filterRedundantBuffers;
        private static readonly FilterRedundantImagesFn _filterRedundantImages;
        private static readonly PackTemplateSliceFn _packTemplateSlice;
        private static readonly CommitAllFn _commitAll;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeVulkanDescriptors()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_vulkan_descriptors_version");
                    _create = GetExport<CreateFn>(handle, "ryu_vulkan_descriptors_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_vulkan_descriptors_destroy");
                    _reset = GetExport<ResetFn>(handle, "ryu_vulkan_descriptors_reset");
                    _updateBuffer = GetExport<UpdateBufferFn>(handle, "ryu_vulkan_descriptors_update_buffer");
                    _updateImage = GetExport<UpdateImageFn>(handle, "ryu_vulkan_descriptors_update_image");
                    _filterRedundantBuffers = GetExport<FilterRedundantBuffersFn>(handle, "ryu_vulkan_descriptors_filter_redundant_buffers");
                    _filterRedundantImages = GetExport<FilterRedundantImagesFn>(handle, "ryu_vulkan_descriptors_filter_redundant_images");
                    _packTemplateSlice = GetExport<PackTemplateSliceFn>(handle, "ryu_vulkan_descriptors_pack_template_slice");
                    _commitAll = GetExport<CommitAllFn>(handle, "ryu_vulkan_descriptors_commit_all");

                    _isAvailable = _version != null && _create != null && _version() >= 0x00010000;
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

        public static IntPtr Create() => _create != null ? _create() : IntPtr.Zero;

        public static void Destroy(IntPtr state) => _destroy?.Invoke(state);

        public static void Reset(IntPtr state) => _reset?.Invoke(state);

        public static bool UpdateBuffer(IntPtr state, uint setType, uint index, ulong bufferHandle, ulong offset, ulong range)
        {
            if (_updateBuffer == null || state == IntPtr.Zero)
            {
                return false;
            }
            return _updateBuffer(state, setType, index, bufferHandle, offset, range) > 0;
        }

        public static bool UpdateImage(IntPtr state, uint setType, uint index, ulong samplerHandle, ulong imageViewHandle, uint layout)
        {
            if (_updateImage == null || state == IntPtr.Zero)
            {
                return false;
            }
            return _updateImage(state, setType, index, samplerHandle, imageViewHandle, layout) > 0;
        }

        public static int FilterRedundantBuffers(IntPtr state, uint setType, Span<uint> outIndices, Span<NativeDescriptorBufferInfo> outInfos, out int count)
        {
            count = 0;
            if (_filterRedundantBuffers == null || state == IntPtr.Zero)
            {
                return -1;
            }

            nuint written = 0;
            fixed (uint* pIdx = outIndices)
            fixed (NativeDescriptorBufferInfo* pInfo = outInfos)
            {
                int res = _filterRedundantBuffers(state, setType, pIdx, pInfo, (nuint)outIndices.Length, &written);
                count = (int)written;
                return res;
            }
        }

        public static int FilterRedundantImages(IntPtr state, uint setType, Span<uint> outIndices, Span<NativeDescriptorImageInfo> outInfos, out int count)
        {
            count = 0;
            if (_filterRedundantImages == null || state == IntPtr.Zero)
            {
                return -1;
            }

            nuint written = 0;
            fixed (uint* pIdx = outIndices)
            fixed (NativeDescriptorImageInfo* pInfo = outInfos)
            {
                int res = _filterRedundantImages(state, setType, pIdx, pInfo, (nuint)outIndices.Length, &written);
                count = (int)written;
                return res;
            }
        }

        public static int PackTemplateSlice(IntPtr state, uint setType, uint binding, uint count, Span<byte> outBuf, out int written)
        {
            written = 0;
            if (_packTemplateSlice == null || state == IntPtr.Zero)
            {
                return -1;
            }

            nuint w = 0;
            fixed (byte* pBuf = outBuf)
            {
                int res = _packTemplateSlice(state, setType, binding, count, pBuf, (nuint)outBuf.Length, &w);
                written = (int)w;
                return res;
            }
        }

        public static void CommitAll(IntPtr state, uint setType) => _commitAll?.Invoke(state, setType);
    }
}
