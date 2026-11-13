using Ryujinx.Common.Logging;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Audio.Renderer.Dsp
{
    public static unsafe class NativeAudioMixer
    {
        private const string LibraryName = "ryu_audio_mixer";

        private delegate uint VersionFn();
        private delegate int MixFn(float* output, float* input, nuint count, float volume);
        private delegate int MixRampFn(float* output, float* input, nuint count, float volume0, float volume1, float* outLastSample);
        private delegate int MixBatchFn(float* output, float** inputs, float* volumes, nuint voiceCount, nuint sampleCount);
        private delegate int MixMatrixFn(float** outputs, nuint outputChannels, float* input, float* gains, nuint sampleCount);

        private static readonly VersionFn _version;
        private static readonly MixFn _mix;
        private static readonly MixRampFn _mixRamp;
        private static readonly MixBatchFn _mixBatch;
        private static readonly MixMatrixFn _mixMatrix;

        private static readonly bool _isAvailable;
        private static string _loadedFrom;
        private static string _loadFailure;
        private static readonly List<string> _missingExports = [];

        public static bool IsAvailable => _isAvailable;

        static NativeAudioMixer()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_audio_mixer_version");
                    _mix = GetExport<MixFn>(handle, "ryu_audio_mix");
                    _mixRamp = GetExport<MixRampFn>(handle, "ryu_audio_mix_ramp");
                    _mixBatch = GetExport<MixBatchFn>(handle, "ryu_audio_mix_batch");
                    _mixMatrix = GetExport<MixMatrixFn>(handle, "ryu_audio_mix_matrix");

                    _isAvailable = _version != null && _mix != null && _mixRamp != null && _version() >= 0x00010000;
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

        public static bool TryMix(Span<float> output, ReadOnlySpan<float> input, float volume)
        {
            if (!_isAvailable || output.IsEmpty || input.IsEmpty)
            {
                return false;
            }

            int count = Math.Min(output.Length, input.Length);

            fixed (float* pOut = output)
            fixed (float* pIn = input)
            {
                return _mix(pOut, pIn, (nuint)count, volume) == 0;
            }
        }

        public static bool TryMixRamp(Span<float> output, ReadOnlySpan<float> input, float volume0, float volume1, out float lastSample)
        {
            lastSample = 0.0f;
            if (!_isAvailable || output.IsEmpty || input.IsEmpty)
            {
                return false;
            }

            int count = Math.Min(output.Length, input.Length);
            float last = 0.0f;

            fixed (float* pOut = output)
            fixed (float* pIn = input)
            {
                if (_mixRamp(pOut, pIn, (nuint)count, volume0, volume1, &last) == 0)
                {
                    lastSample = last;
                    return true;
                }
            }

            return false;
        }
    }
}
