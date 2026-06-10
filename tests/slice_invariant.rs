//! Slice-plane invariant tests (the "4D movement bug")
//!
//! THE INVARIANT: for any world point `p`, its camera-space W coordinate
//! `dot(ana, p - camera_pos)` must not change while the player moves with
//! WASD — regardless of the camera's 4D rotation state. If it changes, the
//! slice plane sweeps through geometry and shapes visibly morph.
//!
//! Only deliberate W movement (Q/E) and 4D rotation may change it.
//!
//! These tests drive the REAL app stack — scene file, physics world, player
//! body, camera controller, simulation system — with a fixed timestep,
//! mirroring `App::new()` in `src/main.rs`.

use rust4d::systems::SimulationSystem;
use rust4d_core::SceneManager;
use rust4d_game::{scene_helpers, CharacterConfig, CharacterController4D};
use rust4d_input::CameraController;
use rust4d_math::Vec4;
use rust4d_physics::PhysicsConfig;
use rust4d_render::camera4d::Camera4D;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

use std::f32::consts::FRAC_PI_4;

const DT: f32 = 1.0 / 60.0;

/// Mirror of config/default.toml input values
const MOVE_SPEED: f32 = 3.0;
const W_MOVE_SPEED: f32 = 2.0;

/// World-space reference point: the tesseract's center in scenes/default.ron
const TESSERACT_CENTER: Vec4 = Vec4 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 0.0,
};

struct TestRig {
    scene_manager: SceneManager,
    camera: Camera4D,
    controller: CameraController,
    character: Option<CharacterController4D>,
    simulation: SimulationSystem,
}

impl TestRig {
    /// Build the full app stack the same way `App::new()` does.
    fn new() -> Self {
        let mut scene_manager = SceneManager::new().with_physics(PhysicsConfig::new(-20.0));

        let scene_name = scene_manager
            .load_scene("scenes/default.ron")
            .expect("load default scene");
        scene_manager.instantiate(&scene_name).expect("instantiate");
        scene_manager.push_scene(&scene_name).expect("push scene");

        // Player body from spawn point (mirrors main.rs)
        let mut player_start = Vec4::new(0.0, 0.0, 5.0, 0.0);
        if let Some(scene) = scene_manager.active_scene_mut() {
            if let Some(spawn) = scene.player_spawn {
                let spawn_pos = Vec4::new(spawn[0], spawn[1], spawn[2], spawn[3]);
                player_start = spawn_pos;
                if let Some(physics) = scene.world.physics_mut() {
                    let key = scene_helpers::create_player_body(physics, spawn_pos, 0.5);
                    scene.player_body_key = Some(key);
                }
            }
        }

        let mut camera = Camera4D::new();
        camera.position = player_start;

        let controller = CameraController::new()
            .with_move_speed(MOVE_SPEED)
            .with_w_move_speed(W_MOVE_SPEED);

        let character = scene_manager
            .active_scene()
            .and_then(|s| s.player_body_key)
            .map(|key| {
                CharacterController4D::new(
                    key,
                    CharacterConfig {
                        move_speed: MOVE_SPEED,
                        w_move_speed: W_MOVE_SPEED,
                        jump_velocity: 8.0,
                    },
                )
            });
        assert!(character.is_some(), "default scene should produce a player body");

        Self {
            scene_manager,
            camera,
            controller,
            character,
            simulation: SimulationSystem::new(),
        }
    }

    fn step(&mut self) {
        self.simulation.update_with_dt(
            DT,
            &mut self.scene_manager,
            &mut self.camera,
            &mut self.controller,
            self.character.as_ref(),
            false,
        );
    }

    /// Run n frames with the current input state.
    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step();
        }
    }

    /// Camera-space W of a world point: dot(ana, p - camera_pos).
    /// This is exactly what the slice shader computes as `pos.w` before
    /// comparing against slice_w.
    fn camera_space_w(&self, p: Vec4) -> f32 {
        let ana = self.camera.ana();
        ana.dot(p - self.camera.position)
    }

    fn press(&mut self, key: KeyCode) {
        self.controller.process_keyboard(key, ElementState::Pressed);
    }

    fn release(&mut self, key: KeyCode) {
        self.controller.process_keyboard(key, ElementState::Released);
    }

    fn debug_state(&self, label: &str) {
        let fwd = self.camera.forward();
        let ana = self.camera.ana();
        let pos = self.camera.position;
        println!(
            "[CAM {label}] pos=({:.4},{:.4},{:.4},{:.4})",
            pos.x, pos.y, pos.z, pos.w
        );
        println!(
            "[CAM {label}] forward=({:.4},{:.4},{:.4},{:.4}) ana=({:.4},{:.4},{:.4},{:.4})",
            fwd.x, fwd.y, fwd.z, fwd.w, ana.x, ana.y, ana.z, ana.w
        );
        println!(
            "[CAM {label}] slice_w(tesseract)={:.6}",
            self.camera_space_w(TESSERACT_CENTER)
        );
    }
}

