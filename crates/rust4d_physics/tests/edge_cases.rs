//! Edge-case tests for the physics crate.
//!
//! Covers: stale BodyKey safety, zero-vector movement, raycasting edge cases,
//! and collision event edge cases.

use rust4d_math::{Ray4D, Vec4};
use rust4d_physics::{
    body::{BodyType, RigidBody4D, StaticCollider},
    collision::{CollisionEventKind, CollisionFilter, CollisionLayer},
    material::PhysicsMaterial,
    raycast::ray_vs_sphere,
    shapes::Sphere4D,
    world::{PhysicsConfig, PhysicsWorld, RayTarget},
};

// =============================================================================
// T2: Stale BodyKey tests
// =============================================================================

/// After removing a body, body_position() with the stale key returns None.
#[test]
fn stale_key_body_position_returns_none() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(Vec4::new(1.0, 2.0, 3.0, 0.0), 0.5);
    let key = world.add_body(body);

    // Valid first
    assert!(world.body_position(key).is_some());

    // Remove
    world.remove_body(key);

    // Stale key -> None
    assert_eq!(world.body_position(key), None);
}

/// After removing a body, body_jump() with the stale key returns false (no panic).
#[test]
fn stale_key_body_jump_returns_false() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5);
    let key = world.add_body(body);
    world.remove_body(key);

    let jumped = world.body_jump(key, 10.0);
    assert!(!jumped, "body_jump with stale key should return false");
}

/// After removing a body, apply_body_movement() with the stale key is a no-op (no panic).
#[test]
fn stale_key_apply_body_movement_no_op() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5);
    let key = world.add_body(body);
    world.remove_body(key);

    // Should silently do nothing
    world.apply_body_movement(key, Vec4::new(10.0, 0.0, 5.0, 0.0));
}

/// After removing a body, get_body() and get_body_mut() return None.
#[test]
fn stale_key_get_body_returns_none() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5);
    let key = world.add_body(body);
    world.remove_body(key);

    assert!(world.get_body(key).is_none());
    assert!(world.get_body_mut(key).is_none());
}

/// body_is_grounded() with a stale key returns false (no panic).
#[test]
fn stale_key_body_is_grounded_returns_false() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5);
    let key = world.add_body(body);
    world.remove_body(key);

    assert!(!world.body_is_grounded(key));
}

/// A stale key should not match a new body inserted in the same slot.
/// SlotMap generational indexing prevents the ABA problem.
#[test]
fn stale_key_does_not_alias_new_body() {
    let mut world = PhysicsWorld::new();

    // Add and remove several bodies to reuse slots
    let key1 = world.add_body(RigidBody4D::new_sphere(Vec4::new(1.0, 0.0, 0.0, 0.0), 0.5));
    world.remove_body(key1);

    let key2 = world.add_body(RigidBody4D::new_sphere(Vec4::new(2.0, 0.0, 0.0, 0.0), 0.5));

    // key1 is stale; key2 is valid
    assert!(world.get_body(key1).is_none(), "Stale key must not resolve to new body");
    assert!(world.get_body(key2).is_some());
    assert_ne!(key1, key2);
}

/// A full physics step with a stale key referenced in active_triggers does not panic.
/// We simulate this by removing a body that was inside a trigger, then stepping.
#[test]
fn stale_key_in_trigger_tracking_no_panic() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    // Trigger zone
    let trigger = StaticCollider::aabb(
        Vec4::ZERO,
        Vec4::new(5.0, 5.0, 5.0, 5.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));
    world.add_static_collider(trigger);

    // Player inside the trigger
    let key = world.add_body(
        RigidBody4D::new_sphere(Vec4::ZERO, 0.5)
            .with_filter(CollisionFilter::player())
            .with_gravity(false),
    );

    // Step to register TriggerEnter
    world.step(0.016);
    let events = world.drain_collision_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, CollisionEventKind::TriggerEnter { .. })),
        "Should have TriggerEnter"
    );

    // Remove body while it is still tracked as "inside" the trigger
    world.remove_body(key);

    // Step again -- should produce TriggerExit (or silently skip) with no panic
    world.step(0.016);
    let events = world.drain_collision_events();

    // Depending on implementation, the exit may or may not fire for the removed key.
    // The critical thing is no panic.
    // If exit fires, it should reference the old key.
    for event in &events {
        if let CollisionEventKind::TriggerExit { body, .. } = event.kind {
            assert_eq!(body, key, "Exit event should reference the original key");
        }
    }
}

