//! Scene helpers for player body creation
//!
//! Provides the canonical player body setup used by the application layer
//! after scene instantiation. `ActiveScene::from_template()` handles static
//! colliders and dynamic bodies; this module handles the player body.

use rust4d_core::Scene;
use rust4d_math::Vec4;
use rust4d_physics::{BodyKey, BodyType, PhysicsMaterial, PhysicsWorld, RigidBody4D};

/// Find the player spawn position from a scene template
///
/// Returns the player spawn position as a `Vec4`, or `None` if no spawn is defined.
pub fn find_player_spawn(scene: &Scene) -> Option<Vec4> {
    scene
        .player_spawn
        .map(|s| Vec4::new(s[0], s[1], s[2], s[3]))
}

/// Create a player body in the physics world
///
/// Creates a kinematic sphere body with gravity enabled (for jumping/falling),
/// using the WOOD physics material. This is the single source of truth for
/// player body creation -- called from the application layer after scene
/// instantiation.
///
/// # Parameters
/// - `physics`: The physics world to add the body to
/// - `position`: Spawn position in 4D space
/// - `radius`: Collision sphere radius
///
/// # Returns
/// The `BodyKey` for the newly created player body.
pub fn create_player_body(physics: &mut PhysicsWorld, position: Vec4, radius: f32) -> BodyKey {
    let body = RigidBody4D::new_sphere(position, radius)
        .with_body_type(BodyType::Kinematic)
        .with_gravity(true) // Kinematic but needs gravity for jumping/falling
        .with_mass(1.0)
        .with_material(PhysicsMaterial::WOOD);
    physics.add_body(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_physics::PhysicsConfig;

    #[test]
    fn test_find_player_spawn_some() {
        let scene = Scene::new("Test").with_player_spawn(1.0, 2.0, 3.0, 4.0);

        let spawn = find_player_spawn(&scene);
        assert!(spawn.is_some());
        let pos = spawn.unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
        assert_eq!(pos.z, 3.0);
        assert_eq!(pos.w, 4.0);
    }

    #[test]
    fn test_find_player_spawn_none() {
        let scene = Scene::new("Test");
        assert!(find_player_spawn(&scene).is_none());
    }

    #[test]
    fn test_create_player_body() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

        let position = Vec4::new(0.0, 2.0, 5.0, 0.0);
        let radius = 0.5;
        let key = create_player_body(&mut physics, position, radius);

        // Body should exist
        let body = physics.get_body(key);
        assert!(body.is_some());

        let body = body.unwrap();
        assert_eq!(body.position, position);
        assert_eq!(body.body_type, BodyType::Kinematic);
        assert!(body.gravity_enabled);
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.material, PhysicsMaterial::WOOD);
    }

    #[test]
    fn test_create_player_body_is_kinematic_with_gravity() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

        let key = create_player_body(&mut physics, Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let body = physics.get_body(key).unwrap();

        // Should be kinematic (user-controlled velocity)
        assert!(body.is_kinematic());
        assert!(!body.is_static());

        // But gravity should be enabled (for jumping/falling)
        assert!(body.affected_by_gravity());
    }

    #[test]
    fn test_create_player_body_falls_with_gravity() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

        let key = create_player_body(&mut physics, Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5);

        // Step physics - body should fall
        physics.step(0.1);

        let body = physics.get_body(key).unwrap();
        assert!(
            body.position.y < 10.0,
            "Player body should fall with gravity"
        );
        assert!(
            body.velocity.y < 0.0,
            "Player body should have downward velocity"
        );
    }

    #[test]
    fn test_player_body_key_is_valid() {
        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

        let key = create_player_body(&mut physics, Vec4::ZERO, 0.5);

        // Can use generic body methods with the key
        assert_eq!(physics.body_position(key), Some(Vec4::ZERO));
        assert!(!physics.body_is_grounded(key));
    }

    #[test]
    fn test_find_player_spawn_with_zeros() {
        let scene = Scene::new("Test").with_player_spawn(0.0, 0.0, 0.0, 0.0);

        let spawn = find_player_spawn(&scene);
        assert!(spawn.is_some());
        let pos = spawn.unwrap();
        assert_eq!(pos, Vec4::ZERO);
    }

    /// T5: create_player_body + CharacterController4D round-trip
    #[test]
    fn test_create_player_body_with_character_controller_round_trip() {
        use crate::{CharacterConfig, CharacterController4D};
        use rust4d_physics::StaticCollider;

        let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));
        physics.add_static_collider(StaticCollider::floor(
            0.0,
            rust4d_physics::PhysicsMaterial::CONCRETE,
        ));

        // Create player body via scene_helpers
        let spawn = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let key = create_player_body(&mut physics, spawn, 0.5);

        // Wrap in CharacterController4D
        let controller = CharacterController4D::new(
            key,
            CharacterConfig {
                move_speed: 5.0,
                w_move_speed: 5.0,
                jump_velocity: 10.0,
            },
        );

        // Verify initial position matches spawn
        let pos = controller.position(&physics).expect("Body should exist");
        assert_eq!(pos, spawn, "Initial position should match spawn");

        // Apply movement and step
        controller.apply_movement(&mut physics, Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::ZERO);
        physics.step(0.016);

        // Position should have changed
        let pos_after = controller
            .position(&physics)
            .expect("Body should still exist");
        assert!(
            (pos_after.x - spawn.x).abs() > 0.01,
            "X position should change after movement. Before: {}, After: {}",
            spawn.x,
            pos_after.x
        );

        // Step physics until grounded
        for _ in 0..100 {
            controller.apply_movement(&mut physics, Vec4::ZERO, Vec4::ZERO);
            physics.step(1.0 / 60.0);
        }

        // Should be grounded after falling
        assert!(
            controller.is_grounded(&physics),
            "Controller should report grounded after settling"
        );

        // Jump should succeed when grounded
        let jumped = controller.jump(&mut physics);
        assert!(jumped, "Jump should succeed when grounded");
        assert!(
            !controller.is_grounded(&physics),
            "Should not be grounded after jump"
        );
    }
}
