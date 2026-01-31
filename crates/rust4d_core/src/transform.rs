//! 4D Transform (position, rotation, scale)
//!
//! A Transform4D represents the position, rotation, and scale of an entity in 4D space.

use rust4d_math::{Vec4, Rotor4};
use serde::{Serialize, Deserialize};

/// A 4D transform with position, rotation, and uniform scale
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform4D {
    /// Position in 4D space
    pub position: Vec4,
    /// Rotation as a 4D rotor
    pub rotation: Rotor4,
    /// Uniform scale factor
    pub scale: f32,
}

impl Default for Transform4D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform4D {
    /// Create an identity transform (no translation, rotation, or scale change)
    pub fn identity() -> Self {
        Self {
            position: Vec4::ZERO,
            rotation: Rotor4::IDENTITY,
            scale: 1.0,
        }
    }

    /// Create a transform with just a position
    pub fn from_position(position: Vec4) -> Self {
        Self {
            position,
            rotation: Rotor4::IDENTITY,
            scale: 1.0,
        }
    }

    /// Create a transform with position and rotation
    pub fn from_position_rotation(position: Vec4, rotation: Rotor4) -> Self {
        Self {
            position,
            rotation,
            scale: 1.0,
        }
    }

    /// Get the rotation matrix as a 4x4 array
    ///
    /// This only includes rotation, not position or scale.
    #[inline]
    pub fn rotation_matrix(&self) -> [[f32; 4]; 4] {
        self.rotation.to_matrix()
    }

    /// Transform a point from local space to world space
    ///
    /// Applies scale, then rotation, then translation.
    pub fn transform_point(&self, p: Vec4) -> Vec4 {
        // Scale
        let scaled = p * self.scale;
        // Rotate
        let rotated = self.rotation.rotate(scaled);
        // Translate
        rotated + self.position
    }

    /// Transform a direction from local space to world space
    ///
    /// Applies scale and rotation, but not translation.
    pub fn transform_direction(&self, d: Vec4) -> Vec4 {
        let scaled = d * self.scale;
        self.rotation.rotate(scaled)
    }

    /// Compute the inverse transform
    ///
    /// The inverse transform undoes this transform:
    /// `transform.inverse().transform_point(transform.transform_point(p)) == p`
    pub fn inverse(&self) -> Self {
        let inv_scale = if self.scale.abs() > 1e-10 {
            1.0 / self.scale
        } else {
            1.0
        };
        let inv_rotation = self.rotation.reverse();
        let inv_position = inv_rotation.rotate(-self.position) * inv_scale;

        Self {
            position: inv_position,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }

    /// Compose two transforms: result = self * other
    ///
    /// The composed transform applies `other` first, then `self`.
    pub fn compose(&self, other: &Self) -> Self {
        Self {
            position: self.transform_point(other.position),
            rotation: self.rotation.compose(&other.rotation),
            scale: self.scale * other.scale,
        }
    }

    /// Translate the transform by an offset
    pub fn translate(&mut self, offset: Vec4) {
        self.position += offset;
    }

    /// Rotate the transform by a rotor
    pub fn rotate(&mut self, rotor: Rotor4) {
        self.rotation = rotor.compose(&self.rotation).normalize();
    }

    /// Set uniform scale
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_math::RotationPlane;
    use std::f32::consts::PI;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    #[test]
    fn test_identity_transform() {
        let t = Transform4D::identity();
        let p = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let transformed = t.transform_point(p);
        assert!(vec_approx_eq(p, transformed));
    }

    #[test]
    fn test_translation() {
        let t = Transform4D::from_position(Vec4::new(1.0, 2.0, 3.0, 4.0));
        let p = Vec4::ZERO;
        let transformed = t.transform_point(p);
        assert!(vec_approx_eq(transformed, Vec4::new(1.0, 2.0, 3.0, 4.0)));
    }

    #[test]
    fn test_scale() {
        let mut t = Transform4D::identity();
        t.scale = 2.0;
        let p = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let transformed = t.transform_point(p);
        assert!(vec_approx_eq(transformed, Vec4::new(2.0, 2.0, 2.0, 2.0)));
    }

    #[test]
    fn test_rotation() {
        let rotor = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);
        let t = Transform4D::from_position_rotation(Vec4::ZERO, rotor);
        let p = Vec4::X;
        let transformed = t.transform_point(p);
        assert!(vec_approx_eq(transformed, Vec4::Y), "Expected Y, got {:?}", transformed);
    }

    #[test]
    fn test_transform_order() {
        // Transform applies: scale, then rotate, then translate
        let rotor = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);
        let mut t = Transform4D::identity();
        t.scale = 2.0;
        t.rotation = rotor;
        t.position = Vec4::new(10.0, 0.0, 0.0, 0.0);

        // X * 2 = (2, 0, 0, 0), rotated 90° in XY = (0, 2, 0, 0), + (10, 0, 0, 0) = (10, 2, 0, 0)
        let p = Vec4::X;
        let transformed = t.transform_point(p);
        assert!(vec_approx_eq(transformed, Vec4::new(10.0, 2.0, 0.0, 0.0)),
            "Expected (10, 2, 0, 0), got {:?}", transformed);
    }

    #[test]
    fn test_inverse() {
        let rotor = Rotor4::from_plane_angle(RotationPlane::XZ, 0.5);
        let mut t = Transform4D::from_position_rotation(Vec4::new(1.0, 2.0, 3.0, 4.0), rotor);
        t.scale = 2.0;

        let p = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let transformed = t.transform_point(p);
        let back = t.inverse().transform_point(transformed);

        assert!(vec_approx_eq(p, back), "Expected {:?}, got {:?}", p, back);
    }

    #[test]
    fn test_transform_direction() {
        let t = Transform4D::from_position(Vec4::new(100.0, 100.0, 100.0, 100.0));
        let d = Vec4::X;
        let transformed = t.transform_direction(d);
        // Direction should not be affected by position
        assert!(vec_approx_eq(transformed, Vec4::X));
    }

    #[test]
    fn test_compose() {
        let t1 = Transform4D::from_position(Vec4::new(1.0, 0.0, 0.0, 0.0));
        let t2 = Transform4D::from_position(Vec4::new(0.0, 2.0, 0.0, 0.0));

        // t1.compose(t2) applies t2 first, then t1
        let composed = t1.compose(&t2);

        let p = Vec4::ZERO;
        let result = composed.transform_point(p);
        // Origin -> +y2 (t2) -> +x1 (t1) = (1, 2, 0, 0)
        assert!(vec_approx_eq(result, Vec4::new(1.0, 2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_default() {
        let t = Transform4D::default();
        assert!(vec_approx_eq(t.position, Vec4::ZERO));
        assert_eq!(t.scale, 1.0);
    }
}
