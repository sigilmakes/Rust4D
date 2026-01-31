//! Game simulation system
//!
//! Manages the game loop simulation including:
//! - Delta time calculation
//! - Input → physics movement
//! - Physics stepping
//! - Camera synchronization

use std::time::Instant;
use rust4d_core::SceneManager;
use rust4d_input::CameraController;
use rust4d_math::Vec4;
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
    /// * `cursor_captured` - Whether cursor is captured (enables mouse look)
    ///
    /// # Returns
    /// SimulationResult with dirty flag and delta time
    pub fn update(
        &mut self,
        scene_manager: &mut SceneManager,
        camera: &mut Camera4D,
        controller: &mut CameraController,
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

        // 3. Calculate movement direction in world space using camera orientation
        let camera_forward = camera.forward();
        let camera_right = camera.right();
        let camera_ana = camera.ana();

        // Project to XZW hyperplane (zero out Y for horizontal movement)
        let forward_xzw =
            Vec4::new(camera_forward.x, 0.0, camera_forward.z, camera_forward.w).normalized();
        let right_xzw =
            Vec4::new(camera_right.x, 0.0, camera_right.z, camera_right.w).normalized();
        let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();

        // Combine movement direction and normalize to prevent faster diagonal movement
        // Without this, 2-axis movement is ~41% faster and 3-axis is ~73% faster
        let move_dir = forward_xzw * forward_input + right_xzw * right_input + ana_xzw * w_input;
        let move_dir = if move_dir.length_squared() > 1.0 {
            move_dir.normalized()
        } else {
            move_dir
        };

        // 4. Apply movement to player via physics
        let move_speed = controller.move_speed;
        if let Some(physics) = scene_manager
            .active_world_mut()
            .and_then(|w| w.physics_mut())
        {
            physics.apply_player_movement(move_dir * move_speed);
        }

        // 5. Handle jump
        if controller.consume_jump() {
            if let Some(physics) = scene_manager
                .active_world_mut()
                .and_then(|w| w.physics_mut())
            {
                physics.player_jump();
            }
        }

        // 6. Step world physics
        scene_manager.update(dt);

        // 7. Check for dirty entities
        let geometry_dirty = scene_manager
            .active_world()
            .map(|w| w.has_dirty_entities())
            .unwrap_or(false);

        // 8. Sync camera position to player physics (all 4 dimensions)
        if let Some(pos) = scene_manager
            .active_world()
            .and_then(|w| w.physics())
            .and_then(|p| p.player_position())
        {
            camera.position = pos;
        }

        // 9. Apply mouse look for camera rotation
        controller.update(camera, dt, cursor_captured);

        // 10. Re-sync position after controller (discard its movement, keep rotation)
        if let Some(pos) = scene_manager
            .active_world()
            .and_then(|w| w.physics())
            .and_then(|p| p.player_position())
        {
            camera.position = pos;
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
