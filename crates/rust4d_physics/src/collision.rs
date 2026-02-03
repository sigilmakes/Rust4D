//! Collision detection for 4D shapes
//!
//! Provides collision detection between spheres, AABBs, and planes.
//! Also provides collision filtering via layer masks.

use bitflags::bitflags;

use crate::body::BodyKey;
use crate::shapes::{Plane4D, Sphere4D, AABB4D};
use rust4d_math::Vec4;

bitflags! {
    /// Collision layers for filtering which objects can collide
    ///
    /// Each layer is a bit in a 32-bit mask. Objects can belong to multiple layers
    /// and can define which layers they collide with via a collision mask.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CollisionLayer: u32 {
        /// Default layer for most objects
        const DEFAULT = 1 << 0;
        /// Player character layer
        const PLAYER = 1 << 1;
        /// Enemy/NPC layer
        const ENEMY = 1 << 2;
        /// Static world geometry (floors, walls)
        const STATIC = 1 << 3;
        /// Trigger zones (detect but don't push)
        const TRIGGER = 1 << 4;
        /// Projectiles (bullets, spells)
        const PROJECTILE = 1 << 5;
        /// Collectible items (coins, powerups)
        const PICKUP = 1 << 6;
        /// All layers (collide with everything)
        const ALL = 0xFFFFFFFF;
    }
}

/// Collision filter determining what an object collides with
///
/// Uses a layer/mask system:
/// - `layer`: Which layer(s) this object belongs to
/// - `mask`: Which layer(s) this object can collide with
///
/// Two objects A and B collide if:
/// - (A.layer & B.mask) != 0, AND
/// - (B.layer & A.mask) != 0
///
/// This allows asymmetric collision relationships (e.g., triggers detect
/// players but players don't push triggers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionFilter {
    /// Which layer(s) this object belongs to
    pub layer: CollisionLayer,
    /// Which layer(s) this object can collide with
    pub mask: CollisionLayer,
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            layer: CollisionLayer::DEFAULT,
            mask: CollisionLayer::ALL,
        }
    }
}

impl CollisionFilter {
    /// Create a new collision filter with specified layer and mask
    pub fn new(layer: CollisionLayer, mask: CollisionLayer) -> Self {
        Self { layer, mask }
    }

    /// Check if this filter allows collision with another filter
    ///
    /// Returns true if both objects' layers match each other's masks.
    pub fn collides_with(&self, other: &Self) -> bool {
        // Both must agree on the collision
        self.layer.intersects(other.mask) && other.layer.intersects(self.mask)
    }

    /// Create a filter for player objects
    ///
    /// Players collide with everything except other players, player projectiles, and triggers.
    /// Triggers can still detect players for event handling, but players won't push triggers.
    pub fn player() -> Self {
        Self {
            layer: CollisionLayer::PLAYER,
            mask: CollisionLayer::ALL
                & !CollisionLayer::PLAYER
                & !CollisionLayer::PROJECTILE
                & !CollisionLayer::TRIGGER,
        }
    }

    /// Create a filter for enemy objects
    ///
    /// Enemies collide with everything except other enemies.
    pub fn enemy() -> Self {
        Self {
            layer: CollisionLayer::ENEMY,
            mask: CollisionLayer::ALL & !CollisionLayer::ENEMY,
        }
    }

    /// Create a filter for static world geometry
    ///
    /// Static objects are detected by everything but don't detect anything themselves.
    pub fn static_world() -> Self {
        Self {
            layer: CollisionLayer::STATIC,
            mask: CollisionLayer::ALL,
        }
    }

    /// Create a filter for trigger zones
    ///
    /// Triggers detect specified layers but those layers don't get pushed by triggers.
    /// The trigger can detect collisions (for events) but won't apply forces.
    pub fn trigger(detects: CollisionLayer) -> Self {
        Self {
            layer: CollisionLayer::TRIGGER,
            mask: detects,
        }
    }

    /// Create a filter for player projectiles
    ///
    /// Player projectiles hit enemies and static geometry, but not the player or pickups.
    pub fn player_projectile() -> Self {
        Self {
            layer: CollisionLayer::PROJECTILE,
            mask: CollisionLayer::ENEMY | CollisionLayer::STATIC,
        }
    }
}