// =============================================================================
// T6: Zero-vector movement tests
// =============================================================================

/// apply_body_movement(key, Vec4::ZERO) should zero the horizontal velocity components.
#[test]
fn zero_vector_movement_zeroes_horizontal_velocity() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5)
        .with_velocity(Vec4::new(5.0, -2.0, 3.0, 1.0))
        .with_body_type(BodyType::Kinematic);
    let key = world.add_body(body);

    // Apply zero movement -- should zero X, Z, W but preserve Y
    world.apply_body_movement(key, Vec4::ZERO);

    let body = world.get_body(key).unwrap();
    assert_eq!(body.velocity.x, 0.0, "X velocity should be zeroed");
    assert_eq!(body.velocity.z, 0.0, "Z velocity should be zeroed");
    assert_eq!(body.velocity.w, 0.0, "W velocity should be zeroed");
    assert_eq!(body.velocity.y, -2.0, "Y velocity should be preserved");
}

/// apply_body_movement only modifies X, Z, W -- never Y.
#[test]
fn movement_preserves_y_velocity() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5)
        .with_velocity(Vec4::new(0.0, -10.0, 0.0, 0.0))
        .with_body_type(BodyType::Dynamic);
    let key = world.add_body(body);

    world.apply_body_movement(key, Vec4::new(1.0, 99.0, 2.0, 3.0));

    let body = world.get_body(key).unwrap();
    assert_eq!(body.velocity.x, 1.0);
    assert_eq!(
        body.velocity.y, -10.0,
        "Y velocity must not change even if movement.y is non-zero"
    );
    assert_eq!(body.velocity.z, 2.0);
    assert_eq!(body.velocity.w, 3.0);
}

/// After zeroing movement and stepping, the body should not drift horizontally.
#[test]
fn zero_movement_then_step_no_drift() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // no gravity

    let body = RigidBody4D::new_sphere(Vec4::new(5.0, 0.0, 3.0, 1.0), 0.5)
        .with_velocity(Vec4::new(10.0, 0.0, -8.0, 4.0))
        .with_body_type(BodyType::Kinematic);
    let key = world.add_body(body);

    // Zero horizontal movement
    world.apply_body_movement(key, Vec4::ZERO);

    let pos_before = world.body_position(key).unwrap();
    world.step(0.1);
    let pos_after = world.body_position(key).unwrap();

    // X, Z, W should not change (velocity was zeroed)
    assert!(
        (pos_after.x - pos_before.x).abs() < 0.001,
        "X should not drift"
    );
    assert!(
        (pos_after.z - pos_before.z).abs() < 0.001,
        "Z should not drift"
    );
    assert!(
        (pos_after.w - pos_before.w).abs() < 0.001,
        "W should not drift"
    );
}

// =============================================================================
// Raycasting edge cases
// =============================================================================

/// Raycast with a very large max_distance still returns correct hit.
#[test]
fn raycast_very_large_max_distance() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));
    let key = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(5.0, 0.0, 0.0, 0.0),
        1.0,
    ));

    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
    let hits = world.raycast(&ray, 1e12, CollisionLayer::ALL);

    assert_eq!(hits.len(), 1, "Should still hit with huge max_distance");
    match hits[0].target {
        RayTarget::Body(k) => assert_eq!(k, key),
        _ => panic!("Expected body hit"),
    }
    assert!(
        (hits[0].hit.distance - 4.0).abs() < 0.1,
        "Distance should be correct"
    );
}

