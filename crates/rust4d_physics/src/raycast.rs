//! Raycasting for 4D physics shapes
//!
//! Provides ray intersection tests against spheres, AABBs, and planes.

use crate::shapes::{Collider, Plane4D, Sphere4D, AABB4D};
use rust4d_math::{Ray4D, Vec4};

/// Information about a ray hit
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// Distance from ray origin to hit point
    pub distance: f32,
    /// The point where the ray hit
    pub point: Vec4,
    /// Surface normal at the hit point (pointing away from the surface)
    pub normal: Vec4,
}

/// Test ray vs sphere intersection
///
/// Returns the nearest intersection in front of the ray (t >= 0).
/// If the ray origin is inside the sphere, returns the exit point.
pub fn ray_vs_sphere(ray: &Ray4D, sphere: &Sphere4D) -> Option<RayHit> {
    let oc = ray.origin - sphere.center;
    // Since direction is normalized: a = 1.0
    let b = oc.dot(ray.direction);
    let c = oc.length_squared() - sphere.radius * sphere.radius;
    let discriminant = b * b - c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();

    // Try nearest root first
    let t = -b - sqrt_disc;
    if t >= 0.0 {
        let point = ray.point_at(t);
        let normal = (point - sphere.center).normalized();
        return Some(RayHit {
            distance: t,
            point,
            normal,
        });
    }

    // Try far root (ray origin inside sphere)
    let t = -b + sqrt_disc;
    if t >= 0.0 {
        let point = ray.point_at(t);
        let normal = (point - sphere.center).normalized();
        return Some(RayHit {
            distance: t,
            point,
            normal,
        });
    }

    None
}

