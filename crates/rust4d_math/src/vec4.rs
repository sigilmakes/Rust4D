//! 4D Vector type

use bytemuck::{Pod, Zeroable};
use serde::{Serialize, Deserialize};

/// 4D Vector with x, y, z, w components
/// The w component represents the 4th spatial dimension (ana/kata)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };
    pub const W: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Create a new Vec4
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Length squared (faster than length)
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Length (magnitude)
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize to unit length
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    /// Extract the xyz components as an array (for 3D rendering)
    #[inline]
    pub fn xyz(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }

    /// Clamp each component between corresponding min and max values
    #[inline]
    pub fn clamp_components(self, min: Self, max: Self) -> Self {
        Self::new(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
            self.w.clamp(min.w, max.w),
        )
    }

    /// Component-wise minimum
    #[inline]
    pub fn min_components(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
            self.w.min(other.w),
        )
    }

    /// Component-wise maximum
    #[inline]
    pub fn max_components(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
            self.w.max(other.w),
        )
    }

    /// Component-wise absolute value
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs(), self.w.abs())
    }

    /// Return the component with the sign of each normal component
    #[inline]
    pub fn sign(self) -> Self {
        Self::new(
            if self.x >= 0.0 { 1.0 } else { -1.0 },
            if self.y >= 0.0 { 1.0 } else { -1.0 },
            if self.z >= 0.0 { 1.0 } else { -1.0 },
            if self.w >= 0.0 { 1.0 } else { -1.0 },
        )
    }

    /// Component-wise multiplication (Hadamard product)
    #[inline]
    pub fn component_mul(self, other: Self) -> Self {
        Self::new(
            self.x * other.x,
            self.y * other.y,
            self.z * other.z,
            self.w * other.w,
        )
    }

    /// Distance between two vectors
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared distance between two vectors (avoids sqrt)
    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    /// Angle between two vectors in radians.
    ///
    /// Returns the angle in the range [0, π]. Both vectors should be
    /// non-zero; the result is undefined for zero vectors.
    #[inline]
    pub fn angle_to(self, other: Self) -> f32 {
        let cos_angle = self.normalized().dot(other.normalized());
        // Clamp to [-1, 1] to handle floating point errors
        cos_angle.clamp(-1.0, 1.0).acos()
    }

    /// Epsilon for geometric comparisons (parallelism, near-zero length).
    const GEOMETRIC_EPSILON: f32 = 1e-6;

    /// Rotate `self` towards `target` by at most `max_radians`.
    ///
    /// If the angle between `self` and `target` is less than `max_radians`,
    /// returns `target`. Otherwise, returns a vector rotated from `self`
    /// towards `target` by exactly `max_radians`.
    ///
    /// Both vectors should be normalized for best results.
    pub fn rotate_towards(self, target: Self, max_radians: f32) -> Self {
        let min_cos = max_radians.cos();
        let cos_ang = self.dot(target);

        // Already within max_radians of target
        if cos_ang >= min_cos {
            return target;
        }

        let min_sin = max_radians.sin();

        // Compute perpendicular component (target - projection of target onto self)
        let mut perp = target - self * cos_ang;
        let perp_mag = perp.length();

        if perp_mag <= Self::GEOMETRIC_EPSILON {
            // Vectors are nearly parallel or anti-parallel.
            // Use the "most orthogonal basis vector" approach: pick the standard
            // basis axis with the smallest component in self, then orthogonalize.
            let abs = self.abs();
            let basis = if abs.x <= abs.y && abs.x <= abs.z && abs.x <= abs.w {
                Self::X
            } else if abs.y <= abs.z && abs.y <= abs.w {
                Self::Y
            } else if abs.z <= abs.w {
                Self::Z
            } else {
                Self::W
            };
            perp = basis - self * self.dot(basis);
        }

        // Rotate: self * cos(max_radians) + perp_normalized * sin(max_radians)
        self * min_cos + perp.normalized() * min_sin
    }
}

// Operator overloads

impl std::ops::Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self::new(
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
            self.w + other.w,
        )
    }
}

impl std::ops::AddAssign for Vec4 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
        self.w += other.w;
    }
}

impl std::ops::Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self::new(
            self.x - other.x,
            self.y - other.y,
            self.z - other.z,
            self.w - other.w,
        )
    }
}

impl std::ops::SubAssign for Vec4 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        self.w -= other.w;
    }
}