/// Raycast with max_distance = 0 should not hit anything.
#[test]
fn raycast_zero_max_distance() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));
    world.add_body(RigidBody4D::new_sphere(
        Vec4::new(0.5, 0.0, 0.0, 0.0),
        1.0,
    ));

    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
    // The ray origin is inside the sphere, so the exit point distance is > 0.
    // With max_distance = 0, it should miss because hit.distance > 0.
    let hits = world.raycast(&ray, 0.0, CollisionLayer::ALL);
    // Even if origin is inside, exit point distance > 0, so with max_distance=0 nothing should match
    assert!(
        hits.is_empty(),
        "Zero max_distance should yield no hits"
    );
}

/// Ray originating inside a sphere should hit the exit point.
#[test]
fn ray_inside_sphere_hits_exit() {
    let sphere = Sphere4D::new(Vec4::ZERO, 2.0);
    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);

    let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit exit point");
    assert!(
        (hit.distance - 2.0).abs() < 0.01,
        "Exit distance should be radius=2.0, got {}",
        hit.distance
    );
    assert!(
        (hit.point.x - 2.0).abs() < 0.01,
        "Exit point should be at x=2.0"
    );
}

/// Tangent ray barely grazing a sphere should either hit or miss cleanly (no crash).
#[test]
fn ray_tangent_to_sphere_no_crash() {
    let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

    // Ray at exactly y=1.0 (the tangent point) traveling in +X
    let tangent_ray = Ray4D::new(Vec4::new(-10.0, 1.0, 0.0, 0.0), Vec4::X);
    let result = ray_vs_sphere(&tangent_ray, &sphere);
    // May or may not detect a hit due to floating point, but must not crash
    if let Some(hit) = result {
        assert!(hit.distance >= 0.0, "Distance should be non-negative");
    }

    // Ray at y = 1.0 + epsilon (just outside) should miss
    let miss_ray = Ray4D::new(Vec4::new(-10.0, 1.001, 0.0, 0.0), Vec4::X);
    assert!(
        ray_vs_sphere(&miss_ray, &sphere).is_none(),
        "Ray just outside tangent should miss"
    );

    // Ray at y = 1.0 - epsilon (just inside tangent) should hit
    let hit_ray = Ray4D::new(Vec4::new(-10.0, 0.999, 0.0, 0.0), Vec4::X);
    assert!(
        ray_vs_sphere(&hit_ray, &sphere).is_some(),
        "Ray just inside tangent should hit"
    );
}

/// Raycasting against a very small sphere (tiny radius).
#[test]
fn raycast_very_small_sphere() {
    let sphere = Sphere4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), 0.001);
    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);

    let hit = ray_vs_sphere(&ray, &sphere);
    assert!(
        hit.is_some(),
        "Should hit very small sphere along direct path"
    );
    let hit = hit.unwrap();
    assert!(
        (hit.distance - 4.999).abs() < 0.01,
        "Distance should be ~5 - 0.001"
    );
}

/// Raycasting with a very small sphere that is off-axis should miss.
#[test]
fn raycast_very_small_sphere_off_axis_misses() {
    // Sphere at (5, 0.01, 0, 0) with radius 0.001 -- ray along +X misses
    let sphere = Sphere4D::new(Vec4::new(5.0, 0.01, 0.0, 0.0), 0.001);
    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);

    assert!(
        ray_vs_sphere(&ray, &sphere).is_none(),
        "Tiny sphere offset from ray axis should be missed"
    );
}

/// World raycast_nearest with very large max_distance still returns the closest.
#[test]
fn raycast_nearest_large_max_distance() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let near = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(3.0, 0.0, 0.0, 0.0),
        0.5,
    ));
    world.add_body(RigidBody4D::new_sphere(
        Vec4::new(100.0, 0.0, 0.0, 0.0),
        0.5,
    ));

    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
    let hit = world
        .raycast_nearest(&ray, f32::MAX, CollisionLayer::ALL)
        .expect("Should hit at least one body");

    match hit.target {
        RayTarget::Body(k) => assert_eq!(k, near, "Nearest body should be returned"),
        _ => panic!("Expected body hit"),
    }
}

