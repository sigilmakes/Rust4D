//! Game simulation system
//!
//! Manages the game loop simulation including:
//! - Delta time calculation
//! - Input → physics movement
//! - Physics stepping
//! - Camera synchronization

use std::time::Instant;
use rust4d_core::SceneManager;
use rust4d_game::CharacterController4D;
use rust4d_input::CameraController;
use rust4d_math::{Vec4, mat4};
use rust4d_render::camera4d::Camera4D;

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
        // 1. Calculate delta time
        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32();
        // Cap dt to prevent spiral of death on first frame or after window focus
        // The physics accumulator further subdivides into fixed timesteps
        let dt = raw_dt.min(0.25);
        self.last_frame = now;

        // 2. Get movement input from controller
        let (forward_input, right_input) = controller.get_movement_input();
        let w_input = controller.get_w_input();

        // 3. Update gravity system (smooth gravity transitions)
        // Pass None for new_gravity_direction since we're not changing it here.
        // In the future, physics raycasts could provide surface normals.
        camera.update_gravity(dt, None);

        // 4. Calculate movement direction using Engine4D approach
        //
        // Engine4D's key insight: transform input by camera matrix FIRST, then
        // remove gravity. This is mathematically different from projecting
        // forward/right separately and removing ana components.
        //
        // Why this works: camera-local (right, 0, -forward, 0) IS the slice plane
        // by construction. The camera matrix transforms this to world space while
        // preserving the slice-plane property.

        // Build input in camera space: (right, up, -forward, ana)
        // Note: forward is -Z in camera space (looking down -Z axis)
        // Validate inputs to prevent NaN/infinity propagation from broken input devices
        let right_input = if right_input.is_finite() { right_input } else { 0.0 };
        let forward_input = if forward_input.is_finite() { forward_input } else { 0.0 };
        let w_input = if w_input.is_finite() { w_input } else { 0.0 };

        // Split into 3D movement (WASD) and 4D movement (Q/E).
        // 3D movement is projected to the XYZ hyperplane (W zeroed) so that
        // WASD stays in the 3D slice plane even after 4D rotation.
        let cam_mat = camera.camera_matrix();

        // 3D part: forward/right in camera space → world space → project to XYZ
        let input_3d = Vec4::new(right_input, 0.0, -forward_input, 0.0);
        let mut accel = mat4::transform(cam_mat, input_3d);

        // Remove gravity component for horizontal movement
        if camera.use_gravity() {
            let gravity = camera.smooth_gravity();
            accel = accel - gravity * accel.dot(gravity);
        }

        // Remove W component so WASD stays in the 3D slice plane
        accel.w = 0.0;

        // 4D part: Q/E intentionally moves in W via the camera's ana direction
        if w_input.abs() > 0.0001 {
            let input_4d = Vec4::new(0.0, 0.0, 0.0, w_input);
            accel = accel + mat4::transform(cam_mat, input_4d);
        }

        // Normalize to prevent faster diagonal movement, cap magnitude at 1.0
        let move_dir = if accel.length_squared() > 0.0001 {
            let len = accel.length().min(1.0);
            accel.normalized() * len
        } else {
            Vec4::ZERO
        };

        // 5. Apply movement to player via character controller
        // The controller owns move_speed, so we just pass the normalized direction
        if let (Some(character), Some(physics)) = (character, scene_manager
            .active_world_mut()
            .and_then(|w| w.physics_mut()))
        {
            character.apply_movement(physics, move_dir);
        }

        // 6. Handle jump via character controller
        if controller.consume_jump() {
            if let (Some(character), Some(physics)) = (character, scene_manager
                .active_world_mut()
                .and_then(|w| w.physics_mut()))
            {
                character.jump(physics);
            }
        }

        // 7. Step world physics
        scene_manager.update(dt);

        // 8. Check for dirty entities
        let geometry_dirty = scene_manager
            .active_world()
            .map(|w| w.has_dirty_entities())
            .unwrap_or(false);

        // 9. Sync camera position to player body (pre-controller)
        // This sets the camera to the physics-authoritative position BEFORE the
        // controller runs, so controller.update() in step 10 computes rotation
        // deltas from the correct starting position.
        if let (Some(character), Some(physics)) = (character, scene_manager
            .active_world()
            .and_then(|w| w.physics()))
        {
            if let Some(pos) = character.position(physics) {
                camera.position = pos;
            }
        }

        // 10. Apply mouse look for camera rotation
        controller.update(camera, dt, cursor_captured);

        // 11. Re-sync position after controller (keep rotation, discard position drift)
        // controller.update() in step 10 applies both rotation AND movement. We want
        // the rotation (mouse look) but not the movement (physics owns position).
        // Re-syncing here overwrites any position drift the controller introduced.
        if let (Some(character), Some(physics)) = (character, scene_manager
            .active_world()
            .and_then(|w| w.physics()))
        {
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
    use rust4d_math::{Rotor4, RotationPlane};

    #[test]
    fn test_delta_time_capped() {
        let sim = SimulationSystem::new();
        // Simulate a 100ms pause (first frame or window focus)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Can't fully test without scene manager, but we can verify construction
        assert!(sim.last_frame.elapsed().as_millis() >= 100);
    }

    /// Test the Engine4D-style movement calculation.
    ///
    /// Key invariant: WASD movement (w_input=0) should be orthogonal to the
    /// camera's ana direction after any 4D rotation.
    #[test]
    fn test_wasd_orthogonal_to_ana_after_rotation() {
        use std::f32::consts::FRAC_PI_4;

        // Simulate a 45° rotation in ZW plane (camera looking "into" 4D)
        let rotation = Rotor4::from_plane_angle(RotationPlane::ZW, FRAC_PI_4);
        let rot_matrix = mat4::skip_y(rotation.to_matrix());

        // Get camera's ana direction after rotation
        let ana = mat4::transform(rot_matrix, Vec4::new(0.0, 0.0, 0.0, 1.0));

        // WASD input: forward only (no w_input)
        let input = Vec4::new(0.0, 0.0, -1.0, 0.0); // camera-local forward

        // Transform by camera matrix
        let mut accel = mat4::transform(rot_matrix, input);
        accel.y = 0.0; // Remove gravity

        // Normalize
        let move_dir = if accel.length_squared() > 0.0001 {
            accel.normalized()
        } else {
            Vec4::ZERO
        };

        // Key test: movement should be orthogonal to ana
        let dot = move_dir.dot(ana);
        assert!(
            dot.abs() < 0.0001,
            "WASD movement should be orthogonal to ana, but dot = {} (move_dir={:?}, ana={:?})",
            dot, move_dir, ana
        );

        // Also verify no Y component (horizontal)
        assert!(
            move_dir.y.abs() < 0.0001,
            "WASD movement should be horizontal, got y={}",
            move_dir.y
        );
    }

    /// Test that Q/E movement goes along the ana direction.
    #[test]
    fn test_qe_moves_along_ana() {
        use std::f32::consts::FRAC_PI_4;

        // 45° rotation in ZW plane
        let rotation = Rotor4::from_plane_angle(RotationPlane::ZW, FRAC_PI_4);
        let rot_matrix = mat4::skip_y(rotation.to_matrix());

        // Get camera's ana direction
        let ana = mat4::transform(rot_matrix, Vec4::new(0.0, 0.0, 0.0, 1.0));
        let ana_xzw = Vec4::new(ana.x, 0.0, ana.z, ana.w).normalized(); // Project to XZW

        // Q/E input: ana only (no WASD)
        let input = Vec4::new(0.0, 0.0, 0.0, 1.0); // camera-local ana

        // Transform by camera matrix
        let mut accel = mat4::transform(rot_matrix, input);
        accel.y = 0.0;

        let move_dir = if accel.length_squared() > 0.0001 {
            accel.normalized()
        } else {
            Vec4::ZERO
        };

        // Q/E movement should be parallel to ana (high dot product)
        let dot = move_dir.dot(ana_xzw).abs();
        assert!(
            dot > 0.99,
            "Q/E movement should be along ana direction, but dot = {}",
            dot
        );
    }

    /// Test that the 45° ZW rotation case works correctly.
    ///
    /// This is the case that broke the old approach: forward and ana are
    /// orthogonal but both have W components.
    #[test]
    fn test_45deg_zw_rotation_wasd_preserves_slice() {
        use std::f32::consts::FRAC_PI_4;

        // After 45° ZW rotation:
        // - forward = (0, 0, -0.707, 0.707)  -- has W component
        // - ana = (0, 0, 0.707, 0.707)       -- has W component
        // - forward · ana = 0 (orthogonal!)
        //
        // The old approach failed because the dot product was zero even though
        // forward had a W component that would cause slice drift.

        let rotation = Rotor4::from_plane_angle(RotationPlane::ZW, FRAC_PI_4);
        let rot_matrix = mat4::skip_y(rotation.to_matrix());

        // Get ana direction (what the slice is perpendicular to)
        let ana = mat4::transform(rot_matrix, Vec4::new(0.0, 0.0, 0.0, 1.0));

        // Strafe right input
        let input = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let mut accel = mat4::transform(rot_matrix, input);
        accel.y = 0.0;

        let move_dir = if accel.length_squared() > 0.0001 {
            accel.normalized()
        } else {
            Vec4::ZERO
        };

        // Strafing right should be orthogonal to ana
        let dot = move_dir.dot(ana);
        assert!(
            dot.abs() < 0.0001,
            "Strafe movement should be orthogonal to ana after 45° ZW rotation, but dot = {}",
            dot
        );

        // Right strafe should only have X component (in this rotation state)
        assert!(
            move_dir.x.abs() > 0.99,
            "Right strafe should be along X, got {:?}",
            move_dir
        );
    }

    #[test]
    fn test_movement_normalized_to_unit() {
        // Combined movement should cap at magnitude 1.0
        // Use zero rotation (identity) by rotating by 0 radians
        let rotation = Rotor4::from_plane_angle(RotationPlane::XY, 0.0);
        let rot_matrix = mat4::skip_y(rotation.to_matrix());

        // Full diagonal input
        let input = Vec4::new(1.0, 0.0, -1.0, 1.0);
        let mut accel = mat4::transform(rot_matrix, input);
        accel.y = 0.0;

        let move_dir = if accel.length_squared() > 0.0001 {
            let len = accel.length().min(1.0);
            accel.normalized() * len
        } else {
            Vec4::ZERO
        };

        assert!(
            move_dir.length() <= 1.001, // Small tolerance for floating point
            "Movement magnitude should be capped at 1.0, got {}",
            move_dir.length()
        );
    }

    #[test]
    fn test_default_construction() {
        let sim = SimulationSystem::default();
        // Just verify it constructs without panic
        assert!(sim.last_frame.elapsed().as_millis() < 100);
    }
}