/// Walk with the given key held for `frames`, returning the maximum absolute
/// drift of the tesseract's camera-space W relative to the start.
fn max_drift_while_walking(rig: &mut TestRig, key: KeyCode, frames: usize) -> f32 {
    let w_start = rig.camera_space_w(TESSERACT_CENTER);
    let mut max_drift: f32 = 0.0;

    rig.press(key);
    for frame in 0..frames {
        rig.step();
        let w_now = rig.camera_space_w(TESSERACT_CENTER);
        let drift = (w_now - w_start).abs();
        max_drift = max_drift.max(drift);
        if frame % 15 == 0 {
            let pos = rig.camera.position;
            println!(
                "[MOVE frame {frame:3}] cam_pos=({:.4},{:.4},{:.4},{:.4}) slice_w={:.6} drift={:.6}",
                pos.x, pos.y, pos.z, pos.w, w_now, drift
            );
        }
    }
    rig.release(key);
    max_drift
}

/// Let the player body settle onto the floor so gravity transients don't
/// pollute the measurement window (Y movement can't affect the invariant —
/// ana.y is always 0 — but a clean baseline makes failures unambiguous).
fn settle(rig: &mut TestRig) {
    rig.run(120);
}

const DRIFT_TOLERANCE: f32 = 1e-3;

// ============================================================================
// Control: no 4D rotation
// ============================================================================

#[test]
fn wasd_without_rotation_keeps_slice_plane() {
    let mut rig = TestRig::new();
    settle(&mut rig);
    rig.debug_state("baseline");

    for key in [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD] {
        let drift = max_drift_while_walking(&mut rig, key, 60);
        assert!(
            drift < DRIFT_TOLERANCE,
            "slice plane drifted {drift} during {key:?} with no 4D rotation"
        );
    }
}

// ============================================================================
// THE BUG: WASD after a ZW rotation must not drift the slice plane
// ============================================================================

#[test]
fn wasd_after_45deg_zw_rotation_keeps_slice_plane() {
    let mut rig = TestRig::new();
    settle(&mut rig);

    // Rotate 45° in ZW — the worst case for anisotropic speed scaling
    rig.camera.rotate_w(FRAC_PI_4);
    rig.debug_state("after 45° ZW rotation");

    for key in [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD] {
        let drift = max_drift_while_walking(&mut rig, key, 60);
        assert!(
            drift < DRIFT_TOLERANCE,
            "SLICE PLANE DRIFTED {drift} during {key:?} after 45° ZW rotation \
             — this is the shape-morphing bug"
        );
    }
}

#[test]
fn wasd_after_combined_4d_rotations_keeps_slice_plane() {
    let mut rig = TestRig::new();
    settle(&mut rig);

    // A messier orientation: ZW + XW + yaw
    rig.camera.rotate_w(0.6);
    rig.camera.rotate_xw(0.35);
    rig.camera.rotate_3d(0.8, 0.0);
    rig.debug_state("after combined rotations");

    for key in [KeyCode::KeyW, KeyCode::KeyD] {
        let drift = max_drift_while_walking(&mut rig, key, 60);
        assert!(
            drift < DRIFT_TOLERANCE,
            "slice plane drifted {drift} during {key:?} after combined 4D rotations"
        );
    }
}

#[test]
fn wasd_with_pitch_after_zw_rotation_keeps_slice_plane() {
    let mut rig = TestRig::new();
    settle(&mut rig);

    rig.camera.rotate_w(FRAC_PI_4);
    rig.camera.rotate_3d(0.0, 0.5); // look up — pitched forward movement
    rig.debug_state("after ZW rotation + pitch");

    let drift = max_drift_while_walking(&mut rig, KeyCode::KeyW, 60);
    assert!(
        drift < DRIFT_TOLERANCE,
        "slice plane drifted {drift} during pitched forward movement after ZW rotation"
    );
}

// ============================================================================
// Sanity: deliberate W movement and rotation SHOULD change the slice
// ============================================================================

#[test]
fn qe_movement_deliberately_changes_slice() {
    let mut rig = TestRig::new();
    settle(&mut rig);

    let w_before = rig.camera_space_w(TESSERACT_CENTER);
    rig.press(KeyCode::KeyQ);
    rig.run(60);
    rig.release(KeyCode::KeyQ);
    let w_after = rig.camera_space_w(TESSERACT_CENTER);

    let delta = (w_after - w_before).abs();
    println!("[Q/E] slice W moved by {delta:.4} over 1s (expected ~{W_MOVE_SPEED})");
    assert!(
        delta > 0.5,
        "Q should move the camera along its ana axis (slice W change), got {delta}"
    );
}

#[test]
fn wasd_actually_moves_the_player() {
    let mut rig = TestRig::new();
    settle(&mut rig);
    rig.camera.rotate_w(FRAC_PI_4);

    let before = rig.camera.position;
    rig.press(KeyCode::KeyW);
    rig.run(60);
    rig.release(KeyCode::KeyW);
    let after = rig.camera.position;

    let dist = (after - before).length();
    println!("[SPEED] moved {dist:.4} units in 1s (move_speed={MOVE_SPEED})");
    assert!(
        dist > MOVE_SPEED * 0.8,
        "player should move roughly move_speed units per second, got {dist}"
    );
}
