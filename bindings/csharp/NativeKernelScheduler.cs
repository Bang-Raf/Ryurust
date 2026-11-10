using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.HLE.HOS.Kernel.Threading
{
    public static unsafe class NativeKernelScheduler
    {
        private const string LibraryName = "ryu_kernel_scheduler";

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn();
        private delegate void DestroyFn(IntPtr state);
        private delegate void ResetFn(IntPtr state);
        private delegate void ScheduleFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate void SchedulePrependFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate void UnscheduleFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate ulong RescheduleFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate void SuggestFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate void UnsuggestFn(IntPtr state, uint prio, uint core, ulong threadId);
        private delegate void TransferToCoreFn(IntPtr state, uint prio, int srcCore, int dstCore, ulong threadId);
        private delegate ulong FirstScheduledFn(IntPtr state, uint core);
        private delegate ulong FirstSuggestedFn(IntPtr state, uint core);
        private delegate ulong ElementAtScheduledFn(IntPtr state, uint core, uint index);
        private delegate ulong DynamicPrioFirstFn(IntPtr state, uint core, uint prio);
        private delegate int HasScheduledFn(IntPtr state, uint core);
        private delegate int HasSuggestedFn(IntPtr state, uint core);
        private delegate ulong GetScheduledMaskFn(IntPtr state, uint core);
        private delegate ulong GetSuggestedMaskFn(IntPtr state, uint core);
        private delegate uint ExportScheduledFn(IntPtr state, uint core, ulong* outBuf, uint maxCount);
        private delegate uint ExportSuggestedFn(IntPtr state, uint core, ulong* outBuf, uint maxCount);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly ResetFn _reset;
        private static readonly ScheduleFn _schedule;
        private static readonly SchedulePrependFn _schedulePrepend;
        private static readonly UnscheduleFn _unschedule;
        private static readonly RescheduleFn _reschedule;
        private static readonly SuggestFn _suggest;
        private static readonly UnsuggestFn _unsuggest;
        private static readonly TransferToCoreFn _transferToCore;
        private static readonly FirstScheduledFn _firstScheduled;
        private static readonly FirstSuggestedFn _firstSuggested;
        private static readonly ElementAtScheduledFn _elementAtScheduled;
        private static readonly DynamicPrioFirstFn _dynamicPrioFirst;
        private static readonly HasScheduledFn _hasScheduled;
        private static readonly HasSuggestedFn _hasSuggested;
        private static readonly GetScheduledMaskFn _getScheduledMask;
        private static readonly GetSuggestedMaskFn _getSuggestedMask;
        private static readonly ExportScheduledFn _exportScheduled;
        private static readonly ExportSuggestedFn _exportSuggested;

        private static readonly bool _isAvailable;

        public static bool IsAvailable => _isAvailable;

        static NativeKernelScheduler()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_scheduler_version");
                    _create = GetExport<CreateFn>(handle, "ryu_scheduler_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_scheduler_destroy");
                    _reset = GetExport<ResetFn>(handle, "ryu_scheduler_reset");
                    _schedule = GetExport<ScheduleFn>(handle, "ryu_scheduler_schedule");
                    _schedulePrepend = GetExport<SchedulePrependFn>(handle, "ryu_scheduler_schedule_prepend");
                    _unschedule = GetExport<UnscheduleFn>(handle, "ryu_scheduler_unschedule");
                    _reschedule = GetExport<RescheduleFn>(handle, "ryu_scheduler_reschedule");
                    _suggest = GetExport<SuggestFn>(handle, "ryu_scheduler_suggest");
                    _unsuggest = GetExport<UnsuggestFn>(handle, "ryu_scheduler_unsuggest");
                    _transferToCore = GetExport<TransferToCoreFn>(handle, "ryu_scheduler_transfer_to_core");
                    _firstScheduled = GetExport<FirstScheduledFn>(handle, "ryu_scheduler_first_scheduled");
                    _firstSuggested = GetExport<FirstSuggestedFn>(handle, "ryu_scheduler_first_suggested");
                    _elementAtScheduled = GetExport<ElementAtScheduledFn>(handle, "ryu_scheduler_element_at_scheduled");
                    _dynamicPrioFirst = GetExport<DynamicPrioFirstFn>(handle, "ryu_scheduler_dynamic_prio_first");
                    _hasScheduled = GetExport<HasScheduledFn>(handle, "ryu_scheduler_has_scheduled");
                    _hasSuggested = GetExport<HasSuggestedFn>(handle, "ryu_scheduler_has_suggested");
                    _getScheduledMask = GetExport<GetScheduledMaskFn>(handle, "ryu_scheduler_get_scheduled_mask");
                    _getSuggestedMask = GetExport<GetSuggestedMaskFn>(handle, "ryu_scheduler_get_suggested_mask");
                    _exportScheduled = GetExport<ExportScheduledFn>(handle, "ryu_scheduler_export_scheduled");
                    _exportSuggested = GetExport<ExportSuggestedFn>(handle, "ryu_scheduler_export_suggested");

                    _isAvailable = _version != null && _create != null && _version() >= 0x00010000;
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

        public static void Destroy(IntPtr state) => _destroy?.Invoke(state);

        public static void Reset(IntPtr state) => _reset?.Invoke(state);

        public static void Schedule(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_schedule != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                _schedule(state, (uint)prio, (uint)core, threadId);
            }
        }

        public static void SchedulePrepend(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_schedulePrepend != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                _schedulePrepend(state, (uint)prio, (uint)core, threadId);
            }
        }

        public static void Unschedule(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_unschedule != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                _unschedule(state, (uint)prio, (uint)core, threadId);
            }
        }

        public static ulong Reschedule(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_reschedule != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                return _reschedule(state, (uint)prio, (uint)core, threadId);
            }
            return 0;
        }

        public static void Suggest(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_suggest != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                _suggest(state, (uint)prio, (uint)core, threadId);
            }
        }

        public static void Unsuggest(IntPtr state, int prio, int core, ulong threadId)
        {
            if (_unsuggest != null && state != IntPtr.Zero && prio >= 0 && core >= 0)
            {
                _unsuggest(state, (uint)prio, (uint)core, threadId);
            }
        }

        public static void TransferToCore(IntPtr state, int prio, int srcCore, int dstCore, ulong threadId)
        {
            if (_transferToCore != null && state != IntPtr.Zero && prio >= 0)
            {
                _transferToCore(state, (uint)prio, srcCore, dstCore, threadId);
            }
        }

        public static ulong FirstScheduled(IntPtr state, int core)
        {
            if (_firstScheduled != null && state != IntPtr.Zero && core >= 0)
            {
                return _firstScheduled(state, (uint)core);
            }
            return 0;
        }

        public static ulong FirstSuggested(IntPtr state, int core)
        {
            if (_firstSuggested != null && state != IntPtr.Zero && core >= 0)
            {
                return _firstSuggested(state, (uint)core);
            }
            return 0;
        }

        public static ulong ElementAtScheduled(IntPtr state, int core, int index)
        {
            if (_elementAtScheduled != null && state != IntPtr.Zero && core >= 0 && index >= 0)
            {
                return _elementAtScheduled(state, (uint)core, (uint)index);
            }
            return 0;
        }

        public static ulong DynamicPrioFirst(IntPtr state, int core, int prio)
        {
            if (_dynamicPrioFirst != null && state != IntPtr.Zero && core >= 0 && prio >= 0)
            {
                return _dynamicPrioFirst(state, (uint)core, (uint)prio);
            }
            return 0;
        }

        public static bool HasScheduled(IntPtr state, int core)
        {
            if (_hasScheduled != null && state != IntPtr.Zero && core >= 0)
            {
                return _hasScheduled(state, (uint)core) != 0;
            }
            return false;
        }

        public static bool HasSuggested(IntPtr state, int core)
        {
            if (_hasSuggested != null && state != IntPtr.Zero && core >= 0)
            {
                return _hasSuggested(state, (uint)core) != 0;
            }
            return false;
        }

        public static ulong GetScheduledMask(IntPtr state, int core)
        {
            if (_getScheduledMask != null && state != IntPtr.Zero && core >= 0)
            {
                return _getScheduledMask(state, (uint)core);
            }
            return 0;
        }

        public static ulong GetSuggestedMask(IntPtr state, int core)
        {
            if (_getSuggestedMask != null && state != IntPtr.Zero && core >= 0)
            {
                return _getSuggestedMask(state, (uint)core);
            }
            return 0;
        }

        public static int ExportScheduled(IntPtr state, int core, Span<ulong> outBuf)
        {
            if (_exportScheduled != null && state != IntPtr.Zero && core >= 0 && !outBuf.IsEmpty)
            {
                fixed (ulong* p = outBuf)
                {
                    return (int)_exportScheduled(state, (uint)core, p, (uint)outBuf.Length);
                }
            }
            return 0;
        }

        public static int ExportSuggested(IntPtr state, int core, Span<ulong> outBuf)
        {
            if (_exportSuggested != null && state != IntPtr.Zero && core >= 0 && !outBuf.IsEmpty)
            {
                fixed (ulong* p = outBuf)
                {
                    return (int)_exportSuggested(state, (uint)core, p, (uint)outBuf.Length);
                }
            }
            return 0;
        }
    }
}