/// Raycast that hits both a body and a static collider returns both, sorted.
#[test]
fn raycast_hits_body_and_static_sorted() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    // Static floor plane at y=0
    world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

    // Body sphere at (0, 5, 0, 0) -- above floor
    world.add_body(RigidBody4D::new_sphere(
        Vec4::new(0.0, 5.0, 0.0, 0.0),
        1.0,
    ));

    // Ray shooting downward from high above
    let ray = Ray4D::new(Vec4::new(0.0, 20.0, 0.0, 0.0), -Vec4::Y);
    let hits = world.raycast(&ray, 100.0, CollisionLayer::ALL);

    assert!(hits.len() >= 2, "Should hit body and floor");
    // First hit should be the body (closer), second the floor
    assert!(
        hits[0].hit.distance < hits[1].hit.distance,
        "Hits should be sorted by distance"
    );
}

// =============================================================================
// Collision event edge cases
// =============================================================================

/// Test that zero-mass bodies don't cause division-by-zero in collision response.
#[test]
fn zero_mass_body_collision_no_panic() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let body_a = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
        .with_mass(0.0);
    let body_b = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5)
        .with_mass(0.0);

    world.add_body(body_a);
    world.add_body(body_b);

    // Should not panic or produce NaN
    world.step(0.016);
    let events = world.drain_collision_events();
    assert!(
        !events.is_empty(),
        "Overlapping zero-mass bodies should still generate events"
    );
}

/// Test that a body with zero mass colliding with a normal body does not panic.
#[test]
fn zero_mass_vs_normal_mass_no_panic() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let zero = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5).with_mass(0.0);
    let normal = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5).with_mass(5.0);

    let key_zero = world.add_body(zero);
    let key_normal = world.add_body(normal);

    world.step(0.016);

    // Neither body should have NaN position
    let pos_zero = world.body_position(key_zero).unwrap();
    let pos_normal = world.body_position(key_normal).unwrap();
    assert!(!pos_zero.x.is_nan(), "Zero-mass body should not produce NaN");
    assert!(
        !pos_normal.x.is_nan(),
        "Normal body should not produce NaN from zero-mass collision"
    );
}

