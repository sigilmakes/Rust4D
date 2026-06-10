//! Integration tests for Interpolatable trait usage from external crates
//!
//! These tests verify that the Interpolatable trait is properly exported and usable
//! by code outside the rust4d_math crate, which is the typical use case for the
//! tween system in rust4d_game.

use rust4d_math::{Interpolatable, RotationPlane, Rotor4, Vec4};
use std::f32::consts::PI;

const EPSILON: f32 = 0.001;

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < EPSILON
}

/// Test that f32 Interpolatable is accessible and works correctly
#[test]
fn test_f32_interpolatable_external() {
    // Use the trait method with fully qualified syntax (how external code often uses it)
    let result = <f32 as Interpolatable>::lerp(&0.0, &100.0, 0.5);
    assert!(approx_eq(result, 50.0));

    // Also works with type inference
    let result: f32 = Interpolatable::lerp(&10.0, &20.0, 0.25);
    assert!(approx_eq(result, 12.5));
}

/// Test that f64 Interpolatable is accessible
#[test]
fn test_f64_interpolatable_external() {
    let result = <f64 as Interpolatable>::lerp(&0.0, &100.0, 0.5);
    assert!((result - 50.0).abs() < 0.001);
}

/// Test that Vec4 Interpolatable is accessible and works correctly
#[test]
fn test_vec4_interpolatable_external() {
    let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
    let b = Vec4::new(10.0, 20.0, 30.0, 40.0);

    let result = <Vec4 as Interpolatable>::lerp(&a, &b, 0.5);

    assert!(approx_eq(result.x, 5.0));
    assert!(approx_eq(result.y, 10.0));
    assert!(approx_eq(result.z, 15.0));
    assert!(approx_eq(result.w, 20.0));
}

/// Test that Rotor4 Interpolatable (slerp) is accessible and works correctly
#[test]
fn test_rotor4_interpolatable_external() {
    let identity = Rotor4::IDENTITY;
    let rotated = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

    // Halfway through a 90-degree rotation should be a 45-degree rotation
    let result = <Rotor4 as Interpolatable>::lerp(&identity, &rotated, 0.5);

    // Apply to a test vector
    let v = Vec4::X;
    let rotated_v = result.rotate(v);

    // Should be at 45 degrees in XY plane
    let expected_x = (PI / 4.0).cos();
    let expected_y = (PI / 4.0).sin();
    assert!(approx_eq(rotated_v.x, expected_x));
    assert!(approx_eq(rotated_v.y, expected_y));
}

/// Test that Rotor4 slerp result is properly normalized (fixes MEDIUM-10)
#[test]
fn test_rotor4_slerp_normalized_external() {
    let a = Rotor4::from_plane_angle(RotationPlane::XZ, 0.3);
    let b = Rotor4::from_plane_angle(RotationPlane::YW, 0.7);

    // Perform many interpolations to accumulate potential floating point errors
    let mut current = a;
    for _ in 0..100 {
        current = <Rotor4 as Interpolatable>::lerp(&current, &b, 0.1);
    }

    // Result should still be unit magnitude
    let mag = current.magnitude();
    assert!(
        approx_eq(mag, 1.0),
        "Rotor magnitude after many slerps: {} (should be 1.0)",
        mag
    );
}

/// Test using Interpolatable in a generic function (common pattern for tween systems)
fn generic_tween<T: Interpolatable>(from: &T, to: &T, progress: f32) -> T {
    Interpolatable::lerp(from, to, progress)
}

#[test]
fn test_generic_interpolatable_usage() {
    // f32
    let f32_result = generic_tween(&0.0f32, &100.0, 0.75);
    assert!(approx_eq(f32_result, 75.0));

    // Vec4
    let vec_result = generic_tween(&Vec4::ZERO, &Vec4::new(4.0, 8.0, 12.0, 16.0), 0.25);
    assert!(approx_eq(vec_result.x, 1.0));
    assert!(approx_eq(vec_result.y, 2.0));
    assert!(approx_eq(vec_result.z, 3.0));
    assert!(approx_eq(vec_result.w, 4.0));

    // Rotor4
    let rot_result = generic_tween(&Rotor4::IDENTITY, &Rotor4::IDENTITY, 0.5);
    assert!(approx_eq(rot_result.magnitude(), 1.0));
}

/// Test that the trait bounds work with Clone (required by Interpolatable)
#[test]
fn test_interpolatable_clone_bound() {
    fn requires_interpolatable_and_clone<T: Interpolatable + Clone>(val: &T) -> T {
        val.clone()
    }

    let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let cloned = requires_interpolatable_and_clone(&v);
    assert_eq!(v, cloned);
}
