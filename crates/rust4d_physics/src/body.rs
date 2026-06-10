//! Rigid body types for 4D physics simulation

use crate::collision::CollisionFilter;
use crate::material::PhysicsMaterial;
use crate::shapes::{Collider, Plane4D};
use rust4d_math::Vec4;
use slotmap::new_key_type;

// Define generational key type for rigid bodies
new_key_type! {
    /// Key to a rigid body in the physics world
    ///
    /// Uses generational indexing to prevent the ABA problem where a handle
    /// could point to a reused slot. If a body is removed and its slot reused,
    /// old keys will return None instead of pointing to the wrong body.
    pub struct BodyKey;
}

/// Type of rigid body that determines how it's simulated
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BodyType {
    /// Full physics simulation with gravity and collision response
    #[default]
    Dynamic,
    /// Never moves, used for floors, walls, platforms
    Static,
    /// User-controlled velocity (gravity opt-in via `gravity_enabled` flag)
    ///
    /// By default, kinematic bodies have gravity disabled. Enable gravity with
    /// `with_gravity(true)` for player characters that need jumping/falling.
    Kinematic,
}

/// A 4D rigid body with position, velocity, and collision shape
#[derive(Clone, Debug)]
pub struct RigidBody4D {
    /// Position in 4D space (world coordinates)
    pub position: Vec4,
    /// Velocity in 4D space (units per second)
    pub velocity: Vec4,
    /// Mass of the body (used for push calculations)
    pub mass: f32,
    /// Physical material properties (friction and restitution)
    pub material: PhysicsMaterial,
    /// The collision shape for this body (stores absolute world position)
    pub collider: Collider,
    /// Type of body (Dynamic, Static, or Kinematic)
    pub body_type: BodyType,
    /// Whether this body is touching the ground (set by physics step)
    pub grounded: bool,
    /// Collision filter (layer membership and collision mask)
    pub filter: CollisionFilter,
    /// Whether gravity applies to this body (independent of body type)
    pub gravity_enabled: bool,
}

impl RigidBody4D {
    /// Check if this body is affected by gravity
    #[inline]
    pub fn affected_by_gravity(&self) -> bool {
        self.gravity_enabled
    }

    /// Check if this body is static (never moves)
    #[inline]
    pub fn is_static(&self) -> bool {
        self.body_type == BodyType::Static
    }

    /// Check if this body is kinematic (user-controlled, gravity opt-in)
    #[inline]
    pub fn is_kinematic(&self) -> bool {
        self.body_type == BodyType::Kinematic
    }
}

