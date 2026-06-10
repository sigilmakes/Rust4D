//! Collision shapes for 4D physics
//!
//! These are lightweight primitives used for collision detection,
//! separate from the renderable shapes in rust4d_math.

use rust4d_math::Vec4;

/// A 4D sphere defined by center and radius
#[derive(Clone, Copy, Debug)]
pub struct Sphere4D {
    pub center: Vec4,
    pub radius: f32,
}

impl Sphere4D {
    /// Create a new sphere at the given center with the given radius
    pub fn new(center: Vec4, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Create a unit sphere at the origin
    pub fn unit() -> Self {
        Self::new(Vec4::ZERO, 1.0)
    }

    /// Check if a point is inside or on the sphere
    pub fn contains(&self, point: Vec4) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Get the closest point on the sphere surface to a given point
    pub fn closest_point(&self, point: Vec4) -> Vec4 {
        let direction = (point - self.center).normalized();
        self.center + direction * self.radius
    }
}

/// A 4D axis-aligned bounding box
#[derive(Clone, Copy, Debug)]
pub struct AABB4D {
    /// Minimum corner (all components are minimums)
    pub min: Vec4,
    /// Maximum corner (all components are maximums)
    pub max: Vec4,
}

impl AABB4D {
    /// Create a new AABB from min and max corners
    pub fn new(min: Vec4, max: Vec4) -> Self {
        Self { min, max }
    }

    /// Create an AABB centered at a position with given half-extents
    pub fn from_center_half_extents(center: Vec4, half_extents: Vec4) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Create a unit cube centered at the origin
    pub fn unit() -> Self {
        Self::from_center_half_extents(Vec4::ZERO, Vec4::new(0.5, 0.5, 0.5, 0.5))
    }

    /// Get the center of the AABB
    pub fn center(&self) -> Vec4 {
        (self.min + self.max) * 0.5
    }

    /// Get the half-extents (half the size in each dimension)
    pub fn half_extents(&self) -> Vec4 {
        (self.max - self.min) * 0.5
    }

    /// Get the full size in each dimension
    pub fn size(&self) -> Vec4 {
        self.max - self.min
    }

    /// Check if a point is inside or on the AABB
    pub fn contains(&self, point: Vec4) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
            && point.w >= self.min.w
            && point.w <= self.max.w
    }

    /// Get the closest point inside or on the AABB to a given point
    pub fn closest_point(&self, point: Vec4) -> Vec4 {
        point.clamp_components(self.min, self.max)
    }

    /// Translate the AABB by a delta
    pub fn translated(&self, delta: Vec4) -> Self {
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }
}

/// A 4D infinite plane defined by normal and distance from origin
///
/// The plane equation is: normal · point = distance
/// Points with normal · point > distance are "above" the plane (positive side)
#[derive(Clone, Copy, Debug)]
pub struct Plane4D {
    /// Unit normal vector pointing to the positive side
    pub normal: Vec4,
    /// Signed distance from origin along the normal
    pub distance: f32,
}

impl Plane4D {
    /// Create a new plane from a normal and distance
    ///
    /// The normal will be normalized automatically.
    pub fn new(normal: Vec4, distance: f32) -> Self {
        let n = normal.normalized();
        Self {
            normal: n,
            distance,
        }
    }

    /// Create a plane from a point on the plane and a normal
    pub fn from_point_normal(point: Vec4, normal: Vec4) -> Self {
        let n = normal.normalized();
        let d = n.dot(point);
        Self {
            normal: n,
            distance: d,
        }
    }

    /// Create a horizontal floor plane at the given Y height
    pub fn floor(y: f32) -> Self {
        Self::from_point_normal(Vec4::new(0.0, y, 0.0, 0.0), Vec4::Y)
    }

    /// Calculate the signed distance from a point to the plane
    ///
    /// Positive = above plane (on normal side)
    /// Negative = below plane (opposite side)
    /// Zero = on plane
    pub fn signed_distance(&self, point: Vec4) -> f32 {
        self.normal.dot(point) - self.distance
    }

