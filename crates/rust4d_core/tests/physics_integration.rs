//! Integration tests for physics pipeline
//!
//! These tests verify the full scene-physics-render pipeline works correctly:
//! 1. Scene loading creates correct physics bodies
//! 2. Physics simulation applies gravity and collision
//! 3. Entity transforms sync from physics bodies
//! 4. Dirty flags trigger geometry rebuild

use rust4d_core::{
    ActiveScene, DirtyFlags, EntityTemplate, Material, Name, PhysicsBody, Scene, ShapeRef,
    ShapeTemplate, Transform4D, World,
};
use rust4d_math::{Tesseract4D, Vec4};
use rust4d_physics::{
    BodyType, PhysicsConfig, PhysicsMaterial, PhysicsWorld, RigidBody4D, StaticCollider,
};

// ==================== Scene Loading Tests ====================

/// Test that dynamic entities get physics bodies created
#[test]
fn test_scene_dynamic_entity_has_physics_body() {
    // Create a scene with a dynamic entity
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(0.0, 0.0, 0.0, 0.0)),
            Material::WHITE,
        )
        .with_name("tesseract")
        .with_tag("dynamic"),
    );

    // Instantiate the scene
    let active = ActiveScene::from_template(&scene, None);

    // Get the entity
    let entity_handle = active
        .world
        .get_by_name("tesseract")
        .expect("Tesseract entity should exist");

    // Verify physics body was created
    let body_comp = active.world.ecs().get::<&PhysicsBody>(entity_handle);
    assert!(
        body_comp.is_ok(),
        "Dynamic entity should have a physics body"
    );

    // Verify the body exists in the physics world
    let physics = active.world.physics().expect("World should have physics");
    let body_key = body_comp.unwrap().0;
    let body = physics
        .get_body(body_key)
        .expect("Physics body should exist");

    // Verify body type is Dynamic
    assert!(!body.is_static(), "Body should not be static");
    assert!(
        body.affected_by_gravity(),
        "Dynamic body should be affected by gravity"
    );
}

/// Test that static floors get colliders created
#[test]
fn test_scene_static_floor_has_collider() {
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::hyperplane(-2.0, 10.0, 10, 5.0, 0.001),
            Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0)),
            Material::GRAY,
        )
        .with_name("floor")
        .with_tag("static"),
    );

    let active = ActiveScene::from_template(&scene, None);

    // Verify static collider was created
    let physics = active.world.physics().expect("World should have physics");
    assert!(
        !physics.static_colliders().is_empty(),
        "Static floor should create a collider"
    );
}

// ==================== Physics Simulation Tests ====================

/// Test that a dynamic body falls under gravity
#[test]
fn test_dynamic_body_falls_under_gravity() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a body at y=10
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let key = physics.add_body(body);

    // Step physics once
    physics.step(0.1);

    // Body should have fallen
    let body = physics.get_body(key).unwrap();
    assert!(
        body.position.y < 10.0,
        "Body should fall under gravity. Position: {:?}",
        body.position
    );
    assert!(
        body.velocity.y < 0.0,
        "Body should have downward velocity. Velocity: {:?}",
        body.velocity
    );
}

/// Test that a dynamic body lands on a floor and becomes grounded
#[test]
fn test_dynamic_body_lands_on_floor() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a floor at y=0
    physics.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

    // Add a body slightly above the floor
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let key = physics.add_body(body);

    // Step physics multiple times until it settles
    for _ in 0..100 {
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();

    // Body should be near the floor (radius 0.5, floor at 0, so center at ~0.5)
    assert!(
        body.position.y < 1.0,
        "Body should have fallen. Y={}",
        body.position.y
    );
    assert!(
        body.position.y > -1.0,
        "Body should be above floor. Y={}",
        body.position.y
    );
    assert!(body.grounded, "Body should be grounded after settling");
}