/// Test that multiple triggers can fire on the same body simultaneously.
#[test]
fn multiple_triggers_on_same_body() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    // Two trigger zones at different positions, both overlapping the body
    let trigger1 = StaticCollider::aabb(
        Vec4::new(0.0, 0.0, 0.0, 0.0),
        Vec4::new(5.0, 5.0, 5.0, 5.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    let trigger2 = StaticCollider::aabb(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(5.0, 5.0, 5.0, 5.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    world.add_static_collider(trigger1);
    world.add_static_collider(trigger2);

    // Player body inside both triggers
    let _key = world.add_body(
        RigidBody4D::new_sphere(Vec4::new(0.5, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player())
            .with_gravity(false),
    );

    world.step(0.016);
    let events = world.drain_collision_events();

    let enter_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::TriggerEnter { .. }))
        .collect();

    assert_eq!(
        enter_events.len(),
        2,
        "Body should trigger Enter on both triggers"
    );

    // Verify both trigger indices are present
    let indices: Vec<usize> = enter_events
        .iter()
        .map(|e| {
            if let CollisionEventKind::TriggerEnter {
                trigger_index, ..
            } = e.kind
            {
                trigger_index
            } else {
                unreachable!()
            }
        })
        .collect();
    assert!(indices.contains(&0), "Should have event for trigger 0");
    assert!(indices.contains(&1), "Should have event for trigger 1");
}

/// After Enter, staying for two steps should produce Stay events each step.
#[test]
fn multiple_triggers_stay_events() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let trigger1 = StaticCollider::aabb(
        Vec4::ZERO,
        Vec4::new(5.0, 5.0, 5.0, 5.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    let trigger2 = StaticCollider::aabb(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(5.0, 5.0, 5.0, 5.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    world.add_static_collider(trigger1);
    world.add_static_collider(trigger2);

    world.add_body(
        RigidBody4D::new_sphere(Vec4::new(0.5, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player())
            .with_gravity(false),
    );

    // Step 1: Enter
    world.step(0.016);
    world.drain_collision_events();

    // Step 2: Stay
    world.step(0.016);
    let events = world.drain_collision_events();

    let stay_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::TriggerStay { .. }))
        .collect();

    assert_eq!(
        stay_events.len(),
        2,
        "Both triggers should produce Stay events"
    );
}

/// Exiting one trigger while staying in another should produce the right mix of events.
#[test]
fn exit_one_trigger_stay_in_other() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    // Trigger 1: small, at origin
    let trigger1 = StaticCollider::aabb(
        Vec4::ZERO,
        Vec4::new(1.0, 1.0, 1.0, 1.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    // Trigger 2: large, also covering origin and extending far in +X
    let trigger2 = StaticCollider::aabb(
        Vec4::new(5.0, 0.0, 0.0, 0.0),
        Vec4::new(10.0, 10.0, 10.0, 10.0),
        PhysicsMaterial::CONCRETE,
    )
    .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

    world.add_static_collider(trigger1);
    world.add_static_collider(trigger2);

    let key = world.add_body(
        RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player())
            .with_gravity(false),
    );

    // Step to get Enter on both
    world.step(0.016);
    let events = world.drain_collision_events();
    let enter_count = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::TriggerEnter { .. }))
        .count();
    assert_eq!(enter_count, 2, "Should enter both triggers");

    // Teleport body far in +X (out of trigger1 but still in trigger2)
    world
        .get_body_mut(key)
        .unwrap()
        .set_position(Vec4::new(8.0, 0.0, 0.0, 0.0));

    world.step(0.016);
    let events = world.drain_collision_events();

    let exit_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::TriggerExit { .. }))
        .collect();
    let stay_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::TriggerStay { .. }))
        .collect();

    assert_eq!(
        exit_events.len(),
        1,
        "Should exit one trigger (the small one)"
    );
    assert_eq!(
        stay_events.len(),
        1,
        "Should stay in one trigger (the large one)"
    );

    if let CollisionEventKind::TriggerExit { trigger_index, .. } = exit_events[0].kind {
        assert_eq!(trigger_index, 0, "Should exit trigger 0 (small)");
    }
    if let CollisionEventKind::TriggerStay { trigger_index, .. } = stay_events[0].kind {
        assert_eq!(trigger_index, 1, "Should stay in trigger 1 (large)");
    }
}

/// Collision events report correct BodyKeys even with multiple bodies.
#[test]
fn collision_events_report_correct_keys() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    // Three non-overlapping bodies -- only first two overlap
    let key_a = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(0.0, 0.0, 0.0, 0.0),
        0.5,
    ));
    let key_b = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(0.8, 0.0, 0.0, 0.0),
        0.5,
    ));
    let key_c = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(10.0, 0.0, 0.0, 0.0),
        0.5,
    ));

    world.step(0.016);
    let events = world.drain_collision_events();

    let body_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CollisionEventKind::BodyVsBody { .. }))
        .collect();

    assert_eq!(body_events.len(), 1, "Only one pair should collide");
    if let CollisionEventKind::BodyVsBody { body_a, body_b } = body_events[0].kind {
        assert_eq!(body_a, key_a);
        assert_eq!(body_b, key_b);
    }

    // key_c should not appear in any events
    for event in &events {
        match event.kind {
            CollisionEventKind::BodyVsBody { body_a, body_b } => {
                assert_ne!(body_a, key_c);
                assert_ne!(body_b, key_c);
            }
            _ => {}
        }
    }
}

/// Bodies removed between steps should not cause panics during collision detection.
#[test]
fn removed_body_during_simulation_no_panic() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

    let key_a = world.add_body(RigidBody4D::new_sphere(Vec4::ZERO, 0.5));
    let key_b = world.add_body(RigidBody4D::new_sphere(
        Vec4::new(0.8, 0.0, 0.0, 0.0),
        0.5,
    ));

    // Step once to establish collision
    world.step(0.016);
    world.drain_collision_events();

    // Remove one body
    world.remove_body(key_a);

    // Step again -- should not panic
    world.step(0.016);

    // key_b should still exist
    assert!(world.get_body(key_b).is_some());
}

