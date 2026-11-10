using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Ryujinx.Input.Motion
{
    public sealed unsafe class NativeMotionFusion : IDisposable
    {
        private const string LibraryName = "ryu_motion_fusion";

        private delegate uint VersionFn();
        private delegate IntPtr CreateFn(float samplePeriod, float kp, float ki);
        private delegate void DestroyFn(IntPtr filter);
        private delegate void ResetFn(IntPtr filter);
        private delegate void SetPeriodFn(IntPtr filter, float period);
        private delegate void UpdateFn(IntPtr filter, float ax, float ay, float az, float gx, float gy, float gz);
        private delegate void GetQuatFn(IntPtr filter, float* outXyzw);
        private delegate void GetMatrixFn(IntPtr filter, float* outMatrix9);

        private static readonly VersionFn _version;
        private static readonly CreateFn _create;
        private static readonly DestroyFn _destroy;
        private static readonly ResetFn _reset;
        private static readonly SetPeriodFn _setPeriod;
        private static readonly UpdateFn _update;
        private static readonly GetQuatFn _getQuat;
        private static readonly GetMatrixFn _getMatrix;

        private static readonly bool _isAvailable;

        public static bool IsAvailable => _isAvailable;

        private IntPtr _handle;

        static NativeMotionFusion()
        {
            try
            {
                IntPtr handle = LoadNativeLibrary();
                if (handle != IntPtr.Zero)
                {
                    _version = GetExport<VersionFn>(handle, "ryu_motion_fusion_version");
                    _create = GetExport<CreateFn>(handle, "ryu_motion_filter_create");
                    _destroy = GetExport<DestroyFn>(handle, "ryu_motion_filter_destroy");
                    _reset = GetExport<ResetFn>(handle, "ryu_motion_filter_reset");
                    _setPeriod = GetExport<SetPeriodFn>(handle, "ryu_motion_filter_set_sample_period");
                    _update = GetExport<UpdateFn>(handle, "ryu_motion_filter_update");
                    _getQuat = GetExport<GetQuatFn>(handle, "ryu_motion_filter_get_quaternion");
                    _getMatrix = GetExport<GetMatrixFn>(handle, "ryu_motion_filter_get_orientation_matrix");

                    _isAvailable = _version != null && _create != null && _update != null && _version() >= 0x00010000;
                }
            }
            catch
            {
                _isAvailable = false;
            }
        }

        public NativeMotionFusion(float samplePeriod, float kp = 1.0f, float ki = 0.0f)
        {
            if (_isAvailable)
            {
                _handle = _create(samplePeriod, kp, ki);
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

            if (NativeLibrary.TryLoad(LibraryName, Assembly.GetExecutingAssembly(), null, out IntPtr defaultHandle))
            {
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
            return null;
        }

        public void SetSamplePeriod(float period)
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                _setPeriod(_handle, period);
            }
        }

        public void Reset()
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                _reset(_handle);
            }
        }

        public void Update(float ax, float ay, float az, float gx, float gy, float gz)
        {
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                _update(_handle, ax, ay, az, gx, gy, gz);
            }
        }

        public void GetQuaternion(out float x, out float y, out float z, out float w)
        {
            x = 0; y = 0; z = 0; w = 1;
            if (_handle != IntPtr.Zero && _isAvailable)
            {
                float* q = stackalloc float[4];
                _getQuat(_handle, q);
                x = q[0];
                y = q[1];
                z = q[2];
                w = q[3];
            }
        }

        public bool TryGetOrientationMatrix(Span<float> matrix9)
        {
            if (_handle != IntPtr.Zero && _isAvailable && matrix9.Length >= 9)
            {
                fixed (float* pM = matrix9)
                {
                    _getMatrix(_handle, pM);
                    return true;
                }
            }
            return false;
        }

        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                _destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }
    }
}