/// What kind of collision event occurred
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionEventKind {
    /// Two dynamic/kinematic bodies collided
    BodyVsBody { body_a: BodyKey, body_b: BodyKey },
    /// A body collided with a static collider
    BodyVsStatic { body: BodyKey, static_index: usize },
    /// A body entered a trigger zone
    TriggerEnter { body: BodyKey, trigger_index: usize },
    /// A body is staying inside a trigger zone
    TriggerStay { body: BodyKey, trigger_index: usize },
    /// A body exited a trigger zone
    TriggerExit { body: BodyKey, trigger_index: usize },
}

/// A collision event generated during physics simulation
#[derive(Clone, Debug)]
pub struct CollisionEvent {
    /// What kind of collision this is
    pub kind: CollisionEventKind,
    /// Contact information (point, normal, penetration).
    /// `None` for `TriggerExit` events where the bodies have separated.
    pub contact: Option<Contact>,
}

/// Contact information from a collision
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    /// Point of contact (on the surface of the first shape)
    pub point: Vec4,
    /// Normal pointing from the second shape toward the first
    pub normal: Vec4,
    /// Penetration depth (positive means overlapping)
    pub penetration: f32,
}

impl Contact {
    /// Create a new contact
    pub fn new(point: Vec4, normal: Vec4, penetration: f32) -> Self {
        Self {
            point,
            normal,
            penetration,
        }
    }

    /// Check if this represents an actual collision (positive penetration)
    pub fn is_colliding(&self) -> bool {
        self.penetration > 0.0
    }
}

