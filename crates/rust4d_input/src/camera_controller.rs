//! Camera controller for 4D Golf-style input handling
//!
//! Controls:
//! - W/S: Forward/backward (Z)
//! - A/D: Left/right strafe (X)
//! - Q/E: Ana/kata movement (W)
//! - Space/Shift: Up/down (Y)
//! - Mouse drag: 3D camera rotation
//! - Right-click + drag: W-axis rotation

use rust4d_math::Vec4;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

/// Camera controller for handling input
pub struct CameraController {
    // Movement state
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    ana: bool,  // Q - move toward +W (ana)
    kata: bool, // E - move toward -W (kata)

    // Jump state (for physics-based movement)
    jump_pressed: bool,

    // Mouse state
    mouse_pressed: bool,
    w_rotation_mode: bool, // Right-click held
    pending_yaw: f32,
    pending_pitch: f32,

    // Input smoothing state
    smooth_yaw: f32,
    smooth_pitch: f32,

    // Configuration
    pub move_speed: f32,
    pub w_move_speed: f32,
    pub mouse_sensitivity: f32,
    pub w_rotation_sensitivity: f32,
    pub smoothing_half_life: f32, // Exponential smoothing half-life in seconds
    pub smoothing_enabled: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            ana: false,
            kata: false,

            jump_pressed: false,

            mouse_pressed: false,
            w_rotation_mode: false,
            pending_yaw: 0.0,
            pending_pitch: 0.0,

            smooth_yaw: 0.0,
            smooth_pitch: 0.0,

