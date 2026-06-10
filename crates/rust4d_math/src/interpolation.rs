//! Interpolation traits and utilities for 4D math types
//!
//! This module provides the [`Interpolatable`] trait for types that support
//! linear interpolation (lerp), which is foundational for animation and tweening.

use crate::{Rotor4, Vec4};

/// Trait for types that can be linearly interpolated
///
/// Linear interpolation smoothly blends between two values based on a
/// parameter `t` where `t=0.0` returns `a` and `t=1.0` returns `b`.
///
/// # Naming Convention
///
/// This trait uses a **static method** signature: `Interpolatable::lerp(&a, &b, t)`.
/// This differs from the **instance method** signature used by [`Vec4::lerp`]:
/// `a.lerp(b, t)`.
///
/// The static signature was chosen because:
/// - It works uniformly for all types (including primitives like `f32`)
/// - It matches the mathematical notation `lerp(a, b, t)`
/// - It avoids ambiguity about which value is "self" vs the target
///
/// When using `Vec4` specifically, both APIs are available:
/// ```ignore
/// // Instance method (Vec4-specific)
/// let result = a.lerp(b, t);
///
/// // Static method (generic Interpolatable)
/// let result = <Vec4 as Interpolatable>::lerp(&a, &b, t);
/// let result = Interpolatable::lerp(&a, &b, t);  // with type inference
/// ```
///
/// The `Interpolatable` implementation for `Vec4` delegates to `Vec4::lerp` internally,
/// so both produce identical results.
pub trait Interpolatable: Clone {
    /// Linear interpolation from `a` to `b` at parameter `t`
    ///
    /// # Arguments
    /// * `a` - The starting value (returned when t=0.0)
    /// * `b` - The ending value (returned when t=1.0)
    /// * `t` - Interpolation parameter, typically in range [0.0, 1.0]
    ///
    /// # Returns
    /// The interpolated value. When t is outside [0.0, 1.0], the result
    /// is extrapolated beyond the input values.
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    #[inline]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl Interpolatable for f64 {
    #[inline]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * (t as f64)
    }
}

impl Interpolatable for Vec4 {
    #[inline]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // Use Vec4's built-in lerp for efficiency
        a.lerp(*b, t)
    }
}

impl Interpolatable for Rotor4 {
    /// Spherical linear interpolation (slerp) for rotors
    ///
    /// This interpolates between two rotations along the shortest path
    /// on the rotation hypersphere, maintaining constant angular velocity.
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // Compute dot product of rotor components (measures how similar they are)
        let mut dot = a.s * b.s
            + a.b_xy * b.b_xy
            + a.b_xz * b.b_xz
            + a.b_xw * b.b_xw
            + a.b_yz * b.b_yz
            + a.b_yw * b.b_yw
            + a.b_zw * b.b_zw
            + a.p * b.p;

        // If dot is negative, negate one rotor to take the shorter path
        let (b_s, b_xy, b_xz, b_xw, b_yz, b_yw, b_zw, b_p) = if dot < 0.0 {
            dot = -dot;
            (
                -b.s, -b.b_xy, -b.b_xz, -b.b_xw, -b.b_yz, -b.b_yw, -b.b_zw, -b.p,
            )
        } else {
            (b.s, b.b_xy, b.b_xz, b.b_xw, b.b_yz, b.b_yw, b.b_zw, b.p)
        };

        // If rotors are very close, use linear interpolation to avoid division by zero
        const THRESHOLD: f32 = 0.9995;
        if dot > THRESHOLD {
            // Linear interpolation and normalize
            let result = Rotor4 {
                s: a.s + t * (b_s - a.s),
                b_xy: a.b_xy + t * (b_xy - a.b_xy),
                b_xz: a.b_xz + t * (b_xz - a.b_xz),
                b_xw: a.b_xw + t * (b_xw - a.b_xw),
                b_yz: a.b_yz + t * (b_yz - a.b_yz),
                b_yw: a.b_yw + t * (b_yw - a.b_yw),
                b_zw: a.b_zw + t * (b_zw - a.b_zw),
                p: a.p + t * (b_p - a.p),
            };
            return result.normalize();
        }

        // Standard slerp formula
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let sin_t_theta = (t * theta).sin();
        let sin_1mt_theta = ((1.0 - t) * theta).sin();

        let scale_a = sin_1mt_theta / sin_theta;
        let scale_b = sin_t_theta / sin_theta;