impl std::ops::Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self::new(
            self.x * scalar,
            self.y * scalar,
            self.z * scalar,
            self.w * scalar,
        )
    }
}

/// Commutative scalar multiplication: `f32 * Vec4`
impl std::ops::Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, vec: Vec4) -> Vec4 {
        Vec4::new(
            self * vec.x,
            self * vec.y,
            self * vec.z,
            self * vec.w,
        )
    }
}

impl std::ops::MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
        self.w *= scalar;
    }
}

impl std::ops::Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl std::ops::Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self::new(
            self.x / scalar,
            self.y / scalar,
            self.z / scalar,
            self.w / scalar,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn test_dot() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
        assert_eq!(a.dot(b), 70.0);
    }

    #[test]
    fn test_length() {
        let v = Vec4::new(1.0, 0.0, 0.0, 0.0);
        assert_eq!(v.length(), 1.0);

        let v2 = Vec4::new(1.0, 1.0, 1.0, 1.0);
        assert!((v2.length() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_normalized() {
        let v = Vec4::new(3.0, 0.0, 0.0, 0.0);
        let n = v.normalized();
        assert!((n.x - 1.0).abs() < 0.0001);
        assert_eq!(n.y, 0.0);
        assert_eq!(n.z, 0.0);
        assert_eq!(n.w, 0.0);
    }

    #[test]
    fn test_add() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let c = a + b;
        assert_eq!(c.x, 6.0);
        assert_eq!(c.y, 8.0);
        assert_eq!(c.z, 10.0);
        assert_eq!(c.w, 12.0);
    }

    #[test]
    fn test_sub() {
        let a = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let b = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let c = a - b;
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 4.0);
        assert_eq!(c.z, 4.0);
        assert_eq!(c.w, 4.0);
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let scaled = v * 2.0;
        assert_eq!(scaled.x, 2.0);
        assert_eq!(scaled.y, 4.0);
        assert_eq!(scaled.z, 6.0);
        assert_eq!(scaled.w, 8.0);
    }

    #[test]
    fn test_neg() {
        let v = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let neg = -v;
        assert_eq!(neg.x, -1.0);
        assert_eq!(neg.y, 2.0);
        assert_eq!(neg.z, -3.0);
        assert_eq!(neg.w, 4.0);
    }

    #[test]
    fn test_xyz() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let xyz = v.xyz();
        assert_eq!(xyz, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_lerp() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 10.0, 10.0, 10.0);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0);
        assert_eq!(mid.z, 5.0);
        assert_eq!(mid.w, 5.0);
    }

    #[test]
    fn test_clamp_components() {
        let v = Vec4::new(-1.0, 5.0, 2.5, 10.0);
        let min = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let max = Vec4::new(3.0, 3.0, 3.0, 3.0);
        let clamped = v.clamp_components(min, max);
        assert_eq!(clamped.x, 0.0);
        assert_eq!(clamped.y, 3.0);
        assert_eq!(clamped.z, 2.5);
        assert_eq!(clamped.w, 3.0);
    }

    #[test]
    fn test_min_max_components() {
        let a = Vec4::new(1.0, 5.0, 2.0, 8.0);
        let b = Vec4::new(3.0, 2.0, 4.0, 6.0);
        let min = a.min_components(b);
        let max = a.max_components(b);
        assert_eq!(min, Vec4::new(1.0, 2.0, 2.0, 6.0));
        assert_eq!(max, Vec4::new(3.0, 5.0, 4.0, 8.0));
    }

    #[test]
    fn test_abs() {
        let v = Vec4::new(-1.0, 2.0, -3.0, 4.0);
        let abs = v.abs();
        assert_eq!(abs, Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_sign() {
        let v = Vec4::new(-1.0, 2.0, 0.0, -0.5);
        let sign = v.sign();
        assert_eq!(sign.x, -1.0);
        assert_eq!(sign.y, 1.0);
        assert_eq!(sign.z, 1.0); // 0.0 is considered positive
        assert_eq!(sign.w, -1.0);
    }

    #[test]
    fn test_component_mul() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 3.0, 4.0, 5.0);
        let result = a.component_mul(b);
        assert_eq!(result, Vec4::new(2.0, 6.0, 12.0, 20.0));
    }

    #[test]
    fn test_distance() {
        let a = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(4.0, 0.0, 0.0, 0.0);
        assert!((a.distance(b) - 3.0).abs() < 0.0001);

        // Distance to self is zero
        assert!(a.distance(a) < 0.0001);

        // Distance is commutative
        assert!((a.distance(b) - b.distance(a)).abs() < 0.0001);
    }

    #[test]
    fn test_distance_squared() {
        let a = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(4.0, 0.0, 0.0, 0.0);
        assert!((a.distance_squared(b) - 9.0).abs() < 0.0001);

        // Diagonal distance: (1,1,1,1) to (0,0,0,0) = sqrt(4) = 2, squared = 4
        let c = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let d = Vec4::ZERO;
        assert!((c.distance_squared(d) - 4.0).abs() < 0.0001);
    }

    #[test]
    fn test_scalar_mul_commutative() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let a = v * 2.5;
        let b = 2.5 * v;
        assert_eq!(a, b);
    }

    #[test]
    fn test_scalar_mul_lhs() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let scaled = 3.0 * v;
        assert_eq!(scaled.x, 3.0);
        assert_eq!(scaled.y, 6.0);
        assert_eq!(scaled.z, 9.0);
        assert_eq!(scaled.w, 12.0);
    }

    #[test]
    fn test_angle_to_orthogonal() {
        use std::f32::consts::FRAC_PI_2;
        let x = Vec4::X;
        let y = Vec4::Y;
        let angle = x.angle_to(y);
        assert!((angle - FRAC_PI_2).abs() < 0.0001, "Orthogonal vectors should have π/2 angle, got {}", angle);
    }

    #[test]
    fn test_angle_to_same_direction() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let angle = v.angle_to(v);
        assert!(angle.abs() < 0.0001, "Same direction should have 0 angle, got {}", angle);
    }

    #[test]
    fn test_angle_to_opposite() {
        use std::f32::consts::PI;
        let v = Vec4::X;
        let neg_v = -v;
        let angle = v.angle_to(neg_v);
        assert!((angle - PI).abs() < 0.0001, "Opposite vectors should have π angle, got {}", angle);
    }

    #[test]
    fn test_rotate_towards_within_range() {
        let from = Vec4::X;
        let to = Vec4::Y;
        // Angle between X and Y is π/2. If we allow more than that, should reach target
        let result = from.rotate_towards(to, 2.0);
        let dot = result.dot(to);
        assert!(dot > 0.99, "rotate_towards should reach target when max_radians is large enough");
    }

    #[test]
    fn test_rotate_towards_limited() {
        use std::f32::consts::FRAC_PI_4;
        let from = Vec4::X;
        let to = Vec4::Y;
        // Rotate by π/4, should be halfway between X and Y
        let result = from.rotate_towards(to, FRAC_PI_4);
        let angle_from_start = from.angle_to(result);
        assert!((angle_from_start - FRAC_PI_4).abs() < 0.01,
            "Should rotate by max_radians, rotated {} instead of {}", angle_from_start, FRAC_PI_4);
    }

    #[test]
    fn test_rotate_towards_preserves_length() {
        let from = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let to = Vec4::new(0.0, 1.0, 1.0, 1.0).normalized();
        let result = from.rotate_towards(to, 0.5);
        let len = result.length();
        assert!((len - 1.0).abs() < 0.01, "rotate_towards should preserve length, got {}", len);
    }

    #[test]
    fn test_rotate_towards_anti_parallel() {
        // T5: Anti-parallel vectors should produce a valid unit-length result
        use std::f32::consts::FRAC_PI_4;
        let from = Vec4::X;
        let to = -Vec4::X;
        let result = from.rotate_towards(to, FRAC_PI_4);
        let len = result.length();
        assert!((len - 1.0).abs() < 0.01,
            "rotate_towards anti-parallel should produce unit vector, got length {}", len);
        // Should have rotated 45° from X toward -X
        let angle = from.angle_to(result);
        assert!((angle - FRAC_PI_4).abs() < 0.01,
            "Should rotate by max_radians, got {}", angle);
    }

    #[test]
    fn test_rotate_towards_anti_parallel_all_axes() {
        // Verify anti-parallel fallback works for all basis directions
        use std::f32::consts::FRAC_PI_4;
        let axes = [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W];
        for axis in &axes {
            let result = axis.rotate_towards(-*axis, FRAC_PI_4);
            let len = result.length();
            assert!((len - 1.0).abs() < 0.01,
                "Anti-parallel rotate_towards failed for {:?}, length = {}", axis, len);
        }
    }
}