            move_speed: 3.0,
            w_move_speed: 2.0,
            mouse_sensitivity: 0.002, // Standard FPS sensitivity
            w_rotation_sensitivity: 0.005,
            smoothing_half_life: 0.05, // 50ms half-life when enabled
            smoothing_enabled: false,  // Disabled by default for responsive FPS feel
        }
    }

    /// Process keyboard input
    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;

        match key {
            KeyCode::KeyW => {
                self.forward = pressed;
                true
            }
            KeyCode::KeyS => {
                self.backward = pressed;
                true
            }
            KeyCode::KeyA => {
                self.left = pressed;
                true
            }
            KeyCode::KeyD => {
                self.right = pressed;
                true
            }
            KeyCode::KeyQ => {
                self.ana = pressed;
                true
            }
            KeyCode::KeyE => {
                self.kata = pressed;
                true
            }
            KeyCode::Space => {
                self.up = pressed;
                // Also track jump for physics mode
                if pressed {
                    self.jump_pressed = true;
                }
                true
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                self.down = pressed;
                true
            }
            _ => false,
        }
    }

    /// Process mouse button input
    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;

        match button {
            MouseButton::Left => {
                self.mouse_pressed = pressed;
            }
            MouseButton::Right => {
                self.w_rotation_mode = pressed;
            }
            _ => {}
        }
    }

    /// Process mouse movement
    pub fn process_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        self.pending_yaw += delta_x as f32;
        self.pending_pitch += delta_y as f32;
    }

    /// Update the camera based on accumulated input
    ///
    /// When `cursor_captured` is true, free look is enabled (no click required).
    /// Returns the camera position for debug display.
    pub fn update<C: CameraControl>(
        &mut self,
        camera: &mut C,
        dt: f32,
        cursor_captured: bool,
    ) -> Vec4 {
        // Calculate movement deltas
        let fwd = (self.forward as i32 - self.backward as i32) as f32;
        let rgt = (self.right as i32 - self.left as i32) as f32;
        let up_down = (self.up as i32 - self.down as i32) as f32;
        let w = (self.ana as i32 - self.kata as i32) as f32;

        // Apply movement
        camera.move_local_xz(fwd * self.move_speed * dt, rgt * self.move_speed * dt);
        camera.move_y(up_down * self.move_speed * dt);
        camera.move_w(w * self.w_move_speed * dt);

        // Apply exponential smoothing to mouse input (engine4d-style)
        let (yaw_input, pitch_input) = if self.smoothing_enabled && dt > 0.0 {
            // Exponential smoothing: new = old * factor + input * (1 - factor)
            // factor = 2^(-dt / half_life), so smaller half_life = faster response
            let smooth_factor = 2.0f32.powf(-dt / self.smoothing_half_life);
            self.smooth_yaw =
                self.smooth_yaw * smooth_factor + self.pending_yaw * (1.0 - smooth_factor);
            self.smooth_pitch =
                self.smooth_pitch * smooth_factor + self.pending_pitch * (1.0 - smooth_factor);
            (self.smooth_yaw, self.smooth_pitch)
        } else {
            // No smoothing - use raw input
            (self.pending_yaw, self.pending_pitch)
        };

        // Apply rotation
        // Free look when cursor is captured, or when mouse button is pressed
        let can_look = cursor_captured || self.mouse_pressed;
        if can_look || self.w_rotation_mode {
            if self.w_rotation_mode {
                // Right-click: W-rotation mode
                // Horizontal mouse: ZW rotation (roll_w)
                // Vertical mouse: XW rotation (roll_xw)
                camera.rotate_w(yaw_input * self.w_rotation_sensitivity);
                camera.rotate_xw(pitch_input * self.w_rotation_sensitivity);
            } else if can_look {
                // Free look: Standard 3D FPS rotation
                // Mouse right (positive delta_x) should turn camera right (positive yaw)
                // Mouse down (positive delta_y) should look down (negative pitch)
                camera.rotate_3d(
                    yaw_input * self.mouse_sensitivity,
                    -pitch_input * self.mouse_sensitivity,
                );
            }
        }

        // Reset pending mouse movement
        self.pending_yaw = 0.0;
        self.pending_pitch = 0.0;

        camera.position()
    }

    /// Check if any movement keys are pressed
    pub fn is_moving(&self) -> bool {
        self.forward
            || self.backward
            || self.left
            || self.right
            || self.up
            || self.down
            || self.ana
            || self.kata
    }

    /// Toggle input smoothing on/off
    pub fn toggle_smoothing(&mut self) -> bool {
        self.smoothing_enabled = !self.smoothing_enabled;
        // Reset smoothing state when toggling
        self.smooth_yaw = 0.0;
        self.smooth_pitch = 0.0;
        self.smoothing_enabled
    }

    /// Check if smoothing is enabled
    pub fn is_smoothing_enabled(&self) -> bool {
        self.smoothing_enabled
    }

    /// Consume the jump input flag
    ///
    /// Returns true if jump was pressed since last consume, then clears the flag.
    /// Use this for physics-based movement where jump should trigger once per press.
    pub fn consume_jump(&mut self) -> bool {
        let was_pressed = self.jump_pressed;
        self.jump_pressed = false;
        was_pressed
    }

    /// Get raw movement input for physics-based movement
    ///
    /// Returns (forward, right) input values in range -1.0 to 1.0.
    /// Forward is positive when W is pressed, negative when S is pressed.
    /// Right is positive when D is pressed, negative when A is pressed.
    pub fn get_movement_input(&self) -> (f32, f32) {
        let forward = (self.forward as i32 - self.backward as i32) as f32;
        let right = (self.right as i32 - self.left as i32) as f32;
        (forward, right)
    }

    /// Get W-axis (ana/kata) movement input
    ///
    /// Returns input value in range -1.0 to 1.0.
    /// Positive when Q is pressed (ana), negative when E is pressed (kata).
    pub fn get_w_input(&self) -> f32 {
        (self.ana as i32 - self.kata as i32) as f32
    }

    /// Builder: set movement speed
    pub fn with_move_speed(mut self, speed: f32) -> Self {
        self.move_speed = speed;
        self
    }

    /// Builder: set W-axis movement speed
    pub fn with_w_move_speed(mut self, speed: f32) -> Self {
        self.w_move_speed = speed;
        self
    }

    /// Builder: set mouse sensitivity
    pub fn with_mouse_sensitivity(mut self, sensitivity: f32) -> Self {
        self.mouse_sensitivity = sensitivity;
        self
    }

    /// Builder: set W-axis rotation sensitivity
    pub fn with_w_rotation_sensitivity(mut self, sensitivity: f32) -> Self {
        self.w_rotation_sensitivity = sensitivity;
        self
    }

    /// Builder: set smoothing half-life (lower = more responsive)
    pub fn with_smoothing_half_life(mut self, half_life: f32) -> Self {
        self.smoothing_half_life = half_life;
        self
    }

    /// Builder: enable or disable smoothing
    pub fn with_smoothing(mut self, enabled: bool) -> Self {
        self.smoothing_enabled = enabled;
        self
    }
}