        // Normalize the result to prevent floating-point error accumulation over many slerp calls.
        // While a single slerp theoretically produces a unit rotor, repeated interpolations
        // (e.g., in animation chains or blending) can cause gradual denormalization.
        Rotor4 {
            s: scale_a * a.s + scale_b * b_s,
            b_xy: scale_a * a.b_xy + scale_b * b_xy,
            b_xz: scale_a * a.b_xz + scale_b * b_xz,
            b_xw: scale_a * a.b_xw + scale_b * b_xw,
            b_yz: scale_a * a.b_yz + scale_b * b_yz,
            b_yw: scale_a * a.b_yw + scale_b * b_yw,
            b_zw: scale_a * a.b_zw + scale_b * b_zw,
            p: scale_a * a.p + scale_b * b_p,
        }
        .normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RotationPlane;
    use std::f32::consts::PI;

    const EPSILON: f32 = 0.001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    #[test]
    fn test_f32_lerp() {
        assert!(approx_eq(f32::lerp(&0.0, &10.0, 0.0), 0.0));
        assert!(approx_eq(f32::lerp(&0.0, &10.0, 0.5), 5.0));
        assert!(approx_eq(f32::lerp(&0.0, &10.0, 1.0), 10.0));
    }

    #[test]
    fn test_f32_lerp_extrapolation() {
        // Beyond bounds should extrapolate
        assert!(approx_eq(f32::lerp(&0.0, &10.0, -0.5), -5.0));
        assert!(approx_eq(f32::lerp(&0.0, &10.0, 1.5), 15.0));
    }

    #[test]
    fn test_vec4_lerp_boundaries() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 20.0, 30.0, 40.0);

        let at_zero = <Vec4 as Interpolatable>::lerp(&a, &b, 0.0);
        assert!(vec_approx_eq(at_zero, a));

        let at_one = <Vec4 as Interpolatable>::lerp(&a, &b, 1.0);
        assert!(vec_approx_eq(at_one, b));
    }

    #[test]
    fn test_vec4_lerp_midpoint() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 20.0, 30.0, 40.0);

        let mid = <Vec4 as Interpolatable>::lerp(&a, &b, 0.5);
        assert!(approx_eq(mid.x, 5.0));
        assert!(approx_eq(mid.y, 10.0));
        assert!(approx_eq(mid.z, 15.0));
        assert!(approx_eq(mid.w, 20.0));
    }

    #[test]
    fn test_rotor4_lerp_identity() {
        let a = Rotor4::IDENTITY;
        let b = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

        // At t=0 should be close to identity
        let at_zero = Rotor4::lerp(&a, &b, 0.0);
        assert!(approx_eq(at_zero.s, a.s), "s: {} vs {}", at_zero.s, a.s);

        // At t=1 should be close to b
        let at_one = Rotor4::lerp(&a, &b, 1.0);
        let v = Vec4::X;
        let expected = b.rotate(v);
        let actual = at_one.rotate(v);
        assert!(
            vec_approx_eq(actual, expected),
            "expected {:?}, got {:?}",
            expected,
            actual
        );
    }

    #[test]
    fn test_rotor4_lerp_halfway() {
        let a = Rotor4::IDENTITY;
        let b = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

        // Halfway should give 45 degree rotation
        let mid = Rotor4::lerp(&a, &b, 0.5);
        let v = Vec4::X;
        let rotated = mid.rotate(v);

        // 45 degrees in XY should put X at (cos45, sin45, 0, 0)
        let expected_x = (PI / 4.0).cos();
        let expected_y = (PI / 4.0).sin();
        assert!(
            approx_eq(rotated.x, expected_x),
            "x: {} vs {}",
            rotated.x,
            expected_x
        );
        assert!(
            approx_eq(rotated.y, expected_y),
            "y: {} vs {}",
            rotated.y,
            expected_y
        );
    }

    #[test]
    fn test_rotor4_lerp_short_path() {
        // Test that slerp takes the short path
        // Rotation of -90 degrees should interpolate through 0, not through 270
        let a = Rotor4::IDENTITY;
        let b = Rotor4::from_plane_angle(RotationPlane::XY, -PI / 2.0);

        let mid = Rotor4::lerp(&a, &b, 0.5);
        let v = Vec4::X;
        let rotated = mid.rotate(v);

        // Should be at -45 degrees, not 135 degrees
        let expected_x = (PI / 4.0).cos();
        let expected_y = -(PI / 4.0).sin();
        assert!(
            approx_eq(rotated.x, expected_x),
            "x: {} vs {}",
            rotated.x,
            expected_x
        );
        assert!(
            approx_eq(rotated.y, expected_y),
            "y: {} vs {}",
            rotated.y,
            expected_y
        );
    }

    #[test]
    fn test_rotor4_lerp_preserves_unit() {
        let a = Rotor4::from_plane_angle(RotationPlane::XZ, 0.3);
        let b = Rotor4::from_plane_angle(RotationPlane::YW, 0.7);

        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let result = Rotor4::lerp(&a, &b, t);
            let mag = result.magnitude();
            assert!(approx_eq(mag, 1.0), "magnitude at t={}: {}", t, mag);
        }
    }
}