/// Ray originating exactly on the sphere surface.
#[test]
fn ray_from_sphere_surface() {
    let sphere = Sphere4D::new(Vec4::ZERO, 1.0);

    // Ray origin at surface pointing outward
    let ray = Ray4D::new(Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::X);
    // Origin is exactly at the surface. The ray is heading outward.
    // oc = (1,0,0,0), b = oc.dot(dir) = 1.0, c = 1-1 = 0
    // discriminant = 1 - 0 = 1, sqrt = 1
    // t1 = -1 - 1 = -2 (behind), t2 = -1 + 1 = 0 (at origin)
    // t=0 should be a valid hit (at the surface)
    let result = ray_vs_sphere(&ray, &sphere);
    if let Some(hit) = result {
        assert!(hit.distance >= 0.0);
    }
}

/// Ray in a diagonal 4D direction hitting a sphere.
#[test]
fn ray_diagonal_4d_direction() {
    let sphere = Sphere4D::new(Vec4::new(5.0, 5.0, 5.0, 5.0), 1.0);

    // Diagonal ray from origin toward (1,1,1,1) direction
    let dir = Vec4::new(1.0, 1.0, 1.0, 1.0).normalized();
    let ray = Ray4D::new(Vec4::ZERO, dir);

    let hit = ray_vs_sphere(&ray, &sphere).expect("Should hit sphere along diagonal");
    assert!(hit.distance > 0.0);
    // The center is at distance sqrt(4*25) = 10 from origin in 4D
    // Hit should be at approximately 10 - 1 = 9
    assert!(
        (hit.distance - 9.0).abs() < 0.1,
        "Distance should be ~9, got {}",
        hit.distance
    );
}

/// Raycast against an empty world returns no hits.
#[test]
fn raycast_empty_world() {
    let world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));
    let ray = Ray4D::new(Vec4::ZERO, Vec4::X);
    let hits = world.raycast(&ray, 100.0, CollisionLayer::ALL);
    assert!(hits.is_empty());
    assert!(world.raycast_nearest(&ray, 100.0, CollisionLayer::ALL).is_none());
}

/// A dynamic body with gravity disabled colliding with another should still produce events.
#[test]
fn gravity_disabled_bodies_still_generate_collision_events() {
    let mut world = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Two overlapping spheres with gravity disabled
    let a = RigidBody4D::new_sphere(Vec4::ZERO, 0.5).with_gravity(false);
    let b = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5).with_gravity(false);

    world.add_body(a);
    world.add_body(b);

    world.step(0.016);
    let events = world.drain_collision_events();

    assert!(
        !events.is_empty(),
        "Gravity-disabled overlapping bodies should still produce collision events"
    );
}

/// Kinematic body: body_type default gravity settings.
#[test]
fn kinematic_default_gravity_disabled() {
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5).with_body_type(BodyType::Kinematic);
    assert!(
        !body.affected_by_gravity(),
        "Kinematic bodies should have gravity disabled by default"
    );
}

/// Dynamic body: gravity enabled by default.
#[test]
fn dynamic_default_gravity_enabled() {
    let body = RigidBody4D::new_sphere(Vec4::ZERO, 0.5);
    assert!(body.affected_by_gravity());
}

/// with_gravity overrides the body type default.
#[test]
fn with_gravity_overrides_body_type() {
    let kinematic_with_gravity = RigidBody4D::new_sphere(Vec4::ZERO, 0.5)
        .with_body_type(BodyType::Kinematic)
        .with_gravity(true);
    assert!(kinematic_with_gravity.affected_by_gravity());

    let dynamic_without_gravity = RigidBody4D::new_sphere(Vec4::ZERO, 0.5)
        .with_body_type(BodyType::Dynamic)
        .with_gravity(false);
    assert!(!dynamic_without_gravity.affected_by_gravity());
}