/// Test bounded floor collision (the specific bug scenario)
#[test]
fn test_aabb_body_lands_on_bounded_floor() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a bounded floor at y=-2 (matching default.ron)
    physics.add_static_collider(StaticCollider::floor_bounded(
        -2.0, // y (surface level)
        10.0, // half_size_xz
        5.0,  // half_size_w
        5.0,  // thickness (minimum)
        PhysicsMaterial::CONCRETE,
    ));

    // Add an AABB body at y=0 with half_extent=1 (matching tesseract in default.ron)
    let body = RigidBody4D::new_aabb(
        Vec4::new(0.0, 0.0, 0.0, 0.0), // position
        Vec4::new(1.0, 1.0, 1.0, 1.0), // half_extents
    )
    .with_body_type(BodyType::Dynamic)
    .with_mass(10.0)
    .with_material(PhysicsMaterial::WOOD);

    let key = physics.add_body(body);

    // Record initial position
    let initial_y = physics.get_body(key).unwrap().position.y;

    // Step physics for 2 seconds
    for _ in 0..120 {
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();

    // Body should have fallen from y=0
    assert!(
        body.position.y < initial_y,
        "Body should have fallen. Initial: {}, Final: {}",
        initial_y,
        body.position.y
    );

    // Body center should be at approximately y=-1 (bottom at y=-2, floor surface at y=-2)
    // With half_extent.y=1, center at y=-1 means bottom is at y=-2 (floor surface)
    assert!(
        body.position.y > -2.0,
        "Body should be above floor. Y={}",
        body.position.y
    );
    assert!(
        body.position.y < 0.0,
        "Body should be below starting position. Y={}",
        body.position.y
    );

    // Body should be grounded
    assert!(
        body.grounded,
        "Body should be grounded after landing. Position: {:?}, Grounded: {}",
        body.position, body.grounded
    );
}

// ==================== Entity-Physics Sync Tests ====================

/// Test that entity transform syncs from physics body
#[test]
fn test_entity_transform_syncs_from_physics() {
    let mut world = World::new().with_physics(PhysicsConfig::new(-20.0));

    // Add a physics body with velocity
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let body_key = world.physics_mut().unwrap().add_body(body);

    // Create an entity linked to the physics body
    let tesseract = Tesseract4D::new(2.0);
    let entity_handle = world.spawn((
        ShapeRef::shared(tesseract),
        Transform4D::identity(),
        Material::default(),
        DirtyFlags::ALL,
        Name::new("test"),
        PhysicsBody(body_key),
    ));

    // Clear dirty flags
    world.clear_all_dirty();

    // Step physics
    world.update(0.1);

    // Entity should have new position
    let transform = world.ecs().get::<&Transform4D>(entity_handle).unwrap();
    assert!(
        transform.position.y < 10.0,
        "Entity should have moved. Y={}",
        transform.position.y
    );

    // Entity should be marked dirty
    let dirty = world.ecs().get::<&DirtyFlags>(entity_handle).unwrap();
    assert!(
        !dirty.is_empty(),
        "Entity should be marked dirty after position sync"
    );
}

// ==================== Full Pipeline Test ====================

/// The critical test: full scene loading to physics settling
/// This tests the exact scenario from default.ron
#[test]
fn test_scene_dynamic_entity_falls_to_floor() {
    // Create a scene matching default.ron structure
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    // Add floor
    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::hyperplane(-2.0, 10.0, 10, 5.0, 0.001),
            Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0)),
            Material::GRAY,
        )
        .with_name("floor")
        .with_tag("static"),
    );

    // Add tesseract at y=0
    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(0.0, 0.0, 0.0, 0.0)),
            Material::WHITE,
        )
        .with_name("tesseract")
        .with_tag("dynamic"),
    );

    // Instantiate scene
    let mut active = ActiveScene::from_template(&scene, None);

    // Get initial tesseract position
    let entity_handle = active.world.get_by_name("tesseract").unwrap();
    let initial_y = active
        .world
        .ecs()
        .get::<&Transform4D>(entity_handle)
        .unwrap()
        .position
        .y;

    // Simulate 2 seconds (120 frames at 60fps)
    for _ in 0..120 {
        active.update(1.0 / 60.0);
    }

    // Get final position
    let entity_handle = active.world.get_by_name("tesseract").unwrap();
    let final_y = active
        .world
        .ecs()
        .get::<&Transform4D>(entity_handle)
        .unwrap()
        .position
        .y;

    // Tesseract should have fallen
    assert!(
        final_y < initial_y,
        "Tesseract should have fallen. Initial: {}, Final: {}",
        initial_y,
        final_y
    );

    // Tesseract should be near the floor (center at ~-1, bottom at -2)
    assert!(
        final_y > -2.0,
        "Tesseract should be above floor surface. Y={}",
        final_y
    );
    assert!(
        final_y < 0.0,
        "Tesseract should be below starting position. Y={}",
        final_y
    );

    // Verify physics body is grounded
    let physics = active.world.physics().unwrap();
    let body_comp = active
        .world
        .ecs()
        .get::<&PhysicsBody>(entity_handle)
        .unwrap();
    let body = physics.get_body(body_comp.0).unwrap();
    assert!(body.grounded, "Tesseract physics body should be grounded");
}

