//! 4D Rotor for representing rotations in 4D space
//!
//! In 4D, rotations happen in planes rather than around axes.
//! There are 6 rotation planes: XY, XZ, XW, YZ, YW, ZW.
//!
//! A rotor has 8 components:
//! - 1 scalar
//! - 6 bivectors (one for each plane)
//! - 1 pseudoscalar (4-vector)

use bytemuck::{Pod, Zeroable};
use serde::{Serialize, Deserialize};
use crate::Vec4;

/// The 6 rotation planes in 4D space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationPlane {
    /// XY plane - standard yaw (rotation around Z axis in 3D)
    XY,
    /// XZ plane - standard pitch (rotation around Y axis in 3D)
    XZ,
    /// YZ plane - standard roll (rotation around X axis in 3D)
    YZ,
    /// XW plane - ana-kata rotation affecting X
    XW,
    /// YW plane - ana-kata rotation affecting Y
    YW,
    /// ZW plane - ana-kata rotation affecting Z (W-roll in 4D Golf)
    ZW,
}

/// 4D Rotor for representing rotations
///
/// Rotor = scalar + bivectors + pseudoscalar
/// R = s + b_xy*e12 + b_xz*e13 + b_xw*e14 + b_yz*e23 + b_yw*e24 + b_zw*e34 + p*e1234
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct Rotor4 {
    /// Scalar component
    pub s: f32,
    /// Bivector component for XY plane (e12)
    pub b_xy: f32,
    /// Bivector component for XZ plane (e13)
    pub b_xz: f32,
    /// Bivector component for XW plane (e14)
    pub b_xw: f32,
    /// Bivector component for YZ plane (e23)
    pub b_yz: f32,
    /// Bivector component for YW plane (e24)
    pub b_yw: f32,
    /// Bivector component for ZW plane (e34)
    pub b_zw: f32,
    /// Pseudoscalar component (e1234)
    pub p: f32,
}

impl Default for Rotor4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rotor4 {
    /// Identity rotor (no rotation)
    pub const IDENTITY: Self = Self {
        s: 1.0,
        b_xy: 0.0,
        b_xz: 0.0,
        b_xw: 0.0,
        b_yz: 0.0,
        b_yw: 0.0,
        b_zw: 0.0,
        p: 0.0,
    };

    /// Create a rotor for rotation in a single plane
    ///
    /// For a rotation by angle θ in a plane, the rotor is:
    /// R = cos(θ/2) - sin(θ/2) * B
    /// where B is the unit bivector for that plane
    pub fn from_plane_angle(plane: RotationPlane, angle: f32) -> Self {
        let half = angle * 0.5;
        let cos_h = half.cos();
        let sin_h = half.sin();

        let mut r = Self::IDENTITY;
        r.s = cos_h;

        // The bivector component is -sin(θ/2) for the rotation plane
        match plane {
            RotationPlane::XY => r.b_xy = -sin_h,
            RotationPlane::XZ => r.b_xz = -sin_h,
            RotationPlane::XW => r.b_xw = -sin_h,
            RotationPlane::YZ => r.b_yz = -sin_h,
            RotationPlane::YW => r.b_yw = -sin_h,
            RotationPlane::ZW => r.b_zw = -sin_h,
        }

        r
    }

    /// Create a rotor from Euler angles (XYZ order, intrinsic)
    ///
    /// This creates a 3D rotation compatible with Engine4D's quaternion Euler angles.
    /// Used with `skip_y` to create 4D rotations that preserve the Y axis.
    ///
    /// # Arguments
    /// * `x` - Rotation around X axis (pitch in YZ plane)
    /// * `y` - Rotation around Y axis (yaw in XZ plane)
    /// * `z` - Rotation around Z axis (roll in XY plane)
    ///
    /// # Rotation order
    /// Rotations are applied in XYZ order (X first, then Y, then Z).
    /// This matches Unity's `Quaternion.Euler(x, y, z)`.
    pub fn from_euler_xyz(x: f32, y: f32, z: f32) -> Self {
        // X rotation = YZ plane
        let rx = Self::from_plane_angle(RotationPlane::YZ, x);
        // Y rotation = XZ plane (note: negated for right-hand rule)
        let ry = Self::from_plane_angle(RotationPlane::XZ, -y);
        // Z rotation = XY plane
        let rz = Self::from_plane_angle(RotationPlane::XY, z);

        // Compose in XYZ order: Z * Y * X (Z applied last)
        rz.compose(&ry.compose(&rx))
    }