/// Trait for camera control
/// Allows the controller to work with different camera implementations
pub trait CameraControl {
    fn move_local_xz(&mut self, forward: f32, right: f32);
    fn move_y(&mut self, delta: f32);
    fn move_w(&mut self, delta: f32);
    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32);
    fn rotate_w(&mut self, delta: f32);
    fn rotate_xw(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event::ElementState;
    use winit::keyboard::KeyCode;

    // ==================== Builder Pattern Tests ====================

    #[test]
    fn test_default_values() {
        let controller = CameraController::new();
        assert_eq!(controller.move_speed, 3.0);
        assert_eq!(controller.w_move_speed, 2.0);
        assert_eq!(controller.mouse_sensitivity, 0.002);
        assert_eq!(controller.w_rotation_sensitivity, 0.005);
        assert_eq!(controller.smoothing_half_life, 0.05);
        assert!(!controller.is_smoothing_enabled());
    }

    #[test]
    fn test_default_trait() {
        let controller = CameraController::default();
        assert_eq!(controller.move_speed, 3.0);
        assert!(!controller.is_smoothing_enabled());
    }

    #[test]
    fn test_builder_move_speed() {
        let controller = CameraController::new().with_move_speed(5.0);
        assert_eq!(controller.move_speed, 5.0);
    }

    #[test]
    fn test_builder_w_move_speed() {
        let controller = CameraController::new().with_w_move_speed(4.0);
        assert_eq!(controller.w_move_speed, 4.0);
    }

    #[test]
    fn test_builder_mouse_sensitivity() {
        let controller = CameraController::new().with_mouse_sensitivity(0.005);
        assert_eq!(controller.mouse_sensitivity, 0.005);
    }

    #[test]
    fn test_builder_w_rotation_sensitivity() {
        let controller = CameraController::new().with_w_rotation_sensitivity(0.01);
        assert_eq!(controller.w_rotation_sensitivity, 0.01);
    }

    #[test]
    fn test_builder_smoothing_half_life() {
        let controller = CameraController::new().with_smoothing_half_life(0.1);
        assert_eq!(controller.smoothing_half_life, 0.1);
    }

    #[test]
    fn test_builder_smoothing() {
        let controller = CameraController::new().with_smoothing(true);
        assert!(controller.is_smoothing_enabled());
    }

    #[test]
    fn test_builder_chaining() {
        let controller = CameraController::new()
            .with_move_speed(5.0)
            .with_w_move_speed(3.0)
            .with_mouse_sensitivity(0.005)
            .with_w_rotation_sensitivity(0.01)
            .with_smoothing(true)
            .with_smoothing_half_life(0.1);

        assert_eq!(controller.move_speed, 5.0);
        assert_eq!(controller.w_move_speed, 3.0);
        assert_eq!(controller.mouse_sensitivity, 0.005);
        assert_eq!(controller.w_rotation_sensitivity, 0.01);
        assert!(controller.is_smoothing_enabled());
        assert_eq!(controller.smoothing_half_life, 0.1);
    }

    // ==================== Key State Tests ====================

    #[test]
    fn test_initial_state_not_moving() {
        let controller = CameraController::new();
        assert!(!controller.is_moving());
    }

    #[test]
    fn test_key_pressed_w() {
        let mut controller = CameraController::new();

        // Initially not moving
        assert!(!controller.is_moving());

        // Press W key
        let handled = controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);
        assert!(handled);
        assert!(controller.is_moving());

        // Release W key
        let handled = controller.process_keyboard(KeyCode::KeyW, ElementState::Released);
        assert!(handled);
        assert!(!controller.is_moving());
    }

    #[test]
    fn test_key_pressed_s() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyS, ElementState::Pressed);
        assert!(controller.is_moving());

        controller.process_keyboard(KeyCode::KeyS, ElementState::Released);
        assert!(!controller.is_moving());
    }

    #[test]
    fn test_key_pressed_a() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyA, ElementState::Pressed);
        assert!(controller.is_moving());
    }

    #[test]
    fn test_key_pressed_d() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyD, ElementState::Pressed);
        assert!(controller.is_moving());
    }

    #[test]
    fn test_key_pressed_q() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyQ, ElementState::Pressed);
        assert!(controller.is_moving());
        assert_eq!(controller.get_w_input(), 1.0);
    }

    #[test]
    fn test_key_pressed_e() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyE, ElementState::Pressed);
        assert!(controller.is_moving());
        assert_eq!(controller.get_w_input(), -1.0);
    }

    #[test]
    fn test_key_pressed_space() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);
        assert!(controller.is_moving());
    }

    #[test]
    fn test_key_pressed_shift() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::ShiftLeft, ElementState::Pressed);
        assert!(controller.is_moving());

        controller.process_keyboard(KeyCode::ShiftLeft, ElementState::Released);
        assert!(!controller.is_moving());

        controller.process_keyboard(KeyCode::ShiftRight, ElementState::Pressed);
        assert!(controller.is_moving());
    }

    #[test]
    fn test_unhandled_key() {
        let mut controller = CameraController::new();

        let handled = controller.process_keyboard(KeyCode::KeyX, ElementState::Pressed);
        assert!(!handled);
        assert!(!controller.is_moving());
    }

    #[test]
    fn test_multiple_keys() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);
        controller.process_keyboard(KeyCode::KeyA, ElementState::Pressed);
        assert!(controller.is_moving());

        // Release one key, still moving
        controller.process_keyboard(KeyCode::KeyW, ElementState::Released);
        assert!(controller.is_moving());

        // Release all keys
        controller.process_keyboard(KeyCode::KeyA, ElementState::Released);
        assert!(!controller.is_moving());
    }

    // ==================== Movement Direction Tests ====================

    #[test]
    fn test_forward_movement() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);

        let (forward, right) = controller.get_movement_input();
        assert_eq!(forward, 1.0);
        assert_eq!(right, 0.0);
    }

    #[test]
    fn test_backward_movement() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyS, ElementState::Pressed);

        let (forward, right) = controller.get_movement_input();
        assert_eq!(forward, -1.0);
        assert_eq!(right, 0.0);
    }

    #[test]
    fn test_right_movement() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyD, ElementState::Pressed);

        let (forward, right) = controller.get_movement_input();
        assert_eq!(forward, 0.0);
        assert_eq!(right, 1.0);
    }

    #[test]
    fn test_left_movement() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyA, ElementState::Pressed);

        let (forward, right) = controller.get_movement_input();
        assert_eq!(forward, 0.0);
        assert_eq!(right, -1.0);
    }

    #[test]
    fn test_diagonal_movement_forward_right() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);
        controller.process_keyboard(KeyCode::KeyD, ElementState::Pressed);

        let (forward, right) = controller.get_movement_input();
        assert_eq!(forward, 1.0);
        assert_eq!(right, 1.0);
    }

    #[test]
    fn test_opposing_keys_cancel_forward_backward() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);
        controller.process_keyboard(KeyCode::KeyS, ElementState::Pressed);

        let (forward, _) = controller.get_movement_input();
        assert_eq!(forward, 0.0); // Forward and back cancel
    }

    #[test]
    fn test_opposing_keys_cancel_left_right() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyA, ElementState::Pressed);
        controller.process_keyboard(KeyCode::KeyD, ElementState::Pressed);

        let (_, right) = controller.get_movement_input();
        assert_eq!(right, 0.0); // Left and right cancel
    }

    #[test]
    fn test_w_axis_input_ana() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyQ, ElementState::Pressed);

        assert_eq!(controller.get_w_input(), 1.0);
    }

    #[test]
    fn test_w_axis_input_kata() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyE, ElementState::Pressed);

        assert_eq!(controller.get_w_input(), -1.0);
    }

    #[test]
    fn test_w_axis_input_cancel() {
        let mut controller = CameraController::new();
        controller.process_keyboard(KeyCode::KeyQ, ElementState::Pressed);
        controller.process_keyboard(KeyCode::KeyE, ElementState::Pressed);

        assert_eq!(controller.get_w_input(), 0.0);
    }

    // ==================== Jump Tests ====================

    #[test]
    fn test_jump_initial_state() {
        let mut controller = CameraController::new();

        // Initially no jump
        assert!(!controller.consume_jump());
    }

    #[test]
    fn test_jump_pressed() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);

        // Jump should be consumable once
        assert!(controller.consume_jump());
        // Second consume should return false
        assert!(!controller.consume_jump());
    }

    #[test]
    fn test_jump_press_and_release() {
        let mut controller = CameraController::new();

        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);
        controller.process_keyboard(KeyCode::Space, ElementState::Released);

        // Jump should still be consumable (it was pressed once)
        assert!(controller.consume_jump());
        assert!(!controller.consume_jump());
    }

    #[test]
    fn test_jump_multiple_presses() {
        let mut controller = CameraController::new();

        // Press space twice without consuming
        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);
        controller.process_keyboard(KeyCode::Space, ElementState::Released);
        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);

        // Should only get one jump per consume
        assert!(controller.consume_jump());
        assert!(!controller.consume_jump());
    }

    // ==================== Mouse Input Tests ====================

    #[test]
    fn test_mouse_motion_accumulation() {
        let mut controller = CameraController::new();

        controller.process_mouse_motion(10.0, 5.0);
        controller.process_mouse_motion(5.0, 3.0);

        // Verify pending values accumulated
        assert_eq!(controller.pending_yaw, 15.0);
        assert_eq!(controller.pending_pitch, 8.0);
    }

    #[test]
    fn test_mouse_button_left() {
        let mut controller = CameraController::new();

        controller.process_mouse_button(MouseButton::Left, ElementState::Pressed);
        assert!(controller.mouse_pressed);

        controller.process_mouse_button(MouseButton::Left, ElementState::Released);
        assert!(!controller.mouse_pressed);
    }

    #[test]
    fn test_mouse_button_right() {
        let mut controller = CameraController::new();

        controller.process_mouse_button(MouseButton::Right, ElementState::Pressed);
        assert!(controller.w_rotation_mode);

        controller.process_mouse_button(MouseButton::Right, ElementState::Released);
        assert!(!controller.w_rotation_mode);
    }

    #[test]
    fn test_mouse_button_other() {
        let mut controller = CameraController::new();

        // Middle button should not affect state
        controller.process_mouse_button(MouseButton::Middle, ElementState::Pressed);
        assert!(!controller.mouse_pressed);
        assert!(!controller.w_rotation_mode);
    }

    // ==================== Smoothing Tests ====================

    #[test]
    fn test_toggle_smoothing() {
        let mut controller = CameraController::new();
        assert!(!controller.is_smoothing_enabled());

        let result = controller.toggle_smoothing();
        assert!(result);
        assert!(controller.is_smoothing_enabled());

        let result = controller.toggle_smoothing();
        assert!(!result);
        assert!(!controller.is_smoothing_enabled());
    }

    #[test]
    fn test_toggle_smoothing_resets_state() {
        let mut controller = CameraController::new();

        // Set some smoothing state
        controller.smooth_yaw = 5.0;
        controller.smooth_pitch = 3.0;

        // Toggle should reset state
        controller.toggle_smoothing();

        assert_eq!(controller.smooth_yaw, 0.0);
        assert_eq!(controller.smooth_pitch, 0.0);
    }

    // ==================== Mock Camera for Update Tests ====================

    /// Mock camera that records all calls for testing
    struct MockCamera {
        pub position: Vec4,
        pub forward_moved: f32,
        pub right_moved: f32,
        pub y_moved: f32,
        pub w_moved: f32,
        pub yaw_rotated: f32,
        pub pitch_rotated: f32,
        pub w_rotated: f32,
        pub xw_rotated: f32,
    }

    impl MockCamera {
        fn new() -> Self {
            Self {
                position: Vec4::ZERO,
                forward_moved: 0.0,
                right_moved: 0.0,
                y_moved: 0.0,
                w_moved: 0.0,
                yaw_rotated: 0.0,
                pitch_rotated: 0.0,
                w_rotated: 0.0,
                xw_rotated: 0.0,
            }
        }
    }

    impl CameraControl for MockCamera {
        fn move_local_xz(&mut self, forward: f32, right: f32) {
            self.forward_moved += forward;
            self.right_moved += right;
        }

        fn move_y(&mut self, delta: f32) {
            self.y_moved += delta;
        }

        fn move_w(&mut self, delta: f32) {
            self.w_moved += delta;
        }

        fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
            self.yaw_rotated += delta_yaw;
            self.pitch_rotated += delta_pitch;
        }

        fn rotate_w(&mut self, delta: f32) {
            self.w_rotated += delta;
        }

        fn rotate_xw(&mut self, delta: f32) {
            self.xw_rotated += delta;
        }

        fn position(&self) -> Vec4 {
            self.position
        }
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_update_forward_movement() {
        let mut controller = CameraController::new().with_move_speed(10.0);
        let mut camera = MockCamera::new();

        controller.process_keyboard(KeyCode::KeyW, ElementState::Pressed);
        controller.update(&mut camera, 0.1, false);

        // forward = 1.0 * 10.0 * 0.1 = 1.0
        assert!((camera.forward_moved - 1.0).abs() < 0.001);
        assert_eq!(camera.right_moved, 0.0);
    }

    #[test]
    fn test_update_strafe_movement() {
        let mut controller = CameraController::new().with_move_speed(10.0);
        let mut camera = MockCamera::new();

        controller.process_keyboard(KeyCode::KeyD, ElementState::Pressed);
        controller.update(&mut camera, 0.1, false);

        assert_eq!(camera.forward_moved, 0.0);
        assert!((camera.right_moved - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_update_vertical_movement() {
        let mut controller = CameraController::new().with_move_speed(10.0);
        let mut camera = MockCamera::new();

        controller.process_keyboard(KeyCode::Space, ElementState::Pressed);
        controller.update(&mut camera, 0.1, false);

        assert!((camera.y_moved - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_update_w_movement() {
        let mut controller = CameraController::new().with_w_move_speed(10.0);
        let mut camera = MockCamera::new();

        controller.process_keyboard(KeyCode::KeyQ, ElementState::Pressed);
        controller.update(&mut camera, 0.1, false);

        // w = 1.0 * 10.0 * 0.1 = 1.0
        assert!((camera.w_moved - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_update_no_rotation_without_capture() {
        let mut controller = CameraController::new();
        let mut camera = MockCamera::new();

        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.1, false); // cursor not captured

        // No rotation should occur without cursor captured or mouse pressed
        assert_eq!(camera.yaw_rotated, 0.0);
        assert_eq!(camera.pitch_rotated, 0.0);
    }

    #[test]
    fn test_update_rotation_with_cursor_captured() {
        let mut controller = CameraController::new().with_mouse_sensitivity(0.01);
        let mut camera = MockCamera::new();

        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.1, true); // cursor captured

        // yaw = 100.0 * 0.01 = 1.0
        // pitch = -50.0 * 0.01 = -0.5 (note the sign flip in update)
        assert!((camera.yaw_rotated - 1.0).abs() < 0.001);
        assert!((camera.pitch_rotated - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn test_update_rotation_with_mouse_pressed() {
        let mut controller = CameraController::new().with_mouse_sensitivity(0.01);
        let mut camera = MockCamera::new();

        controller.process_mouse_button(MouseButton::Left, ElementState::Pressed);
        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.1, false); // cursor not captured but mouse pressed

        assert!((camera.yaw_rotated - 1.0).abs() < 0.001);
        assert!((camera.pitch_rotated - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn test_update_w_rotation_mode() {
        let mut controller = CameraController::new().with_w_rotation_sensitivity(0.01);
        let mut camera = MockCamera::new();

        controller.process_mouse_button(MouseButton::Right, ElementState::Pressed);
        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.1, false);

        // w_rotation = 100.0 * 0.01 = 1.0
        // xw_rotation = 50.0 * 0.01 = 0.5
        assert!((camera.w_rotated - 1.0).abs() < 0.001);
        assert!((camera.xw_rotated - 0.5).abs() < 0.001);

        // 3D rotation should not be applied
        assert_eq!(camera.yaw_rotated, 0.0);
        assert_eq!(camera.pitch_rotated, 0.0);
    }

    #[test]
    fn test_update_clears_pending_mouse() {
        let mut controller = CameraController::new();
        let mut camera = MockCamera::new();

        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.1, true);

        // Pending should be cleared after update
        assert_eq!(controller.pending_yaw, 0.0);
        assert_eq!(controller.pending_pitch, 0.0);
    }

    #[test]
    fn test_update_smoothing_enabled() {
        let mut controller = CameraController::new()
            .with_smoothing(true)
            .with_smoothing_half_life(0.1)
            .with_mouse_sensitivity(0.01);
        let mut camera = MockCamera::new();

        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.016, true);

        // With smoothing, the rotation should be less than direct input
        // because we only take a portion based on the smoothing factor
        assert!(camera.yaw_rotated.abs() < 1.0); // Less than full 100 * 0.01
        assert!(camera.yaw_rotated.abs() > 0.0); // But still some rotation
    }

    #[test]
    fn test_update_smoothing_disabled_direct() {
        let mut controller = CameraController::new()
            .with_smoothing(false)
            .with_mouse_sensitivity(0.01);
        let mut camera = MockCamera::new();

        controller.process_mouse_motion(100.0, 50.0);
        controller.update(&mut camera, 0.016, true);

        // Without smoothing, should get full rotation
        assert!((camera.yaw_rotated - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_update_returns_camera_position() {
        let mut controller = CameraController::new();
        let mut camera = MockCamera::new();
        camera.position = Vec4::new(1.0, 2.0, 3.0, 4.0);

        let pos = controller.update(&mut camera, 0.1, false);

        assert_eq!(pos, Vec4::new(1.0, 2.0, 3.0, 4.0));
    }
}
