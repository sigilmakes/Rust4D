//! Game simulation system
//!
//! Manages the game loop simulation including:
//! - Delta time calculation
//! - Input → physics movement
//! - Physics stepping
//! - Camera synchronization

use rust4d_core::SceneManager;
use rust4d_game::CharacterController4D;
use rust4d_input::CameraController;
use rust4d_math::Vec4;
use rust4d_render::camera4d::Camera4D;
use std::time::Instant;

/// Result of a simulation update
pub struct SimulationResult {
    /// Whether geometry needs to be rebuilt and re-uploaded
    pub geometry_dirty: bool,
}

/// Manages the game simulation loop
///
/// Handles:
/// - Delta time calculation
/// - Input → physics movement
/// - Physics stepping
/// - Camera synchronization
pub struct SimulationSystem {
    last_frame: Instant,
}

impl SimulationSystem {
    /// Create a new simulation system
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
        }
    }

    /// Run one simulation frame
    ///
    /// # Arguments
    /// * `scene_manager` - Scene manager containing world and physics
    /// * `camera` - 4D camera to sync position to
    /// * `controller` - Input controller for movement/rotation
    /// * `character` - Character controller for player movement (None if no player body)
    /// * `cursor_captured` - Whether cursor is captured (enables mouse look)
    ///
    /// # Returns
    /// SimulationResult with dirty flag and delta time
    pub fn update(
        &mut self,
        scene_manager: &mut SceneManager,
        camera: &mut Camera4D,
        controller: &mut CameraController,
        character: Option<&CharacterController4D>,
        cursor_captured: bool,
    ) -> SimulationResult {
        // Calculate delta time
        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32();
        // Cap dt to prevent spiral of death on first frame or after window focus
        // The physics accumulator further subdivides into fixed timesteps
        let dt = raw_dt.min(0.25);
        self.last_frame = now;

        self.update_with_dt(
            dt,
            scene_manager,
            camera,
            controller,
            character,
            cursor_captured,
        )
    }

    /// Run one simulation frame with an explicit delta time.
    ///
    /// This is the deterministic core of [`update`](Self::update); tests and
    /// headless harnesses call it directly with a fixed timestep.
    pub fn update_with_dt(
        &mut self,
        dt: f32,
        scene_manager: &mut SceneManager,
        camera: &mut Camera4D,
        controller: &mut CameraController,
        character: Option<&CharacterController4D>,
        cursor_captured: bool,
    ) -> SimulationResult {
        // 2. Get movement input from controller
        let (forward_input, right_input) = controller.get_movement_input();
        let w_input = controller.get_w_input();

        // Guard against NaN/infinity from broken input state
        let forward_input = if forward_input.is_finite() {
            forward_input
        } else {
            0.0
        };
        let right_input = if right_input.is_finite() {
            right_input
        } else {
            0.0
        };
        let w_input = if w_input.is_finite() { w_input } else { 0.0 };

        // 3. Calculate movement direction in world space using camera orientation
        //
        // INVARIANT (see tests/slice_invariant.rs): WASD movement must stay
        // inside the player's current 3D slice — its world-space direction
        // must remain orthogonal to the camera's ana axis. Otherwise the
        // camera drifts across the slice plane and every cross-section on
        // screen morphs. Two things guarantee the invariant here:
        //
        //   1. forward/right are projected to the horizontal XZW hyperplane
        //      (Y zeroed). They remain orthogonal to ana because ana never has
        //      a Y component (SkipY construction) and rotation preserves
        //      orthogonality.
        //   2. Slice movement (WASD) and ana movement (Q/E) are passed to the
        //      character controller SEPARATELY, so each is speed-scaled
        //      uniformly. Scaling world axes anisotropically would tilt the
        //      direction across the slice plane.
        let camera_forward = camera.forward();
        let camera_right = camera.right();
        let camera_ana = camera.ana();

        // Project to XZW hyperplane (zero out Y for horizontal movement)
        let forward_xzw =
            Vec4::new(camera_forward.x, 0.0, camera_forward.z, camera_forward.w).normalized();
        let right_xzw = Vec4::new(camera_right.x, 0.0, camera_right.z, camera_right.w).normalized();
        let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();

        // Combine WASD slice movement; clamp to unit length to prevent faster
        // diagonal movement (otherwise 2-axis movement is ~41% faster)
        let slice_dir = forward_xzw * forward_input + right_xzw * right_input;
        let slice_dir = if slice_dir.length_squared() > 1.0 {
            slice_dir.normalized()
        } else {
            slice_dir
        };

        // Q/E deliberately moves along the camera's ana axis (this is the one
        // movement that is SUPPOSED to change the visible slice)
        let ana_dir = ana_xzw * w_input;

        // 4. Apply movement to player via character controller
        // The controller owns the speeds and scales each component uniformly
        if let (Some(character), Some(physics)) = (
            character,
            scene_manager
                .active_world_mut()
                .and_then(|w| w.physics_mut()),
        ) {
            character.apply_movement(physics, slice_dir, ana_dir);
        }

        // 5. Handle jump via character controller
        if controller.consume_jump() {
            if let (Some(character), Some(physics)) = (
                character,
                scene_manager
                    .active_world_mut()
                    .and_then(|w| w.physics_mut()),
            ) {
                character.jump(physics);
            }
        }

        // 6. Step world physics
        scene_manager.update(dt);

        // 7. Check for dirty entities
        let geometry_dirty = scene_manager
            .active_world()
            .map(|w| w.has_dirty_entities())
            .unwrap_or(false);

        // 8. Sync camera position to player body (pre-controller)
        // This sets the camera to the physics-authoritative position BEFORE the
        // controller runs, so controller.update() in step 9 computes rotation
        // deltas from the correct starting position.
        if let (Some(character), Some(physics)) = (
            character,
            scene_manager.active_world().and_then(|w| w.physics()),
        ) {
            if let Some(pos) = character.position(physics) {
                camera.position = pos;
            }
        }

        // 9. Apply mouse look for camera rotation
        controller.update(camera, dt, cursor_captured);

        // 10. Re-sync position after controller (keep rotation, discard position drift)
        // controller.update() in step 9 applies both rotation AND movement. We want
        // the rotation (mouse look) but not the movement (physics owns position).
        // Re-syncing here overwrites any position drift the controller introduced.
        if let (Some(character), Some(physics)) = (
            character,
            scene_manager.active_world().and_then(|w| w.physics()),
        ) {
            if let Some(pos) = character.position(physics) {
                camera.position = pos;
            }
        }

        SimulationResult { geometry_dirty }
    }
}

impl Default for SimulationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_time_capped() {
        let sim = SimulationSystem::new();
        // Simulate a 100ms pause (first frame or window focus)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Can't fully test without scene manager, but we can verify construction
        assert!(sim.last_frame.elapsed().as_millis() >= 100);
    }

    #[test]
    fn test_default_construction() {
        let sim = SimulationSystem::default();
        // Just verify it constructs without panic
        assert!(sim.last_frame.elapsed().as_millis() < 100);
    }
}
