using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.HLE.HOS.Services.SurfaceFlinger
{
    public static unsafe class NativeSurfaceFlingerQueue
    {
        private const string LibraryName = "ryu_surfaceflinger_queue";

        [StructLayout(LayoutKind.Sequential)]
        public struct QueueItemNative
        {
            public int Slot;
            public uint Transform;
            public uint ScalingMode;
            public uint Pad;
            public ulong FrameNumber;
            public long Timestamp;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct ParcelHeaderNative
        {
            public uint PayloadSize;
            public uint PayloadOffset;
            public uint ObjectsSize;
            public uint ObjectOffset;
        }

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn(uint maxBuffers);
        private delegate void DestroyFn(IntPtr q);
        private delegate void SetMaxBuffersFn(IntPtr q, uint maxBuffers);
        private delegate int FindFreeSlotFn(IntPtr q, uint maxBuffers);
        private delegate int DequeueSlotFn(IntPtr q, uint maxBuffers);
        private delegate int QueueBufferFn(IntPtr q, int slot, ulong frameNumber, long timestamp, uint transform, uint scalingMode);
        private delegate int AcquireBufferFn(IntPtr q, QueueItemNative* outItem);
        private delegate int ReleaseBufferFn(IntPtr q, int slot);
        private delegate int CancelBufferFn(IntPtr q, int slot);
        private delegate void SyncSlotStateFn(IntPtr q, int slot, byte state, ulong frameNumber);
        private delegate void GetCountsFn(IntPtr q, uint* outFree, uint* outDequeued, uint* outQueued, uint* outAcquired);
        private delegate int ParseHeaderFn(byte* buf, nuint len, ParcelHeaderNative* outHeader);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly SetMaxBuffersFn _setMaxBuffers;
        private static readonly FindFreeSlotFn _findFreeSlot;
        private static readonly DequeueSlotFn _dequeueSlot;
        private static readonly QueueBufferFn _queueBuffer;
        private static readonly AcquireBufferFn _acquireBuffer;
        private static readonly ReleaseBufferFn _releaseBuffer;
        private static readonly CancelBufferFn _cancelBuffer;
        private static readonly SyncSlotStateFn _syncSlotState;
        private static readonly GetCountsFn _getCounts;
        private static readonly ParseHeaderFn _parseHeader;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeSurfaceFlingerQueue()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_surfaceflinger_version");
                    _create = GetExport<CreateFn>(handle, "ryu_sf_queue_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_sf_queue_destroy");
                    _setMaxBuffers = GetExport<SetMaxBuffersFn>(handle, "ryu_sf_queue_set_max_buffers");
                    _findFreeSlot = GetExport<FindFreeSlotFn>(handle, "ryu_sf_queue_find_free_slot");
                    _dequeueSlot = GetExport<DequeueSlotFn>(handle, "ryu_sf_queue_dequeue_slot");
                    _queueBuffer = GetExport<QueueBufferFn>(handle, "ryu_sf_queue_queue_buffer");
                    _acquireBuffer = GetExport<AcquireBufferFn>(handle, "ryu_sf_queue_acquire_buffer");
                    _releaseBuffer = GetExport<ReleaseBufferFn>(handle, "ryu_sf_queue_release_buffer");
                    _cancelBuffer = GetExport<CancelBufferFn>(handle, "ryu_sf_queue_cancel_buffer");
                    _syncSlotState = GetExport<SyncSlotStateFn>(handle, "ryu_sf_queue_sync_slot_state");
                    _getCounts = GetExport<GetCountsFn>(handle, "ryu_sf_queue_get_counts");
                    _parseHeader = GetExport<ParseHeaderFn>(handle, "ryu_sf_parcel_parse_header");

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

        public static IntPtr Create(uint maxBuffers) => _create != null ? _create(maxBuffers) : IntPtr.Zero;

        public static void Destroy(IntPtr handle) => _destroy?.Invoke(handle);

        public static void SetMaxBuffers(IntPtr handle, uint maxBuffers)
        {
            if (handle != IntPtr.Zero) _setMaxBuffers?.Invoke(handle, maxBuffers);
        }

        public static int FindFreeSlot(IntPtr handle, uint maxBuffers)
        {
            return handle != IntPtr.Zero && _findFreeSlot != null ? _findFreeSlot(handle, maxBuffers) : -1;
        }

        public static int DequeueSlot(IntPtr handle, uint maxBuffers)
        {
            return handle != IntPtr.Zero && _dequeueSlot != null ? _dequeueSlot(handle, maxBuffers) : -1;
        }

        public static bool QueueBuffer(IntPtr handle, int slot, ulong frameNumber, long timestamp, uint transform = 0, uint scalingMode = 0)
        {
            return handle != IntPtr.Zero && _queueBuffer != null && _queueBuffer(handle, slot, frameNumber, timestamp, transform, scalingMode) == 1;
        }

        public static bool AcquireBuffer(IntPtr handle, out QueueItemNative item)
        {
            item = default;
            if (handle == IntPtr.Zero || _acquireBuffer == null) return false;
            fixed (QueueItemNative* p = &item)
            {
                return _acquireBuffer(handle, p) == 1;
            }
        }

        public static bool ReleaseBuffer(IntPtr handle, int slot)
        {
            return handle != IntPtr.Zero && _releaseBuffer != null && _releaseBuffer(handle, slot) == 1;
        }

        public static bool CancelBuffer(IntPtr handle, int slot)
        {
            return handle != IntPtr.Zero && _cancelBuffer != null && _cancelBuffer(handle, slot) == 1;
        }

        public static void SyncSlotState(IntPtr handle, int slot, byte state, ulong frameNumber)
        {
            if (handle != IntPtr.Zero) _syncSlotState?.Invoke(handle, slot, state, frameNumber);
        }

        public static void GetCounts(IntPtr handle, out uint free, out uint dequeued, out uint queued, out uint acquired)
        {
            free = 0; dequeued = 0; queued = 0; acquired = 0;
            if (handle == IntPtr.Zero || _getCounts == null) return;
            fixed (uint* pf = &free, pd = &dequeued, pq = &queued, pa = &acquired)
            {
                _getCounts(handle, pf, pd, pq, pa);
            }
        }

        public static bool ParseHeader(ReadOnlySpan<byte> buf, out ParcelHeaderNative header)
        {
            header = default;
            if (buf.Length < 16 || _parseHeader == null) return false;
            fixed (byte* pBuf = buf)
            fixed (ParcelHeaderNative* pHeader = &header)
            {
                return _parseHeader(pBuf, (nuint)buf.Length, pHeader) == 1;
            }
        }
    }
}
