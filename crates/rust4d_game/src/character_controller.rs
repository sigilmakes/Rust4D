//! Character controller for 4D first-person movement
//!
//! Wraps a physics body key and provides high-level movement operations.
//! Does NOT own the PhysicsWorld -- takes `&PhysicsWorld` or `&mut PhysicsWorld`
//! as method parameters.

use rust4d_math::Vec4;
use rust4d_physics::{BodyKey, PhysicsWorld};

/// Configuration for a character controller
#[derive(Clone, Debug)]
pub struct CharacterConfig {
    /// Movement speed multiplier (XZ axes)
    pub move_speed: f32,
    /// Movement speed multiplier for the W axis
    pub w_move_speed: f32,
    /// Jump velocity (upward Y velocity when jumping)
    pub jump_velocity: f32,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            move_speed: 3.0,
            w_move_speed: 3.0,
            jump_velocity: 8.0,
        }
    }
}

/// First-person character controller for 4D space
///
/// Wraps a physics body key and provides high-level movement operations.
/// Does NOT own the PhysicsWorld -- takes `&mut PhysicsWorld` as parameters.
pub struct CharacterController4D {
    /// The physics body this controller manages
    body_key: BodyKey,
    /// Movement configuration
    pub config: CharacterConfig,
}

impl CharacterController4D {
    /// Create a new character controller for the given body key
    pub fn new(body_key: BodyKey, config: CharacterConfig) -> Self {
        Self { body_key, config }
    }

    /// Get the body key this controller manages
    pub fn body_key(&self) -> BodyKey {
        self.body_key
    }

    /// Apply movement as two semantic components (preserves Y for gravity/jumping)
    ///
    /// * `slice_dir` — movement within the player's current 3D slice (WASD).
    ///   In world space this direction is orthogonal to the camera's ana axis;
    ///   it is scaled uniformly by `config.move_speed`.
    /// * `ana_dir` — deliberate movement along the camera's ana axis (Q/E),
    ///   scaled uniformly by `config.w_move_speed`.
    ///
    /// # Why two parameters
    ///
    /// Speeds must scale each *semantic input* uniformly, never individual
    /// world axes. The old single-vector API scaled X/Z by `move_speed` and W
    /// by `w_move_speed` in world space; after a 4D camera rotation the WASD
    /// direction mixes world Z and W, so anisotropic world-axis scaling tilted
    /// the velocity across the slice plane. The camera then drifted along its
    /// ana axis while walking, and every cross-section on screen morphed
    /// (the "4D movement bug", see `tests/slice_invariant.rs`).
    pub fn apply_movement(&self, physics: &mut PhysicsWorld, slice_dir: Vec4, ana_dir: Vec4) {
        let velocity = slice_dir * self.config.move_speed + ana_dir * self.config.w_move_speed;
        physics.apply_body_movement(self.body_key, velocity);
    }

    /// Attempt to jump. Returns true if successful (only works when grounded).
    pub fn jump(&self, physics: &mut PhysicsWorld) -> bool {
        physics.body_jump(self.body_key, self.config.jump_velocity)
    }

    /// Check if the character is grounded
    pub fn is_grounded(&self, physics: &PhysicsWorld) -> bool {
        physics.body_is_grounded(self.body_key)
    }