// Additional RigidBody4D constructors and builder methods
impl RigidBody4D {
    /// Create a new rigid body with a sphere collider
    pub fn new_sphere(position: Vec4, radius: f32) -> Self {
        use crate::shapes::Sphere4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            material: PhysicsMaterial::default(),
            collider: Collider::Sphere(Sphere4D::new(position, radius)),
            body_type: BodyType::Dynamic,
            grounded: false,
            filter: CollisionFilter::default(),
            gravity_enabled: true, // Dynamic bodies have gravity by default
        }
    }

    /// Create a new rigid body with an AABB collider
    pub fn new_aabb(position: Vec4, half_extents: Vec4) -> Self {
        use crate::shapes::AABB4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            material: PhysicsMaterial::default(),
            collider: Collider::AABB(AABB4D::from_center_half_extents(position, half_extents)),
            body_type: BodyType::Dynamic,
            grounded: false,
            filter: CollisionFilter::default(),
            gravity_enabled: true, // Dynamic bodies have gravity by default
        }
    }

    /// Create a static body that doesn't move
    pub fn new_static_aabb(position: Vec4, half_extents: Vec4) -> Self {
        Self::new_aabb(position, half_extents).with_body_type(BodyType::Static)
    }

    /// Set the velocity of this body
    pub fn with_velocity(mut self, velocity: Vec4) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set the mass of this body
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    /// Set the physics material for this body
    pub fn with_material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }

    /// Set the restitution (bounciness) of this body
    ///
    /// This is a convenience method that updates the material's restitution.
    /// For full control over friction and restitution, use `with_material()`.
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.material.restitution = restitution.clamp(0.0, 1.0);
        self
    }

    /// Set the body type (Dynamic, Static, or Kinematic)
    ///
    /// Also sets `gravity_enabled` based on body type as a default:
    /// - Dynamic: gravity enabled
    /// - Static/Kinematic: gravity disabled
    ///
    /// To override gravity independently of body type, call
    /// `with_gravity()` after this method.
    pub fn with_body_type(mut self, body_type: BodyType) -> Self {
        self.body_type = body_type;
        self.gravity_enabled = body_type == BodyType::Dynamic;
        self
    }

    /// Set whether this body is affected by gravity
    ///
    /// This directly controls `gravity_enabled` without changing the body type.
    /// Use this to enable gravity on kinematic bodies (e.g., player characters
    /// that need gravity for jumping/falling) or to disable gravity on dynamic bodies.
    pub fn with_gravity(mut self, affected: bool) -> Self {
        self.gravity_enabled = affected;
        self
    }

    /// Set whether this body is static (legacy API)
    ///
    /// For new code, prefer `with_body_type(BodyType::Static)`.
    #[deprecated(since = "0.2.0", note = "Use with_body_type(BodyType::Static) instead")]
    pub fn with_static(mut self, is_static: bool) -> Self {
        if is_static {
            self.body_type = BodyType::Static;
            self.gravity_enabled = false;
        } else if self.body_type == BodyType::Static {
            self.body_type = BodyType::Dynamic;
            self.gravity_enabled = true;
        }
        self
    }

    /// Set the collision filter for this body
    pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the collision layer (which layer this body belongs to)
    pub fn with_layer(mut self, layer: crate::collision::CollisionLayer) -> Self {
        self.filter.layer = layer;
        self
    }

    /// Set the collision mask (which layers this body can collide with)
    pub fn with_mask(mut self, mask: crate::collision::CollisionLayer) -> Self {
        self.filter.mask = mask;
        self
    }

    /// Update the position and sync the collider
    pub fn set_position(&mut self, position: Vec4) {
        let delta = position - self.position;
        self.position = position;
        self.collider = self.collider.translated(delta);
    }

    /// Apply a positional correction (e.g., from collision resolution)
    pub fn apply_correction(&mut self, correction: Vec4) {
        self.position += correction;
        self.collider = self.collider.translated(correction);
    }
}

/// A collider that doesn't move (floors, walls, platforms)
///
/// Static colliders are checked for collision with all dynamic bodies
/// but never move themselves.
#[derive(Clone, Debug)]
pub struct StaticCollider {
    /// The collision shape
    pub collider: Collider,
    /// Physics material (friction and restitution)
    pub material: PhysicsMaterial,
    /// Collision filter (layer membership and collision mask)
    pub filter: CollisionFilter,
}