/// Test with actual scene file (requires scenes/default.ron to exist)
///
/// Run with: cargo test --test physics_integration test_load_default_scene_file -- --ignored
#[test]
#[ignore = "Requires scenes/default.ron to exist"]
fn test_load_default_scene_file() {
    // Try to load the actual default.ron scene
    let scene = Scene::load("../../../scenes/default.ron")
        .expect("scenes/default.ron should exist for this test");

    // Instantiate scene
    let mut active = ActiveScene::from_template(&scene, None);

    // Verify tesseract entity exists and has physics body
    let entity_handle = active
        .world
        .get_by_name("tesseract")
        .expect("Tesseract entity should exist in default scene");

    assert!(
        active
            .world
            .ecs()
            .get::<&PhysicsBody>(entity_handle)
            .is_ok(),
        "Tesseract should have physics body"
    );

    let initial_y = active
        .world
        .ecs()
        .get::<&Transform4D>(entity_handle)
        .unwrap()
        .position
        .y;

    // Simulate 2 seconds
    for _ in 0..120 {
        active.update(1.0 / 60.0);
    }

    // Get final position
    let entity_handle = active.world.get_by_name("tesseract").unwrap();
    let final_y = active
        .world
        .ecs()
        .get::<&Transform4D>(entity_handle)
        .unwrap()
        .position
        .y;

    // Tesseract should have fallen
    assert!(
        final_y < initial_y,
        "Tesseract should have fallen from {} to near floor. Final: {}",
        initial_y,
        final_y
    );
}

// ==================== Diagnostic Tests ====================

/// Test that walking off the W edge causes falling (true 4D physics)
#[test]
fn test_kinematic_body_falls_off_w_edge() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add bounded floor: W extends from -5 to +5
    physics.add_static_collider(StaticCollider::floor_bounded(
        -2.0,
        10.0,
        5.0,
        5.0,
        PhysicsMaterial::CONCRETE,
    ));

    // Add kinematic sphere at center, resting on floor, with gravity enabled
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, -1.5, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Kinematic)
        .with_gravity(true);
    let key = physics.add_body(body);

    // Step physics to settle
    for _ in 0..10 {
        physics.step(1.0 / 60.0);
    }

    // Body should be grounded
    assert!(
        physics.body_is_grounded(key),
        "Body should be grounded at center"
    );
    let start_y = physics.body_position(key).unwrap().y;

    // Move body to W=6 (outside floor's W bounds of -5 to +5)
    for _ in 0..60 {
        physics.apply_body_movement(key, Vec4::new(0.0, 0.0, 0.0, 10.0));
        physics.step(1.0 / 60.0);
    }

    let pos = physics.body_position(key).unwrap();
    assert!(
        pos.w > 5.0,
        "Body should have moved off W edge. W={}",
        pos.w
    );

    assert!(
        !physics.body_is_grounded(key),
        "Body should NOT be grounded when off W edge. W={}",
        pos.w
    );

    // Continue stepping - body should fall
    for _ in 0..60 {
        physics.apply_body_movement(key, Vec4::ZERO);
        physics.step(1.0 / 60.0);
    }

    let final_pos = physics.body_position(key).unwrap();
    assert!(
        final_pos.y < start_y,
        "Body should fall when off W edge. Start Y={}, Final Y={}",
        start_y,
        final_pos.y
    );
}