    /// Project a point onto the plane
    pub fn project_point(&self, point: Vec4) -> Vec4 {
        point - self.normal * self.signed_distance(point)
    }

    /// Check if a point is on the positive side of the plane
    pub fn is_above(&self, point: Vec4) -> bool {
        self.signed_distance(point) > 0.0
    }
}

/// Collider enum for storing different collision shape types
#[derive(Clone, Copy, Debug)]
pub enum Collider {
    Sphere(Sphere4D),
    AABB(AABB4D),
    Plane(Plane4D),
}

impl Collider {
    /// Get the center of the collider
    ///
    /// For planes, returns a point on the plane at the origin offset.
    pub fn center(&self) -> Vec4 {
        match self {
            Collider::Sphere(s) => s.center,
            Collider::AABB(b) => b.center(),
            Collider::Plane(p) => p.normal * p.distance,
        }
    }

    /// Translate the collider by a delta
    ///
    /// For planes, this adjusts the distance from origin.
    pub fn translated(&self, delta: Vec4) -> Self {
        match self {
            Collider::Sphere(s) => Collider::Sphere(Sphere4D::new(s.center + delta, s.radius)),
            Collider::AABB(b) => Collider::AABB(b.translated(delta)),
            Collider::Plane(p) => {
                // Moving a plane by delta means the distance changes by normal · delta
                let new_distance = p.distance + p.normal.dot(delta);
                Collider::Plane(Plane4D::new(p.normal, new_distance))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_contains() {
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);
        assert!(sphere.contains(Vec4::ZERO));
        assert!(sphere.contains(Vec4::new(0.5, 0.0, 0.0, 0.0)));
        assert!(sphere.contains(Vec4::new(1.0, 0.0, 0.0, 0.0))); // on surface
        assert!(!sphere.contains(Vec4::new(1.1, 0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_from_center_half_extents() {
        let aabb = AABB4D::from_center_half_extents(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(0.5, 0.5, 0.5, 0.5),
        );
        assert_eq!(aabb.min, Vec4::new(0.5, 1.5, 2.5, 3.5));
        assert_eq!(aabb.max, Vec4::new(1.5, 2.5, 3.5, 4.5));
        assert_eq!(aabb.center(), Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_aabb_contains() {
        let aabb = AABB4D::new(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert!(aabb.contains(Vec4::new(0.5, 0.5, 0.5, 0.5)));
        assert!(aabb.contains(Vec4::ZERO)); // corner
        assert!(!aabb.contains(Vec4::new(-0.1, 0.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_closest_point() {
        let aabb = AABB4D::new(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));

        // Point inside
        let inside = Vec4::new(0.5, 0.5, 0.5, 0.5);
        assert_eq!(aabb.closest_point(inside), inside);

        // Point outside
        let outside = Vec4::new(2.0, 0.5, 0.5, 0.5);
        assert_eq!(aabb.closest_point(outside), Vec4::new(1.0, 0.5, 0.5, 0.5));
    }

    #[test]
    fn test_plane_signed_distance() {
        let floor = Plane4D::floor(0.0);

        assert!((floor.signed_distance(Vec4::ZERO)).abs() < 0.0001);
        assert!((floor.signed_distance(Vec4::new(0.0, 1.0, 0.0, 0.0)) - 1.0).abs() < 0.0001);
        assert!((floor.signed_distance(Vec4::new(0.0, -1.0, 0.0, 0.0)) + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_plane_project_point() {
        let floor = Plane4D::floor(0.0);
        let point = Vec4::new(3.0, 5.0, 7.0, 2.0);
        let projected = floor.project_point(point);

        assert_eq!(projected.x, 3.0);
        assert!((projected.y).abs() < 0.0001);
        assert_eq!(projected.z, 7.0);
        assert_eq!(projected.w, 2.0);
    }

    #[test]
    fn test_plane_is_above() {
        let floor = Plane4D::floor(0.0);
        assert!(floor.is_above(Vec4::new(0.0, 1.0, 0.0, 0.0)));
        assert!(!floor.is_above(Vec4::new(0.0, -1.0, 0.0, 0.0)));
    }
}