impl StaticCollider {
    /// Create a new static collider with the given shape and material
    pub fn new(collider: Collider, material: PhysicsMaterial) -> Self {
        Self {
            collider,
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a plane collider
    pub fn plane(normal: Vec4, distance: f32, material: PhysicsMaterial) -> Self {
        Self {
            collider: Collider::Plane(Plane4D::new(normal, distance)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a horizontal floor plane at the given Y height
    pub fn floor(y: f32, material: PhysicsMaterial) -> Self {
        Self {
            collider: Collider::Plane(Plane4D::floor(y)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a bounded floor platform using AABB collision
    ///
    /// Objects can fall off the edges of this platform.
    /// The floor surface is at Y height `y`, with the collider extending downward.
    ///
    /// # Parameters
    /// - `y`: Y height of floor surface (top of AABB)
    /// - `half_size_xz`: Half-extent in X and Z dimensions
    /// - `half_size_w`: Half-extent in W dimension
    /// - `thickness`: Thickness in Y (minimum 5.0 enforced to prevent tunneling)
    /// - `material`: Physics material for friction and restitution
    ///
    /// # Anti-tunneling
    /// The floor uses a minimum thickness of 5.0 units to prevent fast-moving objects
    /// from passing through while keeping the Y overlap small enough that collision
    /// resolution correctly pushes objects upward.
    pub fn floor_bounded(
        y: f32,
        half_size_xz: f32,
        half_size_w: f32,
        thickness: f32,
        material: PhysicsMaterial,
    ) -> Self {
        use crate::shapes::AABB4D;

        // Use reasonable thickness - enough to prevent tunneling but not so thick
        // that Y overlap equals X/Z overlap (which breaks collision axis selection)
        let actual_thickness = thickness.max(5.0);
        let half_thickness = actual_thickness / 2.0;

        // Position AABB so top surface is at y
        let center = Vec4::new(0.0, y - half_thickness, 0.0, 0.0);
        let half_extents = Vec4::new(half_size_xz, half_thickness, half_size_xz, half_size_w);

        Self {
            collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create an AABB static collider
    pub fn aabb(center: Vec4, half_extents: Vec4, material: PhysicsMaterial) -> Self {
        use crate::shapes::AABB4D;
        Self {
            collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Set the collision filter for this static collider
    pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Check if a position (ignoring Y) is within the XZW bounds of this collider
    ///
    /// This is used to detect when a player has walked off the edge of a bounded
    /// floor. If the player's XZW position is outside all floor bounds, they're
    /// in the void and should fall without colliding with floor edges.
    ///
    /// Returns `true` if the position is "over" this collider (within XZW bounds).
    /// For planes (infinite surfaces), this always returns `true`.
    pub fn is_position_over(&self, position: Vec4) -> bool {
        match &self.collider {
            Collider::AABB(aabb) => {
                position.x >= aabb.min.x
                    && position.x <= aabb.max.x
                    && position.z >= aabb.min.z
                    && position.z <= aabb.max.z
                    && position.w >= aabb.min.w
                    && position.w <= aabb.max.w
            }
            Collider::Plane(_) => true, // Infinite planes extend forever
            Collider::Sphere(_) => false, // Spheres aren't floor surfaces
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sphere_body() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let body = RigidBody4D::new_sphere(pos, 0.5);

        assert_eq!(body.position, pos);
        assert_eq!(body.velocity, Vec4::ZERO);
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.material, PhysicsMaterial::default());
        assert!(body.affected_by_gravity());
        assert!(!body.is_static());

        // Check collider is properly set
        assert_eq!(body.collider.center(), pos);
    }

    #[test]
    fn test_new_aabb_body() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let half_extents = Vec4::new(0.5, 1.0, 0.5, 0.5);
        let body = RigidBody4D::new_aabb(pos, half_extents);

        assert_eq!(body.position, pos);
        assert_eq!(body.collider.center(), pos);
    }

    #[test]
    fn test_static_body() {
        let pos = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let body = RigidBody4D::new_static_aabb(pos, Vec4::new(1.0, 1.0, 1.0, 1.0));

        assert!(body.is_static());
        assert!(!body.affected_by_gravity());
    }

    #[test]
    fn test_builder_methods() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_velocity(Vec4::new(1.0, 2.0, 0.0, 0.0))
            .with_mass(5.0)
            .with_restitution(0.8)
            .with_gravity(false);

        assert_eq!(body.velocity, Vec4::new(1.0, 2.0, 0.0, 0.0));
        assert_eq!(body.mass, 5.0);
        assert_eq!(body.material.restitution, 0.8);
        assert!(!body.affected_by_gravity());
    }

    #[test]
    fn test_restitution_clamping() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(1.5);
        assert_eq!(body.material.restitution, 1.0);

        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(-0.5);
        assert_eq!(body.material.restitution, 0.0);
    }

    #[test]
    fn test_with_material() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_material(PhysicsMaterial::RUBBER);

        assert_eq!(body.material, PhysicsMaterial::RUBBER);
    }

    #[test]
    fn test_set_position() {
        let mut body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0);
        let new_pos = Vec4::new(5.0, 10.0, 3.0, 0.0);

        body.set_position(new_pos);

        assert_eq!(body.position, new_pos);
        assert_eq!(body.collider.center(), new_pos);
    }

    #[test]
    fn test_apply_correction() {
        let mut body = RigidBody4D::new_sphere(Vec4::new(1.0, 0.0, 0.0, 0.0), 1.0);
        let correction = Vec4::new(0.0, 0.5, 0.0, 0.0);

        body.apply_correction(correction);

        assert_eq!(body.position, Vec4::new(1.0, 0.5, 0.0, 0.0));
        assert_eq!(body.collider.center(), Vec4::new(1.0, 0.5, 0.0, 0.0));
    }

    #[test]
    #[allow(deprecated)]
    fn test_with_static_disables_gravity() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_gravity(true)
            .with_static(true);

        assert!(body.is_static());
        assert!(!body.affected_by_gravity());
    }

    // ===== Collision Filter Tests =====

    #[test]
    fn test_default_filter() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0);
        assert_eq!(body.filter, CollisionFilter::default());
    }

    #[test]
    fn test_with_filter() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_filter(CollisionFilter::player());

        assert_eq!(body.filter.layer, CollisionLayer::PLAYER);
    }

    #[test]
    fn test_with_layer() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_layer(CollisionLayer::ENEMY);

        assert_eq!(body.filter.layer, CollisionLayer::ENEMY);
    }

    #[test]
    fn test_with_mask() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_mask(CollisionLayer::STATIC | CollisionLayer::ENEMY);

        assert!(body.filter.mask.contains(CollisionLayer::STATIC));
        assert!(body.filter.mask.contains(CollisionLayer::ENEMY));
        assert!(!body.filter.mask.contains(CollisionLayer::PLAYER));
    }

    #[test]
    fn test_static_collider_default_filter() {
        let collider = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE);
        assert_eq!(collider.filter, CollisionFilter::static_world());
    }

