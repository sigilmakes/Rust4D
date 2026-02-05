//! 4D Ray type for raycasting

use crate::Vec4;

/// A ray in 4D space defined by an origin point and a normalized direction
#[derive(Clone, Copy, Debug)]
pub struct Ray4D {
    /// The starting point of the ray
    pub origin: Vec4,
    /// The direction of the ray (always normalized)
    pub direction: Vec4,
}

impl Ray4D {
    /// Create a new ray. Direction will be normalized automatically.
    /// Panics (debug_assert) if direction is zero vector.
    pub fn new(origin: Vec4, direction: Vec4) -> Self {
        debug_assert!(
            direction.length_squared() > 0.0,
            "Ray direction cannot be zero"
        );
        Self {
            origin,
            direction: direction.normalized(),
        }
    }

    /// Get the point along the ray at parameter t.
    #[inline]
    pub fn point_at(&self, t: f32) -> Vec4 {
        self.origin + self.direction * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_new_normalizes_direction() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::new(2.0, 0.0, 0.0, 0.0));

        // Direction should be normalized
        assert!((ray.direction.length() - 1.0).abs() < 0.0001);
        assert!((ray.direction.x - 1.0).abs() < 0.0001);
        assert_eq!(ray.direction.y, 0.0);
        assert_eq!(ray.direction.z, 0.0);
        assert_eq!(ray.direction.w, 0.0);
    }

    #[test]
    fn test_ray_new_normalizes_diagonal_direction() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));

        // Direction should be normalized (length = 1)
        assert!((ray.direction.length() - 1.0).abs() < 0.0001);

        // All components should be equal (1/sqrt(4) = 0.5)
        let expected = 0.5;
        assert!((ray.direction.x - expected).abs() < 0.0001);
        assert!((ray.direction.y - expected).abs() < 0.0001);
        assert!((ray.direction.z - expected).abs() < 0.0001);
        assert!((ray.direction.w - expected).abs() < 0.0001);
    }

    #[test]
    fn test_ray_point_at_origin() {
        let ray = Ray4D::new(Vec4::new(1.0, 2.0, 3.0, 4.0), Vec4::X);

        // t=0 should return origin
        let point = ray.point_at(0.0);
        assert_eq!(point, Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_ray_point_at_positive() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);

        // t=5 should be 5 units along X
        let point = ray.point_at(5.0);
        assert_eq!(point, Vec4::new(5.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_ray_point_at_negative() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);

        // t=-3 should be 3 units in the negative X direction
        let point = ray.point_at(-3.0);
        assert_eq!(point, Vec4::new(-3.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_ray_point_at_with_offset_origin() {
        let ray = Ray4D::new(Vec4::new(10.0, 0.0, 0.0, 0.0), Vec4::Y);

        // t=7 should be 7 units along Y from origin
        let point = ray.point_at(7.0);
        assert_eq!(point, Vec4::new(10.0, 7.0, 0.0, 0.0));
    }

    #[test]
    #[should_panic(expected = "Ray direction cannot be zero")]
    #[cfg(debug_assertions)]
    fn test_ray_new_panics_on_zero_direction() {
        let _ray = Ray4D::new(Vec4::ZERO, Vec4::ZERO);
    }
}