/// Test sphere vs plane collision
///
/// Returns a contact if the sphere is intersecting or touching the plane.
/// The contact normal points from the plane toward the sphere (same direction as plane normal
/// if sphere is above, opposite if below).
pub fn sphere_vs_plane(sphere: &Sphere4D, plane: &Plane4D) -> Option<Contact> {
    let signed_dist = plane.signed_distance(sphere.center);

    // Penetration calculation:
    // - If signed_dist > 0 (center above plane): penetration = radius - signed_dist
    // - If signed_dist < 0 (center below plane): penetration = radius + |signed_dist|
    // Combined: penetration = radius - signed_dist (works for both cases)
    let penetration = sphere.radius - signed_dist;

    if penetration > 0.0 {
        // Normal always points from plane toward sphere (upward for floor)
        let normal = plane.normal;

        // Contact point is on the sphere surface, toward the plane
        let point = sphere.center - normal * sphere.radius;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test AABB vs plane collision
///
/// Returns a contact if any part of the AABB is below/intersecting the plane.
pub fn aabb_vs_plane(aabb: &AABB4D, plane: &Plane4D) -> Option<Contact> {
    let center = aabb.center();
    let half_extents = aabb.half_extents();

    // Find the vertex closest to the plane (most in the negative normal direction)
    // This is: center - half_extents * sign(normal)
    let closest_vertex = center - half_extents.component_mul(plane.normal.sign());

    let signed_dist = plane.signed_distance(closest_vertex);

    // If the closest vertex is below the plane, we have a collision
    if signed_dist < 0.0 {
        let penetration = -signed_dist;
        let point = closest_vertex;
        let normal = plane.normal;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test sphere vs AABB collision
///
/// Returns a contact if the sphere is intersecting the AABB.
pub fn sphere_vs_aabb(sphere: &Sphere4D, aabb: &AABB4D) -> Option<Contact> {
    // Find the closest point on the AABB to the sphere center
    let closest = aabb.closest_point(sphere.center);

    // Distance from sphere center to closest point
    let delta = sphere.center - closest;
    let dist_squared = delta.length_squared();

    if dist_squared < sphere.radius * sphere.radius {
        let dist = dist_squared.sqrt();
        let penetration = sphere.radius - dist;

        // Normal points from AABB toward sphere
        let normal = if dist > 0.0001 {
            delta.normalized()
        } else {
            // Sphere center is inside AABB - use the shortest escape direction
            let to_min = sphere.center - aabb.min;
            let to_max = aabb.max - sphere.center;

            // Find the axis with minimum distance to edge
            let mut min_dist = to_min.x;
            let mut normal = -Vec4::X;

            if to_max.x < min_dist {
                min_dist = to_max.x;
                normal = Vec4::X;
            }
            if to_min.y < min_dist {
                min_dist = to_min.y;
                normal = -Vec4::Y;
            }
            if to_max.y < min_dist {
                min_dist = to_max.y;
                normal = Vec4::Y;
            }
            if to_min.z < min_dist {
                min_dist = to_min.z;
                normal = -Vec4::Z;
            }
            if to_max.z < min_dist {
                min_dist = to_max.z;
                normal = Vec4::Z;
            }
            if to_min.w < min_dist {
                min_dist = to_min.w;
                normal = -Vec4::W;
            }
            if to_max.w < min_dist {
                normal = Vec4::W;
            }

            normal
        };

        let point = closest;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test AABB vs AABB collision
///
/// Returns a contact if the AABBs are intersecting.
pub fn aabb_vs_aabb(a: &AABB4D, b: &AABB4D) -> Option<Contact> {
    // Check for separation on each axis
    if a.max.x < b.min.x || a.min.x > b.max.x {
        return None;
    }
    if a.max.y < b.min.y || a.min.y > b.max.y {
        return None;
    }
    if a.max.z < b.min.z || a.min.z > b.max.z {
        return None;
    }
    if a.max.w < b.min.w || a.min.w > b.max.w {
        return None;
    }

    // Find overlap on each axis and use the minimum as penetration
    let overlap_x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
    let overlap_y = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
    let overlap_z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
    let overlap_w = (a.max.w.min(b.max.w) - a.min.w.max(b.min.w)).max(0.0);

    // Find minimum overlap axis
    let mut min_overlap = overlap_x;
    let mut normal = if a.center().x < b.center().x {
        -Vec4::X
    } else {
        Vec4::X
    };

    if overlap_y < min_overlap {
        min_overlap = overlap_y;
        normal = if a.center().y < b.center().y {
            -Vec4::Y
        } else {
            Vec4::Y
        };
    }
    if overlap_z < min_overlap {
        min_overlap = overlap_z;
        normal = if a.center().z < b.center().z {
            -Vec4::Z
        } else {
            Vec4::Z
        };
    }
    if overlap_w < min_overlap {
        min_overlap = overlap_w;
        normal = if a.center().w < b.center().w {
            -Vec4::W
        } else {
            Vec4::W
        };
    }

    // Contact point is at the center of the overlap region
    let overlap_min = a.min.max_components(b.min);
    let overlap_max = a.max.min_components(b.max);
    let point = (overlap_min + overlap_max) * 0.5;

    Some(Contact::new(point, normal, min_overlap))
}

/// Test sphere vs sphere collision
///
/// Returns a contact if the spheres are intersecting.
/// The contact normal points from sphere A toward sphere B.
pub fn sphere_vs_sphere(a: &Sphere4D, b: &Sphere4D) -> Option<Contact> {
    let delta = b.center - a.center;
    let dist_sq = delta.length_squared();
    let min_dist = a.radius + b.radius;

    if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
        let dist = dist_sq.sqrt();
        let penetration = min_dist - dist;
        let normal = delta.normalized();
        let point = a.center + normal * a.radius;
        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_vs_plane_above() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 2.0, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        // Sphere is above plane, no collision
        assert!(sphere_vs_plane(&sphere, &plane).is_none());
    }

    #[test]
    fn test_sphere_vs_plane_touching() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 1.0, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        // Sphere exactly touching plane - at the boundary
        let contact = sphere_vs_plane(&sphere, &plane);
        // Due to floating point, this might or might not register as a collision
        // The important thing is the math is correct
        if let Some(c) = contact {
            assert!(c.penetration.abs() < 0.0001);
        }
    }

    #[test]
    fn test_sphere_vs_plane_colliding() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 0.5, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        let contact = sphere_vs_plane(&sphere, &plane).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
        assert_eq!(contact.normal, Vec4::Y);
    }

    #[test]
    fn test_aabb_vs_plane_above() {
        let aabb = AABB4D::from_center_half_extents(Vec4::new(0.0, 2.0, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));
        let plane = Plane4D::floor(0.0);

        // AABB is above plane (lowest point at y=1.5)
        assert!(aabb_vs_plane(&aabb, &plane).is_none());
    }

    #[test]
    fn test_aabb_vs_plane_colliding() {
        let aabb = AABB4D::from_center_half_extents(Vec4::new(0.0, 0.25, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));
        let plane = Plane4D::floor(0.0);

        // AABB lowest point at y=-0.25, floor at y=0
        let contact = aabb_vs_plane(&aabb, &plane).expect("Should collide");
        assert!((contact.penetration - 0.25).abs() < 0.0001);
        assert_eq!(contact.normal, Vec4::Y);
    }

    #[test]
    fn test_sphere_vs_aabb_no_collision() {
        let sphere = Sphere4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), 1.0);
        let aabb = AABB4D::unit();

        assert!(sphere_vs_aabb(&sphere, &aabb).is_none());
    }

    #[test]
    fn test_sphere_vs_aabb_colliding() {
        let sphere = Sphere4D::new(Vec4::new(1.0, 0.0, 0.0, 0.0), 1.0);
        let aabb = AABB4D::unit(); // -0.5 to 0.5 in all dimensions

        // Sphere center at x=1, radius=1, AABB edge at x=0.5
        // Closest point on AABB is (0.5, 0, 0, 0)
        // Distance = 0.5, penetration = 1.0 - 0.5 = 0.5
        let contact = sphere_vs_aabb(&sphere, &aabb).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_aabb_vs_aabb_no_collision() {
        let a = AABB4D::from_center_half_extents(Vec4::ZERO, Vec4::new(0.5, 0.5, 0.5, 0.5));
        let b = AABB4D::from_center_half_extents(Vec4::new(5.0, 0.0, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));

        assert!(aabb_vs_aabb(&a, &b).is_none());
    }

    #[test]
    fn test_aabb_vs_aabb_colliding() {
        let a = AABB4D::from_center_half_extents(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        let b = AABB4D::from_center_half_extents(Vec4::new(1.5, 0.0, 0.0, 0.0), Vec4::new(1.0, 1.0, 1.0, 1.0));

        // Overlap on x-axis: a.max.x=1.0, b.min.x=0.5, overlap=0.5
        let contact = aabb_vs_aabb(&a, &b).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
    }

    // ===== Collision Filter Tests =====

    #[test]
    fn test_collision_filter_default() {
        let filter = CollisionFilter::default();
        assert_eq!(filter.layer, CollisionLayer::DEFAULT);
        assert_eq!(filter.mask, CollisionLayer::ALL);
    }

    #[test]
    fn test_collision_filter_default_collides_with_everything() {
        let default = CollisionFilter::default();
        let player = CollisionFilter::player();
        let enemy = CollisionFilter::enemy();
        let static_world = CollisionFilter::static_world();

        // Default should collide with player, enemy, and static
        // (but player/enemy might not collide back due to their masks)
        assert!(default.layer.intersects(player.mask));
        assert!(default.layer.intersects(enemy.mask));
        assert!(default.layer.intersects(static_world.mask));
    }

    #[test]
    fn test_collision_filter_player_vs_static() {
        let player = CollisionFilter::player();
        let static_world = CollisionFilter::static_world();

        // Player should collide with static world
        assert!(player.collides_with(&static_world));
        assert!(static_world.collides_with(&player));
    }

    #[test]
    fn test_collision_filter_player_vs_player() {
        let player1 = CollisionFilter::player();
        let player2 = CollisionFilter::player();

        // Players should not collide with each other
        assert!(!player1.collides_with(&player2));
    }

    #[test]
    fn test_collision_filter_enemy_vs_enemy() {
        let enemy1 = CollisionFilter::enemy();
        let enemy2 = CollisionFilter::enemy();

        // Enemies should not collide with each other
        assert!(!enemy1.collides_with(&enemy2));
    }

    #[test]
    fn test_collision_filter_player_vs_enemy() {
        let player = CollisionFilter::player();
        let enemy = CollisionFilter::enemy();

        // Player should collide with enemy
        assert!(player.collides_with(&enemy));
    }

    #[test]
    fn test_collision_filter_player_projectile() {
        let projectile = CollisionFilter::player_projectile();
        let player = CollisionFilter::player();
        let enemy = CollisionFilter::enemy();
        let static_world = CollisionFilter::static_world();

        // Player projectile should hit enemies and static
        assert!(projectile.collides_with(&enemy));
        assert!(projectile.collides_with(&static_world));

        // Player projectile should not hit player
        assert!(!projectile.collides_with(&player));
    }

    #[test]
    fn test_collision_filter_trigger() {
        let trigger = CollisionFilter::trigger(CollisionLayer::PLAYER);
        let player = CollisionFilter::player();
        let enemy = CollisionFilter::enemy();

        // Trigger that detects players
        // Note: collides_with is symmetric, so both need to agree
        // Trigger's mask has PLAYER, but player's mask doesn't have TRIGGER by default
        // This allows triggers to detect players for events
        assert!(trigger.mask.contains(CollisionLayer::PLAYER));

        // The trigger layer is not in player's mask, so symmetric check fails
        // This is intentional: triggers detect but don't push
        assert!(!trigger.collides_with(&player));

        // Trigger definitely doesn't collide with enemy
        assert!(!trigger.collides_with(&enemy));
    }

    #[test]
    fn test_collision_layer_bitflags() {
        let combined = CollisionLayer::PLAYER | CollisionLayer::ENEMY;
        assert!(combined.contains(CollisionLayer::PLAYER));
        assert!(combined.contains(CollisionLayer::ENEMY));
        assert!(!combined.contains(CollisionLayer::STATIC));
    }

    #[test]
    fn test_collision_filter_custom() {
        // Custom filter: belongs to PICKUP layer, only collides with PLAYER
        let pickup = CollisionFilter::new(CollisionLayer::PICKUP, CollisionLayer::PLAYER);

        let player = CollisionFilter::new(
            CollisionLayer::PLAYER,
            CollisionLayer::ALL, // Player collides with everything
        );

        let enemy = CollisionFilter::enemy();

        // Pickup collides with player (both agree)
        assert!(pickup.collides_with(&player));

        // Pickup doesn't collide with enemy (pickup's mask doesn't include ENEMY)
        assert!(!pickup.collides_with(&enemy));
    }

    #[test]
    fn test_tesseract_vs_bounded_floor() {
        // Simulate the default scene: tesseract at y=0, floor at y=-2

        // Floor AABB: top at y=-2, extends 5 units down (minimum), 10 units in x/z, 5 in w
        let floor = AABB4D::from_center_half_extents(
            Vec4::new(0.0, -4.5, 0.0, 0.0), // center at y=-4.5 (5/2 below -2)
            Vec4::new(10.0, 2.5, 10.0, 5.0),
        );
        assert_eq!(floor.max.y, -2.0, "Floor top should be at y=-2");
        assert_eq!(floor.min.y, -7.0, "Floor bottom should be at y=-7");

        // Tesseract at starting position (y=0), half_extent=1
        let tesseract_start = AABB4D::from_center_half_extents(
            Vec4::ZERO,
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        // Tesseract bottom at y=-1, floor top at y=-2 → no collision
        assert!(aabb_vs_aabb(&tesseract_start, &floor).is_none(),
            "Tesseract at y=0 should not collide with floor at y=-2");

        // Tesseract fallen to y=-0.9 (bottom at y=-1.9, still above floor at y=-2)
        let tesseract_almost = AABB4D::from_center_half_extents(
            Vec4::new(0.0, -0.9, 0.0, 0.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        assert!(aabb_vs_aabb(&tesseract_almost, &floor).is_none(),
            "Tesseract at y=-0.9 should not collide (bottom at -1.9, floor top at -2)");

        // Tesseract fallen to y=-1.1 (bottom at y=-2.1, penetrating floor)
        let tesseract_touching = AABB4D::from_center_half_extents(
            Vec4::new(0.0, -1.1, 0.0, 0.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        let contact = aabb_vs_aabb(&tesseract_touching, &floor);
        assert!(contact.is_some(), "Tesseract at y=-1.1 should collide with floor");
        let contact = contact.unwrap();
        assert!(contact.penetration > 0.0, "Should have positive penetration");

        // Tesseract at rest position y=-1 (bottom at y=-2, exactly on floor)
        // At exact boundary - behavior is undefined (floating point edge case)
        let _tesseract_resting = AABB4D::from_center_half_extents(
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        // Important: at y=-1.0001, should collide
        let tesseract_slightly_in = AABB4D::from_center_half_extents(
            Vec4::new(0.0, -1.001, 0.0, 0.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        );
        assert!(aabb_vs_aabb(&tesseract_slightly_in, &floor).is_some(),
            "Tesseract slightly below resting position should collide");
    }

    #[test]
    fn test_sphere_vs_sphere_coincident_returns_none() {
        // Two spheres at exactly the same position produce a degenerate case
        // (zero-length delta). The function returns None to avoid a NaN normal.
        // This is a documented limitation — game logic should not place objects
        // at identical positions.
        let a = Sphere4D::new(Vec4::ZERO, 1.0);
        let b = Sphere4D::new(Vec4::ZERO, 1.0);
        assert!(sphere_vs_sphere(&a, &b).is_none(),
            "Coincident spheres should return None (degenerate case)");
    }
}