/// Print detailed state for debugging
#[test]
fn test_physics_step_trace() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add bounded floor at y=-2
    physics.add_static_collider(StaticCollider::floor_bounded(
        -2.0,
        10.0,
        5.0,
        5.0,
        PhysicsMaterial::CONCRETE,
    ));

    // Add AABB body at y=0
    let body = RigidBody4D::new_aabb(Vec4::new(0.0, 0.0, 0.0, 0.0), Vec4::new(1.0, 1.0, 1.0, 1.0))
        .with_body_type(BodyType::Dynamic)
        .with_mass(10.0);

    let key = physics.add_body(body);

    println!("=== Physics Step Trace ===");
    println!("Gravity: {}", physics.config.gravity);
    println!("Static colliders: {}", physics.static_colliders().len());

    for frame in 0..10 {
        let body = physics.get_body(key).unwrap();
        println!(
            "Frame {}: pos.y={:.4}, vel.y={:.4}, grounded={}",
            frame, body.position.y, body.velocity.y, body.grounded
        );
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();
    println!(
        "Final: pos.y={:.4}, vel.y={:.4}, grounded={}",
        body.position.y, body.velocity.y, body.grounded
    );

    // Should be falling
    assert!(body.position.y < 0.0, "Body should have fallen");
}

// ==================== Physics Cleanup Tests ====================

/// Test that removing an entity also removes its physics body
#[test]
fn test_remove_entity_cleans_up_physics_body() {
    let mut world = World::new().with_physics(PhysicsConfig::new(-20.0));

    // Add a physics body
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let body_key = world.physics_mut().unwrap().add_body(body);

    // Create an entity linked to the physics body
    let tesseract = Tesseract4D::new(2.0);
    let entity_key = world.spawn((
        ShapeRef::shared(tesseract),
        Transform4D::identity(),
        Material::default(),
        DirtyFlags::ALL,
        Name::new("physics_test"),
        PhysicsBody(body_key),
    ));

    // Verify body exists in physics world
    assert!(
        world.physics().unwrap().get_body(body_key).is_some(),
        "Physics body should exist before entity removal"
    );
    assert_eq!(
        world.physics().unwrap().body_count(),
        1,
        "Should have exactly 1 physics body"
    );

    // Remove the entity
    let removed = world.despawn(entity_key);
    assert!(removed, "Entity should be removed");

    // Verify physics body was also removed
    assert!(
        world.physics().unwrap().get_body(body_key).is_none(),
        "Physics body should be removed when entity is removed"
    );
    assert_eq!(
        world.physics().unwrap().body_count(),
        0,
        "Should have 0 physics bodies after entity removal"
    );
}

/// Test that removing an entity without physics body works correctly
#[test]
fn test_remove_entity_without_physics_body() {
    let mut world = World::new().with_physics(PhysicsConfig::new(-20.0));

    // Create an entity WITHOUT a physics body
    let tesseract = Tesseract4D::new(2.0);
    let entity_key = world.spawn((
        ShapeRef::shared(tesseract),
        Transform4D::identity(),
        Material::default(),
        DirtyFlags::ALL,
        Name::new("no_physics"),
    ));

    // Remove the entity - should not panic
    let removed = world.despawn(entity_key);
    assert!(removed, "Entity should be removed");
    // Verify entity is gone
    assert!(!world.contains(entity_key));
}

/// Test that removing an entity in a world without physics works
#[test]
fn test_remove_entity_world_without_physics() {
    let mut world = World::new(); // No physics enabled

    // Create an entity
    let tesseract = Tesseract4D::new(2.0);
    let entity_key = world.spawn((
        ShapeRef::shared(tesseract),
        Transform4D::identity(),
        Material::default(),
        DirtyFlags::ALL,
        Name::new("test"),
    ));

    // Remove should work fine
    let removed = world.despawn(entity_key);
    assert!(
        removed,
        "Entity should be removed even without physics world"
    );
}