    /// Create a rotor for rotation in the plane spanned by two vectors
    ///
    /// The rotation is in the plane defined by vectors `a` and `b`.
    /// Positive angle rotates from `a` toward `b`.
    ///
    /// Returns identity if the vectors are parallel (no plane defined).
    pub fn from_plane_vectors(a: Vec4, b: Vec4, angle: f32) -> Self {
        // Compute the bivector components: B = a ∧ b
        // B_ij = a_i * b_j - a_j * b_i
        let b_xy = a.x * b.y - a.y * b.x;
        let b_xz = a.x * b.z - a.z * b.x;
        let b_xw = a.x * b.w - a.w * b.x;
        let b_yz = a.y * b.z - a.z * b.y;
        let b_yw = a.y * b.w - a.w * b.y;
        let b_zw = a.z * b.w - a.w * b.z;

        // Magnitude of the bivector
        let mag_sq = b_xy * b_xy + b_xz * b_xz + b_xw * b_xw
                   + b_yz * b_yz + b_yw * b_yw + b_zw * b_zw;

        if mag_sq < 1e-10 {
            // Vectors are parallel, no rotation plane
            return Self::IDENTITY;
        }

        let mag = mag_sq.sqrt();
        let inv_mag = 1.0 / mag;

        // Normalize the bivector
        let b_xy_n = b_xy * inv_mag;
        let b_xz_n = b_xz * inv_mag;
        let b_xw_n = b_xw * inv_mag;
        let b_yz_n = b_yz * inv_mag;
        let b_yw_n = b_yw * inv_mag;
        let b_zw_n = b_zw * inv_mag;

        // Rotor: R = cos(θ/2) - sin(θ/2) * B_normalized
        let half = angle * 0.5;
        let cos_h = half.cos();
        let sin_h = half.sin();

        Self {
            s: cos_h,
            b_xy: -sin_h * b_xy_n,
            b_xz: -sin_h * b_xz_n,
            b_xw: -sin_h * b_xw_n,
            b_yz: -sin_h * b_yz_n,
            b_yw: -sin_h * b_yw_n,
            b_zw: -sin_h * b_zw_n,
            p: 0.0,
        }
    }

