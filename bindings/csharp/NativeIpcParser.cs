using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.HLE.HOS.Ipc
{
    [StructLayout(LayoutKind.Sequential)]
    public struct NativeIpcBufferDesc
    {
        public ulong Position;
        public ulong Size;
        public byte Flags;
        private fixed byte _padding[7];
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct NativeIpcPtrBufferDesc
    {
        public ulong Position;
        public ulong Size;
        public uint Index;
        private uint _padding;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct NativeIpcRecvListBufferDesc
    {
        public ulong Position;
        public ulong Size;
    }

    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct NativeIpcHandleDescriptor
    {
        public uint HasPId;
        private uint _padding;
        public ulong PId;
        public uint CopyCount;
        public uint MoveCount;
        public fixed int CopyHandles[16];
        public fixed int MoveHandles[16];
    }

    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct NativeParsedIpcMessage
    {
        public uint MsgType;
        public uint HandleDescEnabled;
        public NativeIpcHandleDescriptor HandleDesc;

        public uint PtrBuffCount;
        public fixed byte PtrBuffs[16 * 24]; // 16 * sizeof(NativeIpcPtrBufferDesc)

        public uint SendBuffCount;
        public fixed byte SendBuffs[16 * 24]; // 16 * sizeof(NativeIpcBufferDesc)

        public uint RecvBuffCount;
        public fixed byte RecvBuffs[16 * 24];

        public uint XchgBuffCount;
        public fixed byte XchgBuffs[16 * 24];

        public uint RawDataOffset;
        public uint RawDataLength;

        public uint RecvListCount;
        private uint _reserved;
        public fixed byte RecvListBuffs[16 * 16]; // 16 * sizeof(NativeIpcRecvListBufferDesc)
    }

    public static unsafe class NativeIpcParser
    {
        private const string LibraryName = "ryu_ipc_parser";

        private delegate uint VersionFn();
        private delegate int ParseFn(byte* data, nuint dataLen, long cmdPtr, NativeParsedIpcMessage* @out);
        private delegate int SerializeCmifFn(
            NativeParsedIpcMessage* msg,
            byte* rawData,
            nuint rawLen,
            long cmdPtr,
            ulong recvListAddr,
            byte* outBuf,
            nuint outCap,
            nuint* outLen);
        private delegate int SerializeTipcFn(
            NativeParsedIpcMessage* msg,
            byte* rawData,
            nuint rawLen,
            byte* outBuf,
            nuint outCap,
            nuint* outLen);
        private delegate int GetBufferFn(NativeParsedIpcMessage* msg, uint index, ulong* outPos, ulong* outSize);

        private static readonly VersionFn _version;
        private static readonly ParseFn _parse;
        private static readonly SerializeCmifFn _serializeCmif;
        private static readonly SerializeTipcFn _serializeTipc;
        private static readonly GetBufferFn _getBuffer0x21;
        private static readonly GetBufferFn _getBuffer0x22;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeIpcParser()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_ipc_version");
                    _parse = GetExport<ParseFn>(handle, "ryu_ipc_parse");
                    _serializeCmif = GetExport<SerializeCmifFn>(handle, "ryu_ipc_serialize_cmif");
                    _serializeTipc = GetExport<SerializeTipcFn>(handle, "ryu_ipc_serialize_tipc");
                    _getBuffer0x21 = GetExport<GetBufferFn>(handle, "ryu_ipc_get_buffer_0x21");
                    _getBuffer0x22 = GetExport<GetBufferFn>(handle, "ryu_ipc_get_buffer_0x22");

                    _isAvailable = _version != null && _parse != null && _version() >= 0x00010000;
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

        public static bool TryParse(ReadOnlySpan<byte> data, long cmdPtr, out NativeParsedIpcMessage parsed)
        {
            parsed = default;
            if (!_isAvailable || data.Length < 8)
            {
                return false;
            }

            fixed (byte* pData = data)
            fixed (NativeParsedIpcMessage* pParsed = &parsed)
            {
                return _parse(pData, (nuint)data.Length, cmdPtr, pParsed) == 0;
            }
        }
    }
}