/// Test ray vs AABB intersection using the slab method
///
/// Returns the nearest intersection in front of the ray (t >= 0).
pub fn ray_vs_aabb(ray: &Ray4D, aabb: &AABB4D) -> Option<RayHit> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::MAX;
    let mut hit_axis = 0usize;
    let mut hit_sign = 1.0_f32;

    // Check each axis using the slab method
    let origin = [ray.origin.x, ray.origin.y, ray.origin.z, ray.origin.w];
    let dir = [
        ray.direction.x,
        ray.direction.y,
        ray.direction.z,
        ray.direction.w,
    ];
    let mins = [aabb.min.x, aabb.min.y, aabb.min.z, aabb.min.w];
    let maxs = [aabb.max.x, aabb.max.y, aabb.max.z, aabb.max.w];

    for axis in 0..4 {
        if dir[axis].abs() < 1e-8 {
            // Ray is parallel to this slab
            if origin[axis] < mins[axis] || origin[axis] > maxs[axis] {
                return None;
            }
        } else {
            let inv_d = 1.0 / dir[axis];
            let mut t1 = (mins[axis] - origin[axis]) * inv_d;
            let mut t2 = (maxs[axis] - origin[axis]) * inv_d;

            let mut sign = -1.0;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                sign = 1.0;
            }

            if t1 > t_min {
                t_min = t1;
                hit_axis = axis;
                hit_sign = sign;
            }
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        }
    }

    if t_min < 0.0 {
        // Ray origin is inside the AABB; use t_max as exit point
        // But we need a valid hit, so return exit point
        if t_max < 0.0 {
            return None;
        }
        // For origin inside AABB, we return the exit point
        let point = ray.point_at(t_max);
        // Normal points outward at exit
        let mut normal = Vec4::ZERO;
        let exit_origin = [point.x, point.y, point.z, point.w];
        // Find which face we exit through
        let mut best_axis = 0;
        let mut best_dist = f32::MAX;
        for axis in 0..4 {
            let dist_min = (exit_origin[axis] - mins[axis]).abs();
            let dist_max = (exit_origin[axis] - maxs[axis]).abs();
            if dist_min < best_dist {
                best_dist = dist_min;
                best_axis = axis;
            }
            if dist_max < best_dist {
                best_dist = dist_max;
                best_axis = axis;
            }
        }
        match best_axis {
            0 => {
                normal.x = if exit_origin[0] > (mins[0] + maxs[0]) * 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            1 => {
                normal.y = if exit_origin[1] > (mins[1] + maxs[1]) * 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            2 => {
                normal.z = if exit_origin[2] > (mins[2] + maxs[2]) * 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            3 => {
                normal.w = if exit_origin[3] > (mins[3] + maxs[3]) * 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => unreachable!(),
        }
        return Some(RayHit {
            distance: t_max,
            point,
            normal,
        });
    }

    let point = ray.point_at(t_min);
    let mut normal = Vec4::ZERO;
    match hit_axis {
        0 => normal.x = hit_sign,
        1 => normal.y = hit_sign,
        2 => normal.z = hit_sign,
        3 => normal.w = hit_sign,
        _ => unreachable!(),
    }

    Some(RayHit {
        distance: t_min,
        point,
        normal,
    })
}

/// Test ray vs plane intersection
///
/// Returns the intersection point if the ray hits the plane from the front (t >= 0).
pub fn ray_vs_plane(ray: &Ray4D, plane: &Plane4D) -> Option<RayHit> {
    let denom = ray.direction.dot(plane.normal);

    // Ray is parallel to the plane
    if denom.abs() < 1e-8 {
        return None;
    }

    let t = (plane.distance - ray.origin.dot(plane.normal)) / denom;

    if t < 0.0 {
        return None;
    }

    let point = ray.point_at(t);
    // Normal always faces toward the ray origin side
    let normal = if denom < 0.0 {
        plane.normal
    } else {
        -plane.normal
    };

    Some(RayHit {
        distance: t,
        point,
        normal,
    })
}

/// Test ray vs any collider shape
///
/// Dispatches to the appropriate shape-specific intersection test.
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

    // ===== ray_vs_sphere tests =====

    #[test]
    fn test_ray_vs_sphere_miss() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);
        assert!(ray_vs_sphere(&ray, &sphere).is_none());
    }

    #[test]
    fn test_ray_vs_sphere_tangent() {
        // Ray just grazes the top of the sphere
        let ray = Ray4D::new(Vec4::new(-5.0, 1.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);
        // At tangent, discriminant ~ 0; may or may not register as hit
        // depending on floating point. The important thing is no crash.
        let _ = ray_vs_sphere(&ray, &sphere);
    }

    #[test]
    fn test_ray_vs_sphere_through_center() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);
        let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit");

        // Hit point should be at x=-1 (entry point)
        assert!((hit.point.x - (-1.0)).abs() < 0.001);
        assert!((hit.distance - 4.0).abs() < 0.001);
        // Normal should point toward -X at entry
        assert!(hit.normal.x < -0.9);
    }

    #[test]
    fn test_ray_vs_sphere_origin_inside() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 2.0);
        let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit exit point");

        // Should hit the exit point at x=2
        assert!((hit.point.x - 2.0).abs() < 0.001);
        assert!((hit.distance - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_ray_vs_sphere_behind_ray() {
        // Sphere is behind the ray
        let ray = Ray4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), Vec4::X);
        let sphere = Sphere4D::new(Vec4::ZERO, 1.0);
        assert!(ray_vs_sphere(&ray, &sphere).is_none());
    }

    // ===== ray_vs_aabb tests =====

    #[test]
    fn test_ray_vs_aabb_miss() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::X);
        let aabb = AABB4D::unit();
        assert!(ray_vs_aabb(&ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_vs_aabb_hit() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let aabb = AABB4D::unit(); // -0.5 to 0.5
        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");

        // Should hit at x=-0.5
        assert!((hit.point.x - (-0.5)).abs() < 0.001);
        assert!((hit.distance - 4.5).abs() < 0.001);
        // Normal should point -X
        assert!(hit.normal.x < -0.9);
    }

    #[test]
    fn test_ray_vs_aabb_parallel_to_axis() {
        // Ray parallel to Y axis, passing through AABB
        let ray = Ray4D::new(Vec4::new(0.0, -5.0, 0.0, 0.0), Vec4::Y);
        let aabb = AABB4D::unit();
        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit");

        assert!((hit.point.y - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn test_ray_vs_aabb_origin_inside() {
        let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
        let aabb = AABB4D::unit();
        let hit = ray_vs_aabb(&ray, &aabb).expect("Should hit exit");

        // Should hit exit face at x=0.5
        assert!(
            (hit.point.x - 0.5).abs() < 0.01,
            "Exit point x should be 0.5, got {}",
            hit.point.x
        );
        assert!(hit.distance > 0.0, "Distance should be positive");
    }

    // ===== ray_vs_plane tests =====

    #[test]
    fn test_ray_vs_plane_hit_from_above() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), -Vec4::Y);
        let plane = Plane4D::floor(0.0);
        let hit = ray_vs_plane(&ray, &plane).expect("Should hit");

        assert!((hit.point.y).abs() < 0.001);
        assert!((hit.distance - 5.0).abs() < 0.001);
        assert!(hit.normal.y > 0.9); // Normal pointing up
    }

    #[test]
    fn test_ray_vs_plane_hit_from_below() {
        let ray = Ray4D::new(Vec4::new(0.0, -5.0, 0.0, 0.0), Vec4::Y);
        let plane = Plane4D::floor(0.0);
        let hit = ray_vs_plane(&ray, &plane).expect("Should hit");

        assert!((hit.point.y).abs() < 0.001);
        assert!((hit.distance - 5.0).abs() < 0.001);
        assert!(hit.normal.y < -0.9); // Normal pointing down (toward ray origin)
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
        // Ray pointing away from plane
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), Vec4::Y);
        let plane = Plane4D::floor(0.0);
        assert!(ray_vs_plane(&ray, &plane).is_none());
    }

    // ===== ray_vs_collider tests =====

    #[test]
    fn test_ray_vs_collider_sphere() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let collider = Collider::Sphere(Sphere4D::new(Vec4::ZERO, 1.0));
        assert!(ray_vs_collider(&ray, &collider).is_some());
    }

    #[test]
    fn test_ray_vs_collider_aabb() {
        let ray = Ray4D::new(Vec4::new(-5.0, 0.0, 0.0, 0.0), Vec4::X);
        let collider = Collider::AABB(AABB4D::unit());
        assert!(ray_vs_collider(&ray, &collider).is_some());
    }

    #[test]
    fn test_ray_vs_collider_plane() {
        let ray = Ray4D::new(Vec4::new(0.0, 5.0, 0.0, 0.0), -Vec4::Y);
        let collider = Collider::Plane(Plane4D::floor(0.0));
        assert!(ray_vs_collider(&ray, &collider).is_some());
    }
}