    #[test]
    fn test_static_collider_with_filter() {
        use crate::collision::CollisionLayer;
        let collider = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE)
            .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

        assert_eq!(collider.filter.layer, CollisionLayer::TRIGGER);
    }

    // ===== Bounded Floor Tests =====

    #[test]
    fn test_floor_bounded_creates_aabb() {
        use crate::shapes::Collider;
        let collider = StaticCollider::floor_bounded(
            0.0,  // y: floor surface at y=0
            10.0, // half_size_xz
            5.0,  // half_size_w
            1.0,  // thickness (clamped to minimum 5.0)
            PhysicsMaterial::CONCRETE,
        );

        // Should be an AABB, not a plane
        match &collider.collider {
            Collider::AABB(aabb) => {
                // Top surface should be at y=0
                assert_eq!(aabb.max.y, 0.0);
                // Bottom extends 5 units down (minimum thickness)
                assert_eq!(aabb.min.y, -5.0);
                // X/Z extents should be -10 to +10
                assert_eq!(aabb.min.x, -10.0);
                assert_eq!(aabb.max.x, 10.0);
                assert_eq!(aabb.min.z, -10.0);
                assert_eq!(aabb.max.z, 10.0);
                // W extent should be -5 to +5
                assert_eq!(aabb.min.w, -5.0);
                assert_eq!(aabb.max.w, 5.0);
            }
            _ => panic!("Expected AABB collider from floor_bounded"),
        }

        assert_eq!(collider.filter, CollisionFilter::static_world());
    }

    #[test]
    fn test_floor_bounded_minimum_thickness() {
        use crate::shapes::Collider;
        // Thin thickness is clamped to minimum 5.0
        let collider = StaticCollider::floor_bounded(
            5.0,  // y: floor surface at y=5
            1.0,  // half_size_xz
            1.0,  // half_size_w
            0.01, // thickness (clamped to 5.0)
            PhysicsMaterial::RUBBER,
        );

        match &collider.collider {
            Collider::AABB(aabb) => {
                // Top surface at y=5
                assert_eq!(aabb.max.y, 5.0);
                // Bottom at y=0 (5 units down from surface)
                assert_eq!(aabb.min.y, 0.0);
            }
            _ => panic!("Expected AABB collider"),
        }
    }

    #[test]
    fn test_floor_bounded_custom_thickness() {
        use crate::shapes::Collider;
        // Can specify larger thickness
        let collider = StaticCollider::floor_bounded(
            0.0,  // y: floor surface at y=0
            10.0, // half_size_xz
            5.0,  // half_size_w
            20.0, // thickness (larger than minimum)
            PhysicsMaterial::CONCRETE,
        );

        match &collider.collider {
            Collider::AABB(aabb) => {
                assert_eq!(aabb.max.y, 0.0);
                assert_eq!(aabb.min.y, -20.0);
            }
            _ => panic!("Expected AABB collider"),
        }
    }

    #[test]
    fn test_floor_bounded_collision_with_sphere() {
        use crate::collision::sphere_vs_aabb;
        use crate::shapes::{Collider, Sphere4D};

        // Values from default.ron scene
        let collider = StaticCollider::floor_bounded(
            -2.0,  // y: floor surface at y=-2
            10.0,  // half_size_xz
            5.0,   // half_size_w
            0.001, // thickness (clamped to minimum 5.0)
            PhysicsMaterial::CONCRETE,
        );

        let aabb = match &collider.collider {
            Collider::AABB(aabb) => aabb,
            _ => panic!("Expected AABB"),
        };

        // Verify floor bounds
        assert_eq!(aabb.max.y, -2.0, "Floor top should be at y=-2");
        assert_eq!(
            aabb.min.y, -7.0,
            "Floor bottom should extend 5 units down (minimum)"
        );

        // Player spawn at (0, 0, 5, 0) with radius 0.5
        let player_radius = 0.5;

        // Player at spawn position (above floor) - should NOT collide
        let player_above = Sphere4D::new(Vec4::new(0.0, 0.0, 5.0, 0.0), player_radius);
        assert!(
            sphere_vs_aabb(&player_above, aabb).is_none(),
            "Player at spawn should not collide"
        );

        // Player fallen to slightly penetrating floor (center at y = -2 + 0.5 - 0.1 = -1.6)
        // This is 0.1 units below the tangent point
        let player_penetrating_slight =
            Sphere4D::new(Vec4::new(0.0, -1.6, 5.0, 0.0), player_radius);
        let contact = sphere_vs_aabb(&player_penetrating_slight, aabb);
        assert!(contact.is_some(), "Player penetrating floor should collide");

        // Player outside X/Z bounds - should NOT collide (can fall off edge)
        let player_off_edge_xz = Sphere4D::new(Vec4::new(15.0, -1.6, 5.0, 0.0), player_radius);
        assert!(
            sphere_vs_aabb(&player_off_edge_xz, aabb).is_none(),
            "Player off X edge should not collide"
        );

        // Player outside W bounds - should NOT collide (can fall off W edge)
        // Floor W extent is -5 to +5, so W=10 is outside
        let player_off_edge_w = Sphere4D::new(Vec4::new(0.0, -1.6, 5.0, 10.0), player_radius);
        assert!(
            sphere_vs_aabb(&player_off_edge_w, aabb).is_none(),
            "Player off W edge should not collide"
        );
    }

    #[test]
    fn test_floor_bounded_4d_edges() {
        use crate::collision::sphere_vs_aabb;
        use crate::shapes::{Collider, Sphere4D};

        let collider = StaticCollider::floor_bounded(
            -2.0, // y: floor surface
            10.0, // half_size_xz (X/Z from -10 to +10)
            5.0,  // half_size_w (W from -5 to +5)
            5.0,  // thickness
            PhysicsMaterial::CONCRETE,
        );

        let aabb = match &collider.collider {
            Collider::AABB(aabb) => aabb,
            _ => panic!("Expected AABB"),
        };

        let radius = 0.5;
        let y_on_floor = -1.6; // Penetrating floor surface

        // On floor at center - SHOULD collide
        let on_floor = Sphere4D::new(Vec4::new(0.0, y_on_floor, 0.0, 0.0), radius);
        assert!(
            sphere_vs_aabb(&on_floor, aabb).is_some(),
            "Center should collide"
        );

        // On floor at W=-4 (inside W bounds) - SHOULD collide
        let inside_w = Sphere4D::new(Vec4::new(0.0, y_on_floor, 0.0, -4.0), radius);
        assert!(
            sphere_vs_aabb(&inside_w, aabb).is_some(),
            "Inside W bounds should collide"
        );

        // Off floor at W=6 (outside W bounds) - should NOT collide
        let outside_w_pos = Sphere4D::new(Vec4::new(0.0, y_on_floor, 0.0, 6.0), radius);
        assert!(
            sphere_vs_aabb(&outside_w_pos, aabb).is_none(),
            "Outside +W should not collide"
        );

        // Off floor at W=-6 (outside W bounds) - should NOT collide
        let outside_w_neg = Sphere4D::new(Vec4::new(0.0, y_on_floor, 0.0, -6.0), radius);
        assert!(
            sphere_vs_aabb(&outside_w_neg, aabb).is_none(),
            "Outside -W should not collide"
        );

        // Off floor at X=12 - should NOT collide
        let outside_x = Sphere4D::new(Vec4::new(12.0, y_on_floor, 0.0, 0.0), radius);
        assert!(
            sphere_vs_aabb(&outside_x, aabb).is_none(),
            "Outside X should not collide"
        );
    }

    // ===== is_position_over Tests =====

    #[test]
    fn test_is_position_over_aabb_inside() {
        let floor = StaticCollider::floor_bounded(
            0.0,  // y
            10.0, // half_size_xz (X/Z: -10 to +10)
            5.0,  // half_size_w (W: -5 to +5)
            5.0,  // thickness
            PhysicsMaterial::CONCRETE,
        );

        // Position in center (any Y value)
        assert!(floor.is_position_over(Vec4::new(0.0, 0.0, 0.0, 0.0)));
        assert!(floor.is_position_over(Vec4::new(0.0, 100.0, 0.0, 0.0)));
        assert!(floor.is_position_over(Vec4::new(0.0, -100.0, 0.0, 0.0)));

        // Position near edges but inside
        assert!(floor.is_position_over(Vec4::new(9.0, 0.0, 9.0, 4.0)));
        assert!(floor.is_position_over(Vec4::new(-9.0, 0.0, -9.0, -4.0)));
    }

    #[test]
    fn test_is_position_over_aabb_outside() {
        let floor = StaticCollider::floor_bounded(
            0.0,  // y
            10.0, // half_size_xz (X/Z: -10 to +10)
            5.0,  // half_size_w (W: -5 to +5)
            5.0,  // thickness
            PhysicsMaterial::CONCRETE,
        );

        // Outside X bounds
        assert!(!floor.is_position_over(Vec4::new(11.0, 0.0, 0.0, 0.0)));
        assert!(!floor.is_position_over(Vec4::new(-11.0, 0.0, 0.0, 0.0)));

        // Outside Z bounds
        assert!(!floor.is_position_over(Vec4::new(0.0, 0.0, 11.0, 0.0)));
        assert!(!floor.is_position_over(Vec4::new(0.0, 0.0, -11.0, 0.0)));

        // Outside W bounds
        assert!(!floor.is_position_over(Vec4::new(0.0, 0.0, 0.0, 6.0)));
        assert!(!floor.is_position_over(Vec4::new(0.0, 0.0, 0.0, -6.0)));

        // Outside multiple bounds
        assert!(!floor.is_position_over(Vec4::new(15.0, 0.0, 15.0, 10.0)));
    }

    #[test]
    fn test_is_position_over_plane() {
        let floor = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE);

        // Planes extend infinitely - always returns true
        assert!(floor.is_position_over(Vec4::new(0.0, 0.0, 0.0, 0.0)));
        assert!(floor.is_position_over(Vec4::new(1000.0, 0.0, 1000.0, 1000.0)));
        assert!(floor.is_position_over(Vec4::new(-1000.0, 0.0, -1000.0, -1000.0)));
    }

    #[test]
    fn test_is_position_over_ignores_y() {
        let floor = StaticCollider::floor_bounded(
            0.0,  // y: floor at y=0
            10.0, // half_size_xz
            5.0,  // half_size_w
            5.0,  // thickness (floor AABB: y from -5 to 0)
            PhysicsMaterial::CONCRETE,
        );

        // Position inside XZW bounds but far above floor
        assert!(floor.is_position_over(Vec4::new(0.0, 1000.0, 0.0, 0.0)));

        // Position inside XZW bounds but far below floor
        assert!(floor.is_position_over(Vec4::new(0.0, -1000.0, 0.0, 0.0)));

        // Position outside W bounds - Y doesn't matter
        assert!(!floor.is_position_over(Vec4::new(0.0, 0.0, 0.0, 10.0)));
        assert!(!floor.is_position_over(Vec4::new(0.0, 1000.0, 0.0, 10.0)));
    }
}
