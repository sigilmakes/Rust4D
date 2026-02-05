//! Raycasting for 4D collision shapes
//!
//! Provides ray-shape intersection tests for spheres, AABBs, and planes.

use crate::shapes::{Collider, Plane4D, Sphere4D, AABB4D};
use rust4d_math::{Ray4D, Vec4};

/// Information about a ray hit
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// Distance along the ray to the hit point
    pub distance: f32,
    /// The point where the ray hit the surface
    pub point: Vec4,
    /// The surface normal at the hit point (pointing toward ray origin)
    pub normal: Vec4,
}

impl RayHit {
    /// Create a new ray hit
    pub fn new(distance: f32, point: Vec4, normal: Vec4) -> Self {
        Self {
            distance,
            point,
            normal,
        }
    }
}

/// Test ray vs sphere intersection
///
/// Uses the standard quadratic formula to solve |O + tD - C|^2 = r^2
/// Returns the nearest hit in front of the ray (t >= 0).
pub fn ray_vs_sphere(ray: &Ray4D, sphere: &Sphere4D) -> Option<RayHit> {
    // Vector from sphere center to ray origin
    let oc = ray.origin - sphere.center;

    // Quadratic coefficients: at^2 + bt + c = 0
    // where a = |D|^2 = 1 (direction is normalized)
    // b = 2 * (D . OC)
    // c = |OC|^2 - r^2
    let a = 1.0; // Direction is normalized
    let half_b = oc.dot(ray.direction); // Using half_b optimization
    let c = oc.length_squared() - sphere.radius * sphere.radius;

    // Discriminant: b^2 - 4ac, but we use (b/2)^2 - ac for optimization
    let discriminant = half_b * half_b - a * c;

    if discriminant < 0.0 {
        // No intersection
        return None;
    }

    let sqrt_d = discriminant.sqrt();

    // Find the nearest root that is in the acceptable range (t >= 0)
    let t1 = (-half_b - sqrt_d) / a;
    let t2 = (-half_b + sqrt_d) / a;

    // Choose the smallest non-negative t
    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        // Both roots are negative, ray points away from sphere
        return None;
    };

    let point = ray.point_at(t);
    let normal = (point - sphere.center).normalized();

    Some(RayHit::new(t, point, normal))
}

/// Test ray vs AABB intersection using the slab method
///
/// The slab method checks intersection with infinite slabs for each axis,
/// tracking the latest entry (t_min) and earliest exit (t_max).
pub fn ray_vs_aabb(ray: &Ray4D, aabb: &AABB4D) -> Option<RayHit> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    let mut hit_axis = 0usize;
    let mut hit_from_min = true;

    // Check each axis (X=0, Y=1, Z=2, W=3)
    let origin = [ray.origin.x, ray.origin.y, ray.origin.z, ray.origin.w];
    let direction = [
        ray.direction.x,
        ray.direction.y,
        ray.direction.z,
        ray.direction.w,
    ];
    let min_bounds = [aabb.min.x, aabb.min.y, aabb.min.z, aabb.min.w];
    let max_bounds = [aabb.max.x, aabb.max.y, aabb.max.z, aabb.max.w];

    for axis in 0..4 {
        let inv_d = if direction[axis].abs() > 1e-8 {
            1.0 / direction[axis]
        } else {
            // Ray is parallel to this axis's slabs
            if origin[axis] < min_bounds[axis] || origin[axis] > max_bounds[axis] {
                // Origin is outside the slab, no intersection possible
                return None;
            }
            // Origin is inside the slab, this axis doesn't constrain us
            continue;
        };

        let t1 = (min_bounds[axis] - origin[axis]) * inv_d;
        let t2 = (max_bounds[axis] - origin[axis]) * inv_d;

        let (t_near, t_far, from_min) = if t1 < t2 {
            (t1, t2, true)
        } else {
            (t2, t1, false)
        };

        // Update the latest entry
        if t_near > t_min {
            t_min = t_near;
            hit_axis = axis;
            hit_from_min = from_min;
        }

        // Update the earliest exit
        if t_far < t_max {
            t_max = t_far;
        }

        // Check for no intersection
        if t_min > t_max {
            return None;
        }
    }

    // Check if intersection is behind the ray
    if t_max < 0.0 {
        return None;
    }

    // Use t_min if it's positive (hitting from outside),
    // otherwise use t_max (origin is inside AABB)
    let t = if t_min >= 0.0 { t_min } else { t_max };

    // Calculate normal based on which face we hit
    let normal = if t_min >= 0.0 {
        // Entering the AABB
        let mut n = Vec4::ZERO;
        match hit_axis {
            0 => n.x = if hit_from_min { -1.0 } else { 1.0 },
            1 => n.y = if hit_from_min { -1.0 } else { 1.0 },
            2 => n.z = if hit_from_min { -1.0 } else { 1.0 },
            3 => n.w = if hit_from_min { -1.0 } else { 1.0 },
            _ => unreachable!(),
        }
        n
    } else {
        // Origin is inside the AABB, we're exiting
        // Find the axis with smallest distance to boundary
        let point = ray.point_at(t);
        let to_min = point - aabb.min;
        let to_max = aabb.max - point;

        // Distance to each face (min face, max face) for each axis
        let distances = [
            to_min.x, to_max.x,
            to_min.y, to_max.y,
            to_min.z, to_max.z,
            to_min.w, to_max.w,
        ];
        // Corresponding outward normals
        let normals = [
            -Vec4::X, Vec4::X,
            -Vec4::Y, Vec4::Y,
            -Vec4::Z, Vec4::Z,
            -Vec4::W, Vec4::W,
        ];

        // Find the face with minimum distance (closest exit)
        let (min_idx, _) = distances
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        normals[min_idx]
    };

    let point = ray.point_at(t);
    Some(RayHit::new(t, point, normal))
}

