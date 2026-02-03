//! 4D Ray type for raycasting

use crate::Vec4;

/// A ray in 4D space defined by an origin and a normalized direction
#[derive(Clone, Copy, Debug)]
pub struct Ray4D {
    /// The starting point of the ray
    pub origin: Vec4,
    /// The normalized direction of the ray
    pub direction: Vec4,
}

impl Ray4D {
    /// Create a new ray from an origin and direction
    ///
    /// The direction is automatically normalized.
    pub fn new(origin: Vec4, direction: Vec4) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
        }
    }

    /// Get the point along the ray at parameter t
    ///
    /// Returns `origin + direction * t`
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
        let ray = Ray4D::new(Vec4::ZERO, Vec4::new(3.0, 0.0, 0.0, 0.0));
        assert!((ray.direction.length() - 1.0).abs() < 0.0001);
        assert!((ray.direction.x - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_ray_new_diagonal_direction() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert!((ray.direction.length() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_ray_point_at_origin() {
        let ray = Ray4D::new(Vec4::new(1.0, 2.0, 3.0, 4.0), Vec4::X);
        let p = ray.point_at(0.0);
        assert_eq!(p, Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_ray_point_at_positive_t() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::new(1.0, 0.0, 0.0, 0.0));
        let p = ray.point_at(5.0);
        assert!((p.x - 5.0).abs() < 0.0001);
        assert!(p.y.abs() < 0.0001);
    }

    #[test]
    fn test_ray_point_at_negative_t() {
        let ray = Ray4D::new(Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::X);
        let p = ray.point_at(-2.0);
        assert!((p.x - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_ray_preserves_origin() {
        let origin = Vec4::new(5.0, 10.0, -3.0, 7.0);
        let ray = Ray4D::new(origin, Vec4::Y);
        assert_eq!(ray.origin, origin);
    }
}
