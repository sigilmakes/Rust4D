//! Integration tests for game logic layer (CharacterController4D + ActiveScene)
//!
//! These tests verify the integration between rust4d_game and rust4d_core,
//! specifically the CharacterController4D working with ActiveScene and scene_helpers.

use rust4d_core::{ActiveScene, EntityTemplate, Material, Scene, ShapeTemplate, Transform4D};
use rust4d_game::{scene_helpers, CharacterConfig, CharacterController4D};
use rust4d_math::Vec4;

/// T1: Integration test between CharacterController4D and ActiveScene
///
/// Tests the full pipeline: instantiate scene, create player body via scene_helpers,
/// wrap in CharacterController4D, drive movement through it.
#[test]
fn test_character_controller_with_active_scene() {
    // Create a scene with a floor and player spawn
    let mut scene = Scene::new("Controller Test")
        .with_gravity(-20.0)
        .with_player_spawn(0.0, 1.0, 5.0, 0.0);

    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::hyperplane(-2.0, 10.0, 10, 5.0, 0.001),
            Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0)),
            Material::GRAY,
        )
        .with_name("floor")
        .with_tag("static"),
    );

    // Instantiate scene (player body creation is at the app layer)
    let mut active = ActiveScene::from_template(&scene, None);

    // Create player body using scene_helpers (the single source of truth)
    let spawn_pos = Vec4::new(0.0, 1.0, 5.0, 0.0);
    let player_key = {
        let physics = active
            .world
            .physics_mut()
            .expect("Scene should have physics");
        scene_helpers::create_player_body(physics, spawn_pos, 0.5)
    };
    active.player_body_key = Some(player_key);

    // Create CharacterController4D from the player body key
    let controller = CharacterController4D::new(
        player_key,
        CharacterConfig {
            move_speed: 5.0,
            w_move_speed: 5.0,
            jump_velocity: 10.0,
        },
    );

    // Verify initial position
    {
        let physics = active.world.physics().unwrap();
        let pos = controller
            .position(physics)
            .expect("Player body should exist");
        assert_eq!(pos, spawn_pos, "Initial position should match spawn");
    }

    // Step physics to let player fall and settle
    for _ in 0..100 {
        if let Some(physics) = active.world.physics_mut() {
            controller.apply_movement(physics, Vec4::ZERO, Vec4::ZERO);
        }
        active.update(1.0 / 60.0);
    }

    // Player should be grounded
    {
        let physics = active.world.physics().unwrap();
        assert!(
            controller.is_grounded(physics),
            "Player should be grounded after settling"
        );
    }

    // Apply movement in X direction
    if let Some(physics) = active.world.physics_mut() {
        controller.apply_movement(physics, Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::ZERO);
    }
    active.update(1.0 / 60.0);

    // Position X should have changed
    {
        let physics = active.world.physics().unwrap();
        let new_pos = controller.position(physics).unwrap();
        assert!(
            (new_pos.x - spawn_pos.x).abs() > 0.01,
            "X should change after movement. Got: {}",
            new_pos.x
        );
    }
}