/// Test ray vs plane intersection
///
/// Returns the hit if the ray intersects the plane in front of the origin.
pub fn ray_vs_plane(ray: &Ray4D, plane: &Plane4D) -> Option<RayHit> {
    let denom = ray.direction.dot(plane.normal);

    // Check if ray is parallel to plane
    if denom.abs() < 1e-8 {
        return None;
    }

    // Calculate t for intersection
    // plane equation: normal . point = distance
    // ray equation: point = origin + t * direction
    // Substituting: normal . (origin + t * direction) = distance
    // t = (distance - normal . origin) / (normal . direction)
    let t = (plane.distance - ray.origin.dot(plane.normal)) / denom;

    // Check if intersection is behind ray
    if t < 0.0 {
        return None;
    }

    let point = ray.point_at(t);

    // Normal points toward the side the ray came from
    let normal = if denom < 0.0 {
        plane.normal
    } else {
        -plane.normal
    };

    Some(RayHit::new(t, point, normal))
}

/// Test ray vs collider intersection
///
/// Dispatches to the appropriate ray-shape test based on collider type.
pub fn ray_vs_collider(ray: &Ray4D, collider: &Collider) -> Option<RayHit> {
    match collider {
        Collider::Sphere(sphere) => ray_vs_sphere(ray, sphere),
        Collider::AABB(aabb) => ray_vs_aabb(ray, aabb),
        Collider::Plane(plane) => ray_vs_plane(ray, plane),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Ray vs Sphere Tests =====

    #[test]
    fn test_ray_vs_sphere_miss() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

        assert!(ray_vs_sphere(&ray, &sphere).is_none());
    }

    #[test]
    fn test_ray_vs_sphere_hit_through_center() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

        let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit");
        // Entry point at x = -1 (distance 4 from origin at -5)
        assert!((hit.distance - 4.0).abs() < 0.0001);
        assert!((hit.point.x - (-1.0)).abs() < 0.0001);
        assert!((hit.normal.x - (-1.0)).abs() < 0.0001); // Normal points outward
    }

    #[test]
    fn test_ray_vs_sphere_tangent() {
        // Ray that just grazes the top of the sphere
        let ray = Ray4D::new(Vec4::new(-5.0, 1.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

        let hit = ray_vs_sphere(&ray, &sphere);
        // Should hit at exactly one point (tangent)
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.point.y - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_sphere_origin_inside() {
        // Ray originates inside the sphere
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

        let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit exit point");
        // Exit point at x = 1
        assert!((hit.distance - 1.0).abs() < 0.0001);
        assert!((hit.point.x - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_sphere_behind_ray() {
        // Sphere is behind the ray origin
        let ray = Ray4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

        assert!(ray_vs_sphere(&ray, &sphere).is_none());
    }

    #[test]
    fn test_ray_vs_sphere_diagonal_hit() {
        // Ray going diagonally
        let ray = Ray4D::new(Vec4::new(-5.0, -5.0, 0.0, 0.0), Vec4::new(1.0, 1.0, 0.0, 0.0));
        let sphere = Sphere4D::new(Vec4::ZERO, 2.0);

        let hit = ray_vs_sphere(&ray, &sphere);
        assert!(hit.is_some());
    }

    // ===== Ray vs AABB Tests =====

    #[test]
    fn test_ray_vs_aabb_miss() {
        let ray = Ray4D::new(Vec4::new(0.0, 10.0, 0.0, 0.0), Vec4::X);
        let aabb = AABB4D::unit(); // -0.5 to 0.5 in all dimensions

        assert!(ray_vs_aabb(&ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_vs_aabb_hit_x_face() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");
        // Entry at x = -0.5
        assert!((hit.point.x - (-0.5)).abs() < 0.0001);
        assert!((hit.normal.x - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_aabb_hit_y_face() {
        let ray = Ray4D::new(Vec4::new(0.0, -5.0, 0.0, 0.0), Vec4::Y);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");
        assert!((hit.point.y - (-0.5)).abs() < 0.0001);
        assert!((hit.normal.y - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_aabb_hit_opposite_x_face() {
        // Ray coming from +X direction
        let ray = Ray4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), -Vec4::X);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");
        assert!((hit.point.x - 0.5).abs() < 0.0001);
        assert!((hit.normal.x - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_aabb_parallel_to_axis() {
        // Ray parallel to Y axis but passing through AABB
        let ray = Ray4D::new(Vec4::new(0.0, -5.0, 0.0, 0.0), Vec4::Y);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb);
        assert!(hit.is_some());
    }

    #[test]
    fn test_ray_vs_aabb_parallel_miss() {
        // Ray parallel to Y axis but outside AABB in X
        let ray = Ray4D::new(Vec4::new(5.0, -5.0, 0.0, 0.0), Vec4::Y);
        let aabb = AABB4D::unit();

        assert!(ray_vs_aabb(&ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_vs_aabb_origin_inside() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit exit point");
        // Exit at x = 0.5
        assert!((hit.point.x - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_aabb_behind_ray() {
        let ray = Ray4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), Vec4::X);
        let aabb = AABB4D::unit();

        assert!(ray_vs_aabb(&ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_vs_aabb_w_axis() {
        // Test the 4th dimension
        let ray = Ray4D::new(Vec4::new(0.0, 0.0, 0.0, -5.0), Vec4::W);
        let aabb = AABB4D::unit();

        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");
        assert!((hit.point.w - (-0.5)).abs() < 0.0001);
        assert!((hit.normal.w - (-1.0)).abs() < 0.0001);
    }

    // ===== Ray vs Plane Tests =====

    #[test]
    fn test_ray_vs_plane_hit_from_above() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), -Vec4::Y);
        let plane = Plane4D::floor(0.0);

        let hit = ray_vs_plane(&ray, &plane).expect("Should hit");
        assert!((hit.distance - 5.0).abs() < 0.0001);
        assert!((hit.point.y).abs() < 0.0001);
        assert!((hit.normal.y - 1.0).abs() < 0.0001); // Normal points toward ray
    }

    #[test]
    fn test_ray_vs_plane_hit_from_below() {
        let ray = Ray4D::new(Vec4::new(0.0, -5.0, 0.0, 0.0), Vec4::Y);
        let plane = Plane4D::floor(0.0);

        let hit = ray_vs_plane(&ray, &plane).expect("Should hit");
        assert!((hit.distance - 5.0).abs() < 0.0001);
        assert!((hit.point.y).abs() < 0.0001);
        assert!((hit.normal.y - (-1.0)).abs() < 0.0001); // Normal points toward ray
    }

    #[test]
    fn test_ray_vs_plane_parallel_miss() {
        // Ray parallel to the floor
        let ray = Ray4D::new(Vec4::new(0.0, 1.0, 0.0, 0.0), Vec4::X);
        let plane = Plane4D::floor(0.0);

        assert!(ray_vs_plane(&ray, &plane).is_none());
    }

    #[test]
    fn test_ray_vs_plane_behind_ray() {
        // Ray pointing away from the plane
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::Y);
        let plane = Plane4D::floor(0.0);

        assert!(ray_vs_plane(&ray, &plane).is_none());
    }

    #[test]
    fn test_ray_vs_plane_angled() {
        // Ray at 45 degrees to floor
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::new(1.0, -1.0, 0.0, 0.0));
        let plane = Plane4D::floor(0.0);

        let hit = ray_vs_plane(&ray, &plane).expect("Should hit");
        // At t=5*sqrt(2), ray reaches y=0
        assert!((hit.point.y).abs() < 0.0001);
        assert!(hit.point.x > 0.0); // Moved in positive X
    }

    // ===== Ray vs Collider Tests =====

    #[test]
    fn test_ray_vs_collider_sphere() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let collider = Collider::Sphere(Sphere4D::new(Vec4::ZERO, 1.0));

        let hit = ray_vs_collider(&ray, &collider).expect("Should hit");
        assert!((hit.point.x - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_collider_aabb() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let collider = Collider::AABB(AABB4D::unit());

        let hit = ray_vs_collider(&ray, &collider).expect("Should hit");
        assert!((hit.point.x - (-0.5)).abs() < 0.0001);
    }

    #[test]
    fn test_ray_vs_collider_plane() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), -Vec4::Y);
        let collider = Collider::Plane(Plane4D::floor(0.0));

        let hit = ray_vs_collider(&ray, &collider).expect("Should hit");
        assert!((hit.point.y).abs() < 0.0001);
    }
}