    /// Get the character's current position
    pub fn position(&self, physics: &PhysicsWorld) -> Option<Vec4> {
        physics.body_position(self.body_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_physics::{PhysicsConfig, RigidBody4D, BodyType, PhysicsMaterial, StaticCollider};

    /// Create a test physics world with a floor at y=0
    fn test_world_with_floor() -> PhysicsWorld {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));
        physics.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));
        physics
    }

    /// Create a kinematic player body (typical character setup)
    fn create_player_body(physics: &mut PhysicsWorld, position: Vec4) -> BodyKey {
        let body = RigidBody4D::new_sphere(position, 0.5)
            .with_body_type(BodyType::Kinematic)
            .with_gravity(true)
            .with_mass(1.0)
            .with_material(PhysicsMaterial::WOOD);
        physics.add_body(body)
    }

    #[test]
    fn test_construction_and_config() {
        let mut physics = PhysicsWorld::new();
        let body_key = create_player_body(&mut physics, Vec4::new(0.0, 1.0, 0.0, 0.0));

        let config = CharacterConfig {
            move_speed: 5.0,
            w_move_speed: 4.0,
            jump_velocity: 10.0,
        };
        let controller = CharacterController4D::new(body_key, config.clone());

        assert_eq!(controller.body_key(), body_key);
        assert_eq!(controller.config.move_speed, 5.0);
        assert_eq!(controller.config.w_move_speed, 4.0);
        assert_eq!(controller.config.jump_velocity, 10.0);
    }

    #[test]
    fn test_default_config() {
        let config = CharacterConfig::default();
        assert_eq!(config.move_speed, 3.0);
        assert_eq!(config.w_move_speed, 3.0);
        assert_eq!(config.jump_velocity, 8.0);
    }

    #[test]
    fn test_apply_movement_delegates_to_physics() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity
        let body_key = physics.add_body(
            RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
                .with_body_type(BodyType::Kinematic),
        );

        let config = CharacterConfig {
            move_speed: 2.0,
            w_move_speed: 2.0,
            jump_velocity: 8.0,
        };
        let controller = CharacterController4D::new(body_key, config);

        // Apply movement (1, 0, 1, 0) with speed 2.0 => physics gets (2, 0, 2, 0)
        controller.apply_movement(&mut physics, Vec4::new(1.0, 0.0, 1.0, 0.0), Vec4::ZERO);

        // Step physics and check position changed
        physics.step(1.0);
        let pos = physics.body_position(body_key).unwrap();
        assert!((pos.x - 2.0).abs() < 0.01, "Expected x=2.0, got {}", pos.x);
        assert!((pos.z - 2.0).abs() < 0.01, "Expected z=2.0, got {}", pos.z);
    }

    #[test]
    fn test_apply_movement_uses_w_move_speed_for_ana_component() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity
        let body_key = physics.add_body(
            RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
                .with_body_type(BodyType::Kinematic),
        );

        let config = CharacterConfig {
            move_speed: 2.0,
            w_move_speed: 5.0,
            jump_velocity: 8.0,
        };
        let controller = CharacterController4D::new(body_key, config);

        // Slice movement uses move_speed (2.0), ana movement uses w_move_speed (5.0)
        controller.apply_movement(
            &mut physics,
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        physics.step(1.0);
        let pos = physics.body_position(body_key).unwrap();
        assert!((pos.x - 2.0).abs() < 0.01, "Expected x=2.0 (move_speed), got {}", pos.x);
        assert!((pos.w - 5.0).abs() < 0.01, "Expected w=5.0 (w_move_speed), got {}", pos.w);
    }

    #[test]
    fn test_apply_movement_preserves_slice_direction_after_4d_rotation() {
        // THE regression test for the 4D movement bug: a slice-movement
        // direction that is orthogonal to the camera's ana axis must remain
        // orthogonal after speed scaling, for ANY speed configuration.
        // World-axis anisotropic scaling violated this.
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(0.0));
        let body_key = physics.add_body(
            RigidBody4D::new_sphere(Vec4::ZERO, 0.5).with_body_type(BodyType::Kinematic),
        );
        let config = CharacterConfig {
            move_speed: 3.0,
            w_move_speed: 2.0, // anisotropic, like config/default.toml
            jump_velocity: 8.0,
        };
        let controller = CharacterController4D::new(body_key, config);

        // Camera rotated 45° in ZW: forward and ana both mix Z and W
        let theta = std::f32::consts::FRAC_PI_4;
        let forward = Vec4::new(0.0, 0.0, -theta.cos(), -theta.sin());
        let ana = Vec4::new(0.0, 0.0, -theta.sin(), theta.cos());
        assert!(forward.dot(ana).abs() < 1e-6, "test setup: forward ⊥ ana");

        controller.apply_movement(&mut physics, forward, Vec4::ZERO);
        physics.step(1.0);
        let pos = physics.body_position(body_key).unwrap();

        // The displacement must stay orthogonal to ana — zero drift across
        // the slice plane.
        let drift = pos.dot(ana);
        assert!(
            drift.abs() < 1e-4,
            "slice movement drifted {drift} along the ana axis — shapes would morph"
        );
    }

    #[test]
    fn test_jump_only_when_grounded() {
        let mut physics = test_world_with_floor();
        // Place body slightly penetrating floor so it gets grounded
        let body_key = create_player_body(&mut physics, Vec4::new(0.0, 0.4, 0.0, 0.0));

        let controller = CharacterController4D::new(body_key, CharacterConfig::default());

        // Step to get grounded
        physics.step(0.016);
        assert!(controller.is_grounded(&physics), "Should be grounded after step near floor");

        // Jump should succeed
        let jumped = controller.jump(&mut physics);
        assert!(jumped, "Jump should succeed when grounded");
        assert!(!controller.is_grounded(&physics), "Should not be grounded after jump");
    }

    #[test]
    fn test_jump_fails_when_airborne() {
        let mut physics = PhysicsWorld::new();
        // Body high in the air (no floor)
        let body_key = create_player_body(&mut physics, Vec4::new(0.0, 10.0, 0.0, 0.0));

        let controller = CharacterController4D::new(body_key, CharacterConfig::default());

        assert!(!controller.is_grounded(&physics), "Should not be grounded in air");

        let jumped = controller.jump(&mut physics);
        assert!(!jumped, "Jump should fail when airborne");
    }

    #[test]
    fn test_is_grounded_returns_correct_state() {
        let mut physics = test_world_with_floor();
        let body_key = create_player_body(&mut physics, Vec4::new(0.0, 0.4, 0.0, 0.0));

        let controller = CharacterController4D::new(body_key, CharacterConfig::default());

        // Before step: not grounded (grounded resets each step)
        assert!(!controller.is_grounded(&physics));

        // After step near floor: grounded
        physics.step(0.016);
        assert!(controller.is_grounded(&physics));
    }

    #[test]
    fn test_position_returns_body_position() {
        let mut physics = PhysicsWorld::new();
        let start_pos = Vec4::new(5.0, 2.0, 3.0, 1.0);
        let body_key = create_player_body(&mut physics, start_pos);

        let controller = CharacterController4D::new(body_key, CharacterConfig::default());

        let pos = controller.position(&physics);
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, start_pos.x);
        assert_eq!(pos.y, start_pos.y);
        assert_eq!(pos.z, start_pos.z);
        assert_eq!(pos.w, start_pos.w);
    }

    #[test]
    fn test_jump_uses_config_velocity() {
        let mut physics = PhysicsWorld::new();
        let mut body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Kinematic);
        body.grounded = true; // Manually set grounded for test
        let body_key = physics.add_body(body);

        let config = CharacterConfig {
            move_speed: 3.0,
            w_move_speed: 3.0,
            jump_velocity: 12.0,
        };
        let controller = CharacterController4D::new(body_key, config);

        controller.jump(&mut physics);

        let vel = physics.get_body(body_key).unwrap().velocity;
        assert_eq!(vel.y, 12.0, "Jump should use config.jump_velocity");
    }

    #[test]
    fn test_stale_body_key_returns_none() {
        let mut physics = PhysicsWorld::new();
        let body_key = create_player_body(&mut physics, Vec4::new(0.0, 1.0, 0.0, 0.0));
        let controller = CharacterController4D::new(body_key, CharacterConfig::default());

        // Remove the body from the physics world
        physics.remove_body(body_key);

        // Position should return None for stale key
        assert!(
            controller.position(&physics).is_none(),
            "Position should return None for removed body"
        );

        // is_grounded should return false for stale key
        assert!(
            !controller.is_grounded(&physics),
            "is_grounded should return false for removed body"
        );

        // jump should return false for stale key
        assert!(
            !controller.jump(&mut physics),
            "jump should return false for removed body"
        );

        // apply_movement should not panic for stale key
        controller.apply_movement(&mut physics, Vec4::new(1.0, 0.0, 1.0, 0.0), Vec4::ZERO);
    }

    #[test]
    fn test_apply_movement_with_zero_vector() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity
        let body_key = physics.add_body(
            RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
                .with_body_type(BodyType::Kinematic),
        );

        let controller = CharacterController4D::new(
            body_key,
            CharacterConfig { move_speed: 5.0, w_move_speed: 5.0, jump_velocity: 8.0 },
        );

        // First apply some movement
        controller.apply_movement(&mut physics, Vec4::new(1.0, 0.0, 1.0, 0.0), Vec4::ZERO);
        physics.step(0.016);

        // Now apply zero movement -- should stop horizontal movement
        controller.apply_movement(&mut physics, Vec4::ZERO, Vec4::ZERO);
        physics.step(0.016);

        let body = physics.get_body(body_key).unwrap();
        // Horizontal velocity should be zero (XZW)
        assert!(
            body.velocity.x.abs() < 0.001,
            "X velocity should be ~0 after zero movement, got {}",
            body.velocity.x
        );
        assert!(
            body.velocity.z.abs() < 0.001,
            "Z velocity should be ~0 after zero movement, got {}",
            body.velocity.z
        );
        assert!(
            body.velocity.w.abs() < 0.001,
            "W velocity should be ~0 after zero movement, got {}",
            body.velocity.w
        );
    }
}
