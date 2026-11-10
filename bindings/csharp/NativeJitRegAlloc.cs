using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace ARMeilleure.CodeGen.RegisterAllocators
{
    public static unsafe class NativeJitRegAlloc
    {
        private const string LibraryName = "ryu_jit_regalloc";

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn();
        private delegate void DestroyFn(IntPtr ctx);
        private delegate void ResetFn(IntPtr ctx);
        private delegate uint AddIntervalFn(IntPtr ctx, uint id, int start, int end, byte regType, byte isFixed, short fixedReg);
        private delegate nuint RunPassFn(IntPtr ctx, uint intAvail, uint vecAvail, uint regCount);
        private delegate int GetResultFn(IntPtr ctx, nuint index, uint* outId, short* outReg, int* outSpill);
        private delegate int GetHighestIndexFn(int* span, nuint len);
        private delegate uint BitsetOrFn(ulong* dst, ulong* src, nuint count);
        private delegate uint BitsetAndNotFn(ulong* dst, ulong* src, nuint count);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly ResetFn _reset;
        private static readonly AddIntervalFn _addInterval;
        private static readonly RunPassFn _runPass;
        private static readonly GetResultFn _getResult;
        private static readonly GetHighestIndexFn _getHighestIndex;
        private static readonly BitsetOrFn _bitsetOr;
        private static readonly BitsetAndNotFn _bitsetAndNot;

        private static readonly bool _isAvailable;

        public static bool IsAvailable => _isAvailable;

        static NativeJitRegAlloc()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_jit_regalloc_version");
                    _create = GetExport<CreateFn>(handle, "ryu_regalloc_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_regalloc_destroy");
                    _reset = GetExport<ResetFn>(handle, "ryu_regalloc_reset");
                    _addInterval = GetExport<AddIntervalFn>(handle, "ryu_regalloc_add_interval");
                    _runPass = GetExport<RunPassFn>(handle, "ryu_regalloc_run_pass");
                    _getResult = GetExport<GetResultFn>(handle, "ryu_regalloc_get_result");
                    _getHighestIndex = GetExport<GetHighestIndexFn>(handle, "ryu_regalloc_get_highest_index");
                    _bitsetOr = GetExport<BitsetOrFn>(handle, "ryu_regalloc_bitset_or");
                    _bitsetAndNot = GetExport<BitsetAndNotFn>(handle, "ryu_regalloc_bitset_and_not");

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

        public static IntPtr Create() => _create != null ? _create() : IntPtr.Zero;

        public static void Destroy(IntPtr ctx) => _destroy?.Invoke(ctx);

        public static void Reset(IntPtr ctx)
        {
            if (ctx != IntPtr.Zero) _reset?.Invoke(ctx);
        }

        public static uint AddInterval(IntPtr ctx, uint id, int start, int end, byte regType, bool isFixed, short fixedReg)
        {
            return ctx != IntPtr.Zero && _addInterval != null
                ? _addInterval(ctx, id, start, end, regType, (byte)(isFixed ? 1 : 0), fixedReg)
                : 0;
        }

        public static int RunPass(IntPtr ctx, uint intAvail, uint vecAvail, uint regCount)
        {
            return ctx != IntPtr.Zero && _runPass != null
                ? (int)_runPass(ctx, intAvail, vecAvail, regCount)
                : 0;
        }

        public static bool GetResult(IntPtr ctx, int index, out uint id, out short reg, out int spillOffset)
        {
            id = 0; reg = -1; spillOffset = -1;
            if (ctx == IntPtr.Zero || _getResult == null) return false;
            fixed (uint* pId = &id)
            fixed (short* pReg = &reg)
            fixed (int* pSpill = &spillOffset)
            {
                return _getResult(ctx, (nuint)index, pId, pReg, pSpill) == 1;
            }
        }

        public static int GetHighestIndex(ReadOnlySpan<int> span)
        {
            if (span.IsEmpty || _getHighestIndex == null) return 0;
            fixed (int* p = span)
            {
                return _getHighestIndex(p, (nuint)span.Length);
            }
        }

        public static bool BitsetOr(long* dst, long* src, int wordCount)
        {
            if (_bitsetOr != null && wordCount > 0)
            {
                return _bitsetOr((ulong*)dst, (ulong*)src, (nuint)wordCount) != 0;
            }
            return false;
        }

        public static bool BitsetAndNot(long* dst, long* src, int wordCount)
        {
            if (_bitsetAndNot != null && wordCount > 0)
            {
                return _bitsetAndNot((ulong*)dst, (ulong*)src, (nuint)wordCount) != 0;
            }
            return false;
        }
    }
}