    /// Compute the squared magnitude of the rotor
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.s * self.s
            + self.b_xy * self.b_xy
            + self.b_xz * self.b_xz
            + self.b_xw * self.b_xw
            + self.b_yz * self.b_yz
            + self.b_yw * self.b_yw
            + self.b_zw * self.b_zw
            + self.p * self.p
    }

    /// Compute the magnitude of the rotor
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.magnitude_squared().sqrt()
    }

    /// Normalize the rotor to unit magnitude
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            let inv_mag = 1.0 / mag;
            Self {
                s: self.s * inv_mag,
                b_xy: self.b_xy * inv_mag,
                b_xz: self.b_xz * inv_mag,
                b_xw: self.b_xw * inv_mag,
                b_yz: self.b_yz * inv_mag,
                b_yw: self.b_yw * inv_mag,
                b_zw: self.b_zw * inv_mag,
                p: self.p * inv_mag,
            }
        } else {
            Self::IDENTITY
        }
    }

    /// Compute the reverse (conjugate) of the rotor
    /// For unit rotors, this is the inverse rotation
    /// Reverse negates all bivector components
    pub fn reverse(&self) -> Self {
        Self {
            s: self.s,
            b_xy: -self.b_xy,
            b_xz: -self.b_xz,
            b_xw: -self.b_xw,
            b_yz: -self.b_yz,
            b_yw: -self.b_yw,
            b_zw: -self.b_zw,
            p: self.p, // Pseudoscalar doesn't change sign under reverse
        }
    }

    /// Rotate a 4D vector using the sandwich product: v' = R v R̃
    ///
    /// This computes the sandwich product directly for correctness.
    pub fn rotate(&self, v: Vec4) -> Vec4 {
        // Compute R * v (rotor times vector)
        // A vector v = v.x*e1 + v.y*e2 + v.z*e3 + v.w*e4
        // R = s + b12*e12 + b13*e13 + b14*e14 + b23*e23 + b24*e24 + b34*e34 + p*e1234

        let s = self.s;
        let b12 = self.b_xy;
        let b13 = self.b_xz;
        let b14 = self.b_xw;
        let b23 = self.b_yz;
        let b24 = self.b_yw;
        let b34 = self.b_zw;
        let p = self.p;

        // R * v produces vector and trivector parts
        // Vector part of R*v (coefficients of e1, e2, e3, e4):
        let rv_e1 = s * v.x + b12 * v.y + b13 * v.z + b14 * v.w;
        let rv_e2 = s * v.y - b12 * v.x + b23 * v.z + b24 * v.w;
        let rv_e3 = s * v.z - b13 * v.x - b23 * v.y + b34 * v.w;
        let rv_e4 = s * v.w - b14 * v.x - b24 * v.y - b34 * v.z;

        // Trivector part of R*v (coefficients of e123, e124, e134, e234):
        let rv_e123 = b12 * v.z - b13 * v.y + b23 * v.x + p * v.w;
        let rv_e124 = b12 * v.w - b14 * v.y + b24 * v.x - p * v.z;
        let rv_e134 = b13 * v.w - b14 * v.z + b34 * v.x + p * v.y;
        let rv_e234 = b23 * v.w - b24 * v.z + b34 * v.y - p * v.x;

        // Now compute (R*v) * R̃
        // R̃ = s - b12*e12 - b13*e13 - b14*e14 - b23*e23 - b24*e24 - b34*e34 + p*e1234
        // (bivectors negate, scalar and pseudoscalar stay same)

        // The vector part of (R*v)*R̃:
        // From vector * scalar: rv_ei * s
        // From vector * bivector: various terms
        // From trivector * bivector: various terms
        // From trivector * pseudoscalar: trivector * e1234 = vector

        // e1 coefficient:
        let new_x = rv_e1 * s
            + rv_e2 * b12 + rv_e3 * b13 + rv_e4 * b14  // from e_i * e_1i
            + rv_e123 * b23 + rv_e124 * b24 + rv_e134 * b34  // from e_1jk * e_jk
            - rv_e234 * p;  // from e_234 * e_1234 = -e_1

        // e2 coefficient:
        let new_y = rv_e2 * s
            - rv_e1 * b12 + rv_e3 * b23 + rv_e4 * b24  // from e_i * e_2i
            - rv_e123 * b13 - rv_e124 * b14 + rv_e234 * b34  // from e_2jk * e_jk
            + rv_e134 * p;  // from e_134 * e_1234 = e_2

        // e3 coefficient:
        let new_z = rv_e3 * s
            - rv_e1 * b13 - rv_e2 * b23 + rv_e4 * b34  // from e_i * e_3i
            + rv_e123 * b12 - rv_e134 * b14 - rv_e234 * b24  // from e_3jk * e_jk
            - rv_e124 * p;  // from e_124 * e_1234 = -e_3

        // e4 coefficient:
        let new_w = rv_e4 * s
            - rv_e1 * b14 - rv_e2 * b24 - rv_e3 * b34  // from e_i * e_4i
            + rv_e124 * b12 + rv_e134 * b13 + rv_e234 * b23  // from e_4jk * e_jk
            + rv_e123 * p;  // from e_123 * e_1234 = e_4

        Vec4::new(new_x, new_y, new_z, new_w)
    }

    /// Compose two rotations: result = self * other
    /// The composed rotation applies `other` first, then `self`
    pub fn compose(&self, other: &Self) -> Self {
        // Geometric product of two rotors
        // This is a lengthy computation involving all 8 components

        let a = self;
        let b = other;

        // Scalar part
        let s = a.s * b.s
            - a.b_xy * b.b_xy
            - a.b_xz * b.b_xz
            - a.b_xw * b.b_xw
            - a.b_yz * b.b_yz
            - a.b_yw * b.b_yw
            - a.b_zw * b.b_zw
            + a.p * b.p;

        // XY bivector
        let b_xy = a.s * b.b_xy + a.b_xy * b.s
            - a.b_xz * b.b_yz + a.b_yz * b.b_xz
            - a.b_xw * b.b_yw + a.b_yw * b.b_xw
            - a.b_zw * b.p - a.p * b.b_zw;

        // XZ bivector
        let b_xz = a.s * b.b_xz + a.b_xz * b.s
            + a.b_xy * b.b_yz - a.b_yz * b.b_xy
            - a.b_xw * b.b_zw + a.b_zw * b.b_xw
            + a.b_yw * b.p + a.p * b.b_yw;

        // XW bivector
        let b_xw = a.s * b.b_xw + a.b_xw * b.s
            + a.b_xy * b.b_yw - a.b_yw * b.b_xy
            + a.b_xz * b.b_zw - a.b_zw * b.b_xz
            - a.b_yz * b.p - a.p * b.b_yz;

        // YZ bivector
        let b_yz = a.s * b.b_yz + a.b_yz * b.s
            - a.b_xy * b.b_xz + a.b_xz * b.b_xy
            - a.b_yw * b.b_zw + a.b_zw * b.b_yw
            - a.b_xw * b.p - a.p * b.b_xw;

        // YW bivector
        let b_yw = a.s * b.b_yw + a.b_yw * b.s
            - a.b_xy * b.b_xw + a.b_xw * b.b_xy
            + a.b_yz * b.b_zw - a.b_zw * b.b_yz
            + a.b_xz * b.p + a.p * b.b_xz;

        // ZW bivector
        let b_zw = a.s * b.b_zw + a.b_zw * b.s
            - a.b_xz * b.b_xw + a.b_xw * b.b_xz
            - a.b_yz * b.b_yw + a.b_yw * b.b_yz
            - a.b_xy * b.p - a.p * b.b_xy;

        // Pseudoscalar
        let p = a.s * b.p + a.p * b.s
            + a.b_xy * b.b_zw + a.b_zw * b.b_xy
            - a.b_xz * b.b_yw - a.b_yw * b.b_xz
            + a.b_xw * b.b_yz + a.b_yz * b.b_xw;

        Self {
            s,
            b_xy,
            b_xz,
            b_xw,
            b_yz,
            b_yw,
            b_zw,
            p,
        }
    }

    /// Convert rotor to a 4x4 rotation matrix
    /// Useful for sending to GPU
    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        // We compute the matrix by rotating each basis vector
        let x_col = self.rotate(Vec4::X);
        let y_col = self.rotate(Vec4::Y);
        let z_col = self.rotate(Vec4::Z);
        let w_col = self.rotate(Vec4::W);

        // Column-major order
        [
            [x_col.x, x_col.y, x_col.z, x_col.w],
            [y_col.x, y_col.y, y_col.z, y_col.w],
            [z_col.x, z_col.y, z_col.z, z_col.w],
            [w_col.x, w_col.y, w_col.z, w_col.w],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    #[test]
    fn test_identity_rotation() {
        let r = Rotor4::IDENTITY;
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(v, rotated));
    }

    #[test]
    fn test_xy_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

        // Rotating X by 90° in XY plane should give Y
        let v = Vec4::X;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::Y), "Expected Y, got {:?}", rotated);

        // Rotating Y by 90° in XY plane should give -X
        let v = Vec4::Y;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, -Vec4::X), "Expected -X, got {:?}", rotated);
    }

    #[test]
    fn test_xz_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 2.0);

        // Rotating X by 90° in XZ plane should give Z
        let v = Vec4::X;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::Z), "Expected Z, got {:?}", rotated);
    }

    #[test]
    fn test_zw_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::ZW, PI / 2.0);

        // Rotating Z by 90° in ZW plane should give W
        let v = Vec4::Z;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::W), "Expected W, got {:?}", rotated);
    }

    #[test]
    fn test_rotation_preserves_length() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, 1.23);
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = r.rotate(v);
        assert!(approx_eq(v.length(), rotated.length()));
    }

    #[test]
    fn test_compose_identity() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let identity = Rotor4::IDENTITY;

        let composed = r.compose(&identity);
        assert!(approx_eq(composed.s, r.s));
        assert!(approx_eq(composed.b_xy, r.b_xy));
    }

    #[test]
    fn test_compose_inverse() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 3.0);
        let r_inv = r.reverse();

        let composed = r.compose(&r_inv);
        // Should be close to identity
        assert!(approx_eq(composed.normalize().s, 1.0), "Expected identity, got {:?}", composed);
    }

    #[test]
    fn test_full_rotation() {
        // Two 180° rotations should give identity
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI);
        let composed = r.compose(&r);

        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = composed.normalize().rotate(v);
        assert!(vec_approx_eq(v, rotated), "Expected original, got {:?}", rotated);
    }

    #[test]
    fn test_normalize() {
        let mut r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        // Artificially scale
        r.s *= 2.0;
        r.b_xy *= 2.0;

        let normalized = r.normalize();
        assert!(approx_eq(normalized.magnitude(), 1.0));
    }

    #[test]
    fn test_to_matrix_identity() {
        let r = Rotor4::IDENTITY;
        let m = r.to_matrix();

        // Should be identity matrix
        assert!(approx_eq(m[0][0], 1.0) && approx_eq(m[0][1], 0.0) && approx_eq(m[0][2], 0.0) && approx_eq(m[0][3], 0.0));
        assert!(approx_eq(m[1][0], 0.0) && approx_eq(m[1][1], 1.0) && approx_eq(m[1][2], 0.0) && approx_eq(m[1][3], 0.0));
        assert!(approx_eq(m[2][0], 0.0) && approx_eq(m[2][1], 0.0) && approx_eq(m[2][2], 1.0) && approx_eq(m[2][3], 0.0));
        assert!(approx_eq(m[3][0], 0.0) && approx_eq(m[3][1], 0.0) && approx_eq(m[3][2], 0.0) && approx_eq(m[3][3], 1.0));
    }

    #[test]
    fn test_yz_rotation_90() {
        // YZ rotation (pitch) - X axis unchanged
        let r = Rotor4::from_plane_angle(RotationPlane::YZ, PI / 2.0);

        // X should be unchanged
        let rotated_x = r.rotate(Vec4::X);
        assert!(vec_approx_eq(rotated_x, Vec4::X), "X should be unchanged, got {:?}", rotated_x);

        // Y should go to Z
        let rotated_y = r.rotate(Vec4::Y);
        assert!(vec_approx_eq(rotated_y, Vec4::Z), "Y should become Z, got {:?}", rotated_y);

        // Z should go to -Y
        let rotated_z = r.rotate(Vec4::Z);
        assert!(vec_approx_eq(rotated_z, -Vec4::Y), "Z should become -Y, got {:?}", rotated_z);
    }

    #[test]
    fn test_composed_rotation_orthogonality() {
        // Compose XZ (yaw) and YZ (pitch) rotations
        let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 4.0);
        let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, PI / 6.0);
        let composed = r_pitch.compose(&r_yaw).normalize();

        // Rotate basis vectors and verify orthonormality
        let x = composed.rotate(Vec4::X);
        let y = composed.rotate(Vec4::Y);
        let z = composed.rotate(Vec4::Z);
        let w = composed.rotate(Vec4::W);

        // Check lengths are preserved
        assert!(approx_eq(x.length(), 1.0), "X length not preserved: {}", x.length());
        assert!(approx_eq(y.length(), 1.0), "Y length not preserved: {}", y.length());
        assert!(approx_eq(z.length(), 1.0), "Z length not preserved: {}", z.length());
        assert!(approx_eq(w.length(), 1.0), "W length not preserved: {}", w.length());

        // Check orthogonality (dot products should be 0)
        assert!(approx_eq(x.dot(y), 0.0), "X.Y not orthogonal: {}", x.dot(y));
        assert!(approx_eq(x.dot(z), 0.0), "X.Z not orthogonal: {}", x.dot(z));
        assert!(approx_eq(x.dot(w), 0.0), "X.W not orthogonal: {}", x.dot(w));
        assert!(approx_eq(y.dot(z), 0.0), "Y.Z not orthogonal: {}", y.dot(z));
        assert!(approx_eq(y.dot(w), 0.0), "Y.W not orthogonal: {}", y.dot(w));
        assert!(approx_eq(z.dot(w), 0.0), "Z.W not orthogonal: {}", z.dot(w));
    }

    #[test]
    fn test_multiple_rotation_composition() {
        // Test composing all 4 rotation planes used in camera
        let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, 0.5);
        let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, 0.3);
        let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, 0.2);
        let r_roll_xw = Rotor4::from_plane_angle(RotationPlane::XW, 0.1);

        let composed = r_roll_xw.compose(&r_roll_w.compose(&r_pitch.compose(&r_yaw))).normalize();

        // Verify it's still a unit rotor
        assert!(approx_eq(composed.magnitude(), 1.0), "Composed rotor not unit: {}", composed.magnitude());

        // Verify rotation preserves lengths (use slightly larger epsilon for accumulated error)
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = composed.rotate(v);
        let length_error = (v.length() - rotated.length()).abs();
        assert!(length_error < 0.001,
            "Length not preserved: {} vs {} (error: {})", v.length(), rotated.length(), length_error);
    }

    #[test]
    fn test_to_matrix_matches_rotate() {
        // Verify that to_matrix produces the same results as rotate()
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 3.0)
            .compose(&Rotor4::from_plane_angle(RotationPlane::YZ, PI / 4.0))
            .normalize();

        let m = r.to_matrix();
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);

        // Rotate using rotor
        let rotated_rotor = r.rotate(v);

        // Rotate using matrix (column-major: result = M * v)
        let rotated_matrix = Vec4::new(
            m[0][0] * v.x + m[1][0] * v.y + m[2][0] * v.z + m[3][0] * v.w,
            m[0][1] * v.x + m[1][1] * v.y + m[2][1] * v.z + m[3][1] * v.w,
            m[0][2] * v.x + m[1][2] * v.y + m[2][2] * v.z + m[3][2] * v.w,
            m[0][3] * v.x + m[1][3] * v.y + m[2][3] * v.z + m[3][3] * v.w,
        );

        assert!(vec_approx_eq(rotated_rotor, rotated_matrix),
            "Rotor and matrix give different results: {:?} vs {:?}", rotated_rotor, rotated_matrix);
    }

    #[test]
    fn test_rotation_matrix_is_orthogonal() {
        // An orthogonal matrix has M * M^T = I
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, 0.7)
            .compose(&Rotor4::from_plane_angle(RotationPlane::ZW, 0.4))
            .normalize();

        let m = r.to_matrix();

        // Check that column vectors are orthonormal
        for i in 0..4 {
            let col_i = Vec4::new(m[i][0], m[i][1], m[i][2], m[i][3]);
            assert!(approx_eq(col_i.length(), 1.0), "Column {} not unit length", i);

            for j in (i+1)..4 {
                let col_j = Vec4::new(m[j][0], m[j][1], m[j][2], m[j][3]);
                let dot = col_i.dot(col_j);
                assert!(approx_eq(dot, 0.0), "Columns {} and {} not orthogonal: dot = {}", i, j, dot);
            }
        }
    }

    #[test]
    fn test_same_plane_composition() {
        // Composing two rotations in the same plane should add angles
        let r1 = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let r2 = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let composed = r1.compose(&r2);

        // Should be equivalent to 90° rotation
        let expected = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

        let v = Vec4::X;
        let rotated_composed = composed.rotate(v);
        let rotated_expected = expected.rotate(v);

        assert!(vec_approx_eq(rotated_composed, rotated_expected),
            "Same-plane composition failed: {:?} vs {:?}", rotated_composed, rotated_expected);
    }

    #[test]
    fn test_simple_composed_rotation() {
        // Test a simple case: XZ then YZ rotation
        // Apply rotations sequentially to a vector and compare
        let r_xz = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 2.0);
        let r_yz = Rotor4::from_plane_angle(RotationPlane::YZ, PI / 2.0);

        // Sequential: first XZ, then YZ
        let v = Vec4::X;
        let step1 = r_xz.rotate(v);
        let step2 = r_yz.rotate(step1);

        // Composed: YZ.compose(XZ) applies XZ first, then YZ
        let composed = r_yz.compose(&r_xz);
        let result = composed.rotate(v);

        assert!(vec_approx_eq(step2, result),
            "Sequential {:?} vs composed {:?}", step2, result);
    }

    #[test]
    fn test_rotor_components_after_compose() {
        // Compose XZ and YZ rotations and check the resulting rotor
        let r_xz = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 2.0);
        let r_yz = Rotor4::from_plane_angle(RotationPlane::YZ, PI / 2.0);
        let composed = r_yz.compose(&r_xz);

        println!("r_xz: s={}, b_xz={}", r_xz.s, r_xz.b_xz);
        println!("r_yz: s={}, b_yz={}", r_yz.s, r_yz.b_yz);
        println!("composed: s={}, b_xy={}, b_xz={}, b_yz={}, p={}",
            composed.s, composed.b_xy, composed.b_xz, composed.b_yz, composed.p);
        println!("composed magnitude: {}", composed.magnitude());

        // The composed rotor should be a unit rotor
        assert!(approx_eq(composed.magnitude(), 1.0),
            "Composed rotor not unit: {}", composed.magnitude());
    }

    #[test]
    fn test_debug_rotation_formula() {
        // Manual verification of the rotation formula
        // Composed rotor: s=0.5, b_xy=0.5, b_xz=-0.5, b_yz=-0.5
        // Expected result from GA: R*e1*R̃ = -e2 = (0, -1, 0, 0)

        let r_xz = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 2.0);
        let r_yz = Rotor4::from_plane_angle(RotationPlane::YZ, PI / 2.0);
        let composed = r_yz.compose(&r_xz);

        // Print the rotation matrix
        let m = composed.to_matrix();
        println!("Rotation matrix for composed rotor:");
        println!("[{:6.3} {:6.3} {:6.3} {:6.3}]", m[0][0], m[0][1], m[0][2], m[0][3]);
        println!("[{:6.3} {:6.3} {:6.3} {:6.3}]", m[1][0], m[1][1], m[1][2], m[1][3]);
        println!("[{:6.3} {:6.3} {:6.3} {:6.3}]", m[2][0], m[2][1], m[2][2], m[2][3]);
        println!("[{:6.3} {:6.3} {:6.3} {:6.3}]", m[3][0], m[3][1], m[3][2], m[3][3]);

        // The correct matrix should have:
        // Column 0 (what X maps to): (0, -1, 0, 0) based on GA calculation
        // Let's verify with sequential:
        let x_via_seq = r_yz.rotate(r_xz.rotate(Vec4::X));
        println!("X via sequential: {:?}", x_via_seq);

        let x_via_composed = composed.rotate(Vec4::X);
        println!("X via composed: {:?}", x_via_composed);

        // The first column of the matrix tells us where X goes
        println!("Matrix column 0: ({}, {}, {}, {})", m[0][0], m[1][0], m[2][0], m[3][0]);
    }
}
