pub const SAMPLE_RATE_COEFFICIENT: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline(always)]
    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn to_rotation_matrix_3x3(&self) -> [f32; 9] {
        let x = self.x;
        let y = self.y;
        let z = self.z;
        let w = self.w;

        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let wz = w * z;
        let xz = x * z;
        let wy = w * y;
        let yz = y * z;
        let wx = w * x;

        // max/min maps NaN to -1.0 while `f32::clamp` would return NaN; keep the existing result.
        #[allow(clippy::manual_clamp)]
        let clamp = |v: f32| v.max(-1.0).min(1.0);

        [
            clamp(1.0 - 2.0 * (yy + zz)), // M11
            clamp(2.0 * (xy + wz)),       // M12
            clamp(2.0 * (xz - wy)),       // M13
            clamp(2.0 * (xy - wz)),       // M21
            clamp(1.0 - 2.0 * (xx + zz)), // M22
            clamp(2.0 * (yz + wx)),       // M23
            clamp(2.0 * (xz + wy)),       // M31
            clamp(2.0 * (yz - wx)),       // M32
            clamp(1.0 - 2.0 * (xx + yy)), // M33
        ]
    }
}

#[derive(Debug, Clone)]
pub struct MahonySensorFilter {
    pub sample_period: f32,
    pub kp: f32,
    pub ki: f32,
    pub quaternion: Quaternion,
    integral_error: Vector3,
}

impl MahonySensorFilter {
    pub fn new(sample_period: f32, kp: f32, ki: f32) -> Self {
        Self {
            sample_period,
            kp,
            ki,
            quaternion: Quaternion::identity(),
            integral_error: Vector3::zero(),
        }
    }

    pub fn reset(&mut self) {
        self.quaternion = Quaternion::identity();
        self.integral_error = Vector3::zero();
    }

    pub fn update(&mut self, mut accel: Vector3, mut gyro: Vector3) {
        let accel_len = accel.length();
        if accel_len == 0.0 || !accel_len.is_finite() {
            return;
        }

        let norm_accel = 1.0 / accel_len;
        accel.x *= norm_accel;
        accel.y *= norm_accel;
        accel.z *= norm_accel;

        let q2 = self.quaternion.x;
        let q3 = self.quaternion.y;
        let q4 = self.quaternion.z;
        let mut q1 = self.quaternion.w;

        // Estimated direction of gravity
        let gravity_x = 2.0 * (q2 * q4 - q1 * q3);
        let gravity_y = 2.0 * (q1 * q2 + q3 * q4);
        let gravity_z = q1 * q1 - q2 * q2 - q3 * q3 + q4 * q4;

        // Error is cross product between estimated and measured direction of gravity
        let error_x = accel.y * gravity_z - accel.z * gravity_y;
        let error_y = accel.z * gravity_x - accel.x * gravity_z;
        let error_z = accel.x * gravity_y - accel.y * gravity_x;

        if self.ki > 0.0 {
            self.integral_error.x += error_x;
            self.integral_error.y += error_y;
            self.integral_error.z += error_z;
        } else {
            self.integral_error = Vector3::zero();
        }

        // Apply feedback terms
        gyro.x += self.kp * error_x + self.ki * self.integral_error.x;
        gyro.y += self.kp * error_y + self.ki * self.integral_error.y;
        gyro.z += self.kp * error_z + self.ki * self.integral_error.z;

        // Integrate rate of change of quaternion
        let delta_x = q2;
        let delta_y = q3;
        let delta_z = q4;

        let dt_coeff = SAMPLE_RATE_COEFFICIENT * self.sample_period;

        q1 += (-q2 * gyro.x - q3 * gyro.y - q4 * gyro.z) * dt_coeff;
        let mut new_q2 = q2 + (q1 * gyro.x + delta_y * gyro.z - delta_z * gyro.y) * dt_coeff;
        let mut new_q3 = q3 + (q1 * gyro.y - delta_x * gyro.z + delta_z * gyro.x) * dt_coeff;
        let mut new_q4 = q4 + (q1 * gyro.z + delta_x * gyro.y - delta_y * gyro.x) * dt_coeff;

        // Normalise quaternion
        let q_len_sq = new_q2 * new_q2 + new_q3 * new_q3 + new_q4 * new_q4 + q1 * q1;
        let q_len = q_len_sq.sqrt();
        if q_len == 0.0 || !q_len.is_finite() {
            return;
        }

        let norm_q = 1.0 / q_len;
        new_q2 *= norm_q;
        new_q3 *= norm_q;
        new_q4 *= norm_q;
        q1 *= norm_q;

        self.quaternion = Quaternion::new(new_q2, new_q3, new_q4, q1);
    }
}
