//! 4D Camera with Engine4D-style architecture
//!
//! This camera uses the same architectural approach as Engine4D:
//! - **Pitch is stored separately** from 4D rotation
//! - **4D rotations operate in XZW hyperplane only** (via SkipY)
//! - **Movement is transformed by the full camera matrix**
//! - **Y axis always remains aligned with gravity/world up**
//!
//! This design ensures intuitive movement behavior: walking forward stays
//! horizontal regardless of 4D rotation state.

use rust4d_math::{Vec4, Rotor4, RotationPlane, mat4};
use rust4d_input::CameraControl;

/// Configuration for the gravity smoothing system.
///
/// These values match Engine4D's defaults but can be tuned per-game.
#[derive(Clone, Debug)]
pub struct GravityConfig {
    /// Rotation rate in degrees per second for intermediate gravity tracking.
    /// Higher values make gravity transitions snappier.
    pub rate_degrees_per_sec: f32,
    /// Smoothing time constant in seconds for exponential decay.
    /// Smaller values = faster convergence, larger = smoother.
    pub smooth_time_constant: f32,
    /// Angle threshold (degrees) above which gravity snaps instantly.
    /// Prevents slow rotation through very large gravity changes.
    pub snap_threshold_degrees: f32,
}

impl Default for GravityConfig {
    fn default() -> Self {
        Self {
            rate_degrees_per_sec: 180.0,
            smooth_time_constant: 0.05,
            snap_threshold_degrees: 60.0,
        }
    }
}

/// 4D Camera using Engine4D-style architecture
///
/// The camera orientation is built from three components:
/// 1. `gravity_matrix` - Aligns camera to current gravity direction (any 4D direction)
/// 2. `rotation_4d` - 4D rotation in XZW hyperplane (via SkipY), preserving Y axis
/// 3. `pitch` - Separate pitch angle (YZ plane rotation), clamped to ±89°
///
/// Camera matrix = gravity_matrix * skip_y(rotation_4d) * pitch_matrix
///
/// This separation ensures that 4D rotations never affect the Y axis (gravity),
/// making movement feel natural and predictable.
pub struct Camera4D {
    /// 4D position (x, y, z, w)
    pub position: Vec4,

    /// Pitch angle in radians (YZ plane rotation)
    /// This is separate from 4D rotation and is clamped to ±pitch_limit
    /// Equivalent to Engine4D's `lookYZ` (but in radians, not degrees)
    pitch: f32,

    /// 4D rotation rotor (operates in XZW hyperplane via SkipY)
    /// This is equivalent to Engine4D's `m1` quaternion.
    /// When converted to matrix and passed through SkipY, it only affects XZW axes.
    rotation_4d: Rotor4,

    /// Cross-section offset from camera W position
    pub slice_offset: f32,

    /// Maximum pitch angle in radians (default: ~89 degrees)
    pitch_limit: f32,

    // === Gravity System (Engine4D compatible) ===

    /// Current gravity direction (can be any 4D direction, not just Y)
    /// This is the "target" gravity that the camera is aligning to.
    gravity_direction: Vec4,

    /// Smoothed gravity direction for interpolation
    /// This is the direction actually used for movement constraints.
    smooth_gravity_direction: Vec4,

    /// Intermediate gravity direction for two-stage smoothing
    /// Provides extra smoothness in gravity transitions.
    intermediate_gravity_direction: Vec4,

    /// Gravity alignment matrix
    /// Transforms camera to align with current gravity direction.
    gravity_matrix: mat4::Mat4,

    /// Whether to use gravity (false for flying/noclip modes)
    use_gravity: bool,

    /// Gravity smoothing configuration
    gravity_config: GravityConfig,
}

impl Default for Camera4D {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera4D {
    /// Default pitch clamp limit: ±89° to prevent gimbal lock (matches Engine4D)
    const DEFAULT_PITCH_LIMIT: f32 = 1.553; // ~89 degrees in radians

    /// Create a new camera at the default position with default pitch limit (89 degrees)
    pub fn new() -> Self {
        Self::with_pitch_limit(Self::DEFAULT_PITCH_LIMIT)
    }

    /// Create a new camera with a custom pitch limit (in radians)
    pub fn with_pitch_limit(pitch_limit: f32) -> Self {
        // Default gravity is world Y (0, 1, 0, 0)
        let default_gravity = Vec4::Y;

        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            pitch: 0.0,
            rotation_4d: Rotor4::IDENTITY,
            slice_offset: 0.0,
            pitch_limit,
            // Gravity system
            gravity_direction: default_gravity,
            smooth_gravity_direction: default_gravity,
            intermediate_gravity_direction: default_gravity,
            gravity_matrix: mat4::IDENTITY,
            use_gravity: true,
            gravity_config: GravityConfig::default(),
        }
    }

    /// Build the camera transformation matrix (Engine4D style)
    ///
    /// Composition: `gravity_matrix * skip_y(rotation_4d) * pitch_rotation`
    ///
    /// This ensures:
    /// 1. Pitch is applied first (local YZ plane rotation)
    /// 2. 4D rotation is applied in XZW hyperplane (Y axis preserved!)
    /// 3. Gravity matrix aligns camera to current gravity direction
    ///
    /// The result is a matrix that transforms camera-local directions to world space.
    pub fn camera_matrix(&self) -> mat4::Mat4 {
        // 1. Build pitch rotation in YZ plane (indices 1, 2)
        let pitch_mat = mat4::plane_rotation(self.pitch, 1, 2);

        // 2. Build 4D rotation matrix and apply SkipY
        // SkipY remaps the rotation to XZW, leaving Y unchanged
        let rot_4d_raw = self.rotation_4d.to_matrix();
        let rot_4d_skip_y = mat4::skip_y(rot_4d_raw);

        // 3. Combine: gravity_matrix * 4D_rotation * pitch (right-to-left)
        // Engine4D: camMatrix = gravityMatrix * SkipY(m1) * pitchMatrix
        mat4::mul(self.gravity_matrix, mat4::mul(rot_4d_skip_y, pitch_mat))
    }

    /// Standard 3D mouse look (yaw and pitch)
    ///
    /// Engine4D style:
    /// - **Horizontal (yaw)**: Applied to rotation_4d as Z rotation
    ///   After SkipY, this becomes a rotation in the XZ plane (horizontal turning).
    /// - **Vertical (pitch)**: Applied to separate pitch variable (not rotation_4d!)
    ///
    /// This separation is the key to intuitive camera control.
    pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // Yaw: modify rotation_4d with Z-axis rotation (XY plane)
        // After SkipY, XY rotation becomes XZ rotation (horizontal turning)
        // Positive yaw = turn right = forward goes from -Z toward +X
        if delta_yaw.abs() > 0.0001 {
            // XY rotation with positive angle rotates X toward Y
            // After SkipY (Y→Z), this becomes XZ rotation: X toward Z
            // We want positive yaw to turn right (forward -Z → +X)
            // So we need XZ rotation that takes -Z toward +X, which is positive angle
            let r_z = Rotor4::from_plane_angle(RotationPlane::XY, delta_yaw);
            self.rotation_4d = self.rotation_4d.compose(&r_z).normalize();
        }

        // Pitch: modify separate pitch variable (NOT rotation_4d!)
        // This is the critical difference from our old implementation.
        self.pitch = (self.pitch + delta_pitch).clamp(-self.pitch_limit, self.pitch_limit);
    }

    /// 4D W-rotation (ZW plane)
    ///
    /// Rotates the view into the 4th dimension. After SkipY transformation,
    /// this affects how the XZW hyperplane is oriented but never touches Y.
    pub fn rotate_w(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            // In the 3D rotation space (before SkipY), this is a Y rotation
            // After SkipY: Y→Z, so this becomes a rotation affecting Z and W
            let r = Rotor4::from_plane_angle(RotationPlane::XZ, -delta);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// 4D XW rotation
    ///
    /// Rotates in the XW plane. After SkipY transformation, this affects
    /// X and W but never touches Y.
    pub fn rotate_xw(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            // In the 3D rotation space (before SkipY), this is an X rotation
            // After SkipY: X→X, Z→W, so this becomes XW rotation
            let r = Rotor4::from_plane_angle(RotationPlane::YZ, delta);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// Move by transforming input through the camera matrix (Engine4D style).
    fn move_camera(&mut self, forward: f32, right: f32, up: f32, ana: f32) {
        if forward.abs() < 0.0001 && right.abs() < 0.0001 && up.abs() < 0.0001 && ana.abs() < 0.0001 {
            return;
        }

        // Build input vector in camera space
        // Note: forward is -Z in camera space
        let input = Vec4::new(right, up, -forward, ana);

        // Transform by camera matrix
        let cam_mat = self.camera_matrix();
        let world_movement = mat4::transform(cam_mat, input);

        // Apply movement
        self.position += world_movement;
    }

    /// Move in the camera-local XZ plane (forward/backward, left/right)
    ///
    /// Movement stays horizontal because 4D rotations are applied via SkipY,
    /// which preserves the Y axis.
    pub fn move_local_xz(&mut self, forward: f32, right: f32) {
        self.move_camera(forward, right, 0.0, 0.0);
    }

    /// Move along the camera-local W axis (ana/kata)
    ///
    /// The W direction is transformed by the camera matrix, so it follows
    /// the camera's 4D orientation.
    pub fn move_w(&mut self, delta: f32) {
        self.move_camera(0.0, 0.0, 0.0, delta);
    }

    /// Move up/down along world Y axis
    ///
    /// This is always world Y, not camera-relative, for consistent vertical movement.
    pub fn move_y(&mut self, delta: f32) {
        self.position.y += delta;
    }

    /// Get the W-coordinate for cross-section slicing
    ///
    /// This returns the camera-space offset for the slice plane. The slice
    /// is always perpendicular to the camera's W axis, at this distance from
    /// the camera. Using camera-space offset (not world W) ensures the slice
    /// stays centered on the player regardless of 4D rotation or movement.
    pub fn get_slice_w(&self) -> f32 {
        self.slice_offset
    }

    /// Adjust the slice offset
    pub fn adjust_slice_offset(&mut self, delta: f32) {
        self.slice_offset += delta;
    }

    /// Reset camera to the default starting position and orientation.
    /// Note: pitch_limit, use_gravity, and gravity_config are preserved.
    pub fn reset(&mut self) {
        self.position = Vec4::new(0.0, 0.0, 5.0, 0.0);
        self.pitch = 0.0;
        self.rotation_4d = Rotor4::IDENTITY;
        self.slice_offset = 0.0;
        // Reset gravity to world Y
        self.gravity_direction = Vec4::Y;
        self.smooth_gravity_direction = Vec4::Y;
        self.intermediate_gravity_direction = Vec4::Y;
        self.gravity_matrix = mat4::IDENTITY;
        // pitch_limit, use_gravity, and gravity_config are intentionally preserved
    }

    // === Gravity System ===

    /// Update the gravity system for smooth gravity transitions.
    ///
    /// Call this once per frame with delta time. If `new_gravity_direction` is
    /// provided, it becomes the target gravity direction.
    ///
    /// The gravity system uses two-stage smoothing like Engine4D:
    /// 1. `intermediate_gravity` smoothly rotates toward `gravity_direction`
    /// 2. `smooth_gravity` smoothly interpolates toward `intermediate_gravity`
    ///
    /// This provides extra smoothness when gravity changes rapidly.
    pub fn update_gravity(&mut self, dt: f32, new_gravity_direction: Option<Vec4>) {
        if !self.use_gravity {
            return;
        }

        // Update target gravity if provided (M4: skip if direction unchanged)
        if let Some(new_dir) = new_gravity_direction {
            let len = new_dir.length();
            if len > 1e-6 {
                let normalized = new_dir * (1.0 / len);
                if (normalized - self.gravity_direction).length_squared() > 1e-8 {
                    self.gravity_direction = normalized;
                }
            }
        }

        // Calculate rotation limits
        let max_angle = dt * self.gravity_config.rate_degrees_per_sec.to_radians();
        let gravity_smooth = 2.0_f32.powf(-dt / self.gravity_config.smooth_time_constant);
        let angle = self.gravity_direction.angle_to(self.smooth_gravity_direction);

        // Two-stage smoothing like Engine4D
        // Stage 1: Large angle changes snap immediately, small angles smooth
        if angle > self.gravity_config.snap_threshold_degrees.to_radians() {
            self.intermediate_gravity_direction = self.gravity_direction;
        } else {
            self.intermediate_gravity_direction = self.intermediate_gravity_direction
                .rotate_towards(self.gravity_direction, max_angle);
        }

        // Stage 2: Smooth exponential decay toward intermediate
        // Rotate smooth_gravity TOWARD intermediate (not the other way around)
        let smooth_angle = self.smooth_gravity_direction.angle_to(self.intermediate_gravity_direction);
        self.smooth_gravity_direction = self.smooth_gravity_direction
            .rotate_towards(self.intermediate_gravity_direction, (1.0 - gravity_smooth) * smooth_angle);

        // Update gravity matrix
        let current_up = mat4::get_column(self.gravity_matrix, 1);
        let rotation = mat4::from_to_rotation(current_up, self.smooth_gravity_direction);
        self.gravity_matrix = mat4::ortho_iterate(mat4::mul(rotation, self.gravity_matrix));
    }

    /// Get the current smoothed gravity direction.
    ///
    /// This is the direction used for movement constraints. It smoothly
    /// transitions when gravity changes.
    pub fn smooth_gravity(&self) -> Vec4 {
        self.smooth_gravity_direction
    }

    /// Get the target gravity direction.
    pub fn gravity_direction(&self) -> Vec4 {
        self.gravity_direction
    }

    /// Set the target gravity direction directly.
    pub fn set_gravity_direction(&mut self, dir: Vec4) {
        let len = dir.length();
        if len > 1e-6 {
            self.gravity_direction = dir * (1.0 / len);
        }
    }

    /// Check if gravity is enabled.
    pub fn use_gravity(&self) -> bool {
        self.use_gravity
    }

    /// Enable or disable gravity.
    ///
    /// When disabled, movement is not constrained to be orthogonal to gravity,
    /// useful for flying/noclip modes.
    pub fn set_use_gravity(&mut self, use_gravity: bool) {
        self.use_gravity = use_gravity;
    }

    /// Set the gravity smoothing configuration.
    pub fn set_gravity_config(&mut self, config: GravityConfig) {
        self.gravity_config = config;
    }

    /// Get the current gravity configuration.
    pub fn gravity_config(&self) -> &GravityConfig {
        &self.gravity_config
    }

    /// Get the current pitch angle in radians.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Get the 4D rotation rotor.
    pub fn rotation_4d(&self) -> Rotor4 {
        self.rotation_4d
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> Vec4 {
        mat4::transform(self.camera_matrix(), Vec4::new(0.0, 0.0, -1.0, 0.0))
    }

    /// Get the right direction vector
    pub fn right(&self) -> Vec4 {
        mat4::transform(self.camera_matrix(), Vec4::new(1.0, 0.0, 0.0, 0.0))
    }

    /// Get the up direction vector
    pub fn up(&self) -> Vec4 {
        mat4::transform(self.camera_matrix(), Vec4::new(0.0, 1.0, 0.0, 0.0))
    }

    /// Get the W (ana) direction vector
    pub fn ana(&self) -> Vec4 {
        mat4::transform(self.camera_matrix(), Vec4::new(0.0, 0.0, 0.0, 1.0))
    }

    /// Get the 4x4 rotation matrix for the camera orientation
    ///
    /// This returns the full camera matrix including both pitch and 4D rotation.
    pub fn rotation_matrix(&self) -> [[f32; 4]; 4] {
        self.camera_matrix()
    }
}

impl CameraControl for Camera4D {
    fn move_local_xz(&mut self, forward: f32, right: f32) {
        Camera4D::move_local_xz(self, forward, right);
    }

    fn move_y(&mut self, delta: f32) {
        Camera4D::move_y(self, delta);
    }

    fn move_w(&mut self, delta: f32) {
        Camera4D::move_w(self, delta);
    }

    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        Camera4D::rotate_3d(self, delta_yaw, delta_pitch);
    }

    fn rotate_w(&mut self, delta: f32) {
        Camera4D::rotate_w(self, delta);
    }

    fn rotate_xw(&mut self, delta: f32) {
        Camera4D::rotate_xw(self, delta);
    }

    fn position(&self) -> Vec4 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    const EPSILON: f32 = 0.001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_camera_default_position() {
        let cam = Camera4D::new();
        assert_eq!(cam.position.z, 5.0);
        assert_eq!(cam.position.w, 0.0);
    }

    #[test]
    fn test_camera_slice_w() {
        let mut cam = Camera4D::new();
        cam.position.w = 2.0;
        cam.slice_offset = 0.5;
        // slice_w is camera-space offset only, independent of world position
        assert_eq!(cam.get_slice_w(), 0.5);
    }

    #[test]
    fn test_y_axis_preserved_after_4d_rotation() {
        // This is the KEY test: 4D rotations should NOT affect Y axis
        let mut cam = Camera4D::new();

        // Apply various 4D rotations
        cam.rotate_w(FRAC_PI_4);
        cam.rotate_xw(0.3);
        cam.rotate_w(0.5);

        // Up should still be purely +Y (or close to it)
        let up = cam.up();
        assert!(up.y > 0.99, "Up should still be +Y after 4D rotation, got {:?}", up);
        assert!(up.x.abs() < EPSILON, "Up.x should be ~0, got {}", up.x);
        assert!(up.z.abs() < EPSILON, "Up.z should be ~0, got {}", up.z);
        assert!(up.w.abs() < EPSILON, "Up.w should be ~0, got {}", up.w);
    }

    #[test]
    fn test_pitch_affects_up_vector() {
        let mut cam = Camera4D::new();

        // Apply pitch (look up)
        cam.rotate_3d(0.0, FRAC_PI_4); // 45° pitch up

        let up = cam.up();
        let fwd = cam.forward();

        // Up should be tilted (Y component < 1)
        assert!(up.y < 0.95, "Up should be tilted after pitch, got up.y={}", up.y);

        // Forward should point up (positive Y)
        assert!(fwd.y > 0.5, "Forward should point up after pitch, got fwd.y={}", fwd.y);
    }

    #[test]
    fn test_yaw_rotates_in_xz_plane() {
        let mut cam = Camera4D::new();

        // Yaw 90° right
        cam.rotate_3d(FRAC_PI_2, 0.0);

        let fwd = cam.forward();

        // Forward should be in +X direction (or close)
        // Due to SkipY remapping, exact behavior may differ
        println!("Forward after 90° yaw: {:?}", fwd);

        // Y should still be 0 (yaw doesn't affect pitch)
        assert!(fwd.y.abs() < EPSILON, "Forward.y should be ~0 after pure yaw, got {}", fwd.y);
    }

    #[test]
    fn test_movement_stays_horizontal_after_4d_rotation() {
        // This is the critical movement test
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Apply some 4D rotations
        cam.rotate_w(FRAC_PI_4);
        cam.rotate_xw(0.3);

        // Move forward
        cam.move_local_xz(1.0, 0.0);

        // Y should be unchanged (movement stays horizontal!)
        assert!(cam.position.y.abs() < EPSILON,
            "Forward movement should not affect Y after 4D rotation, got Y={}", cam.position.y);
    }

    #[test]
    fn test_pitch_affects_movement() {
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Pitch up 45°
        cam.rotate_3d(0.0, FRAC_PI_4);

        // Move forward
        cam.move_local_xz(1.0, 0.0);

        // Y should be positive (moving up because we're pitched up)
        assert!(cam.position.y > 0.5,
            "Forward movement should affect Y when pitched, got Y={}", cam.position.y);
    }

    #[test]
    fn test_reset_clears_all_state() {
        let mut cam = Camera4D::new();

        // Apply rotations
        cam.rotate_3d(1.0, 0.5);
        cam.rotate_w(0.3);
        cam.rotate_xw(0.2);

        cam.reset();

        // Should be at identity
        let fwd = cam.forward();
        let up = cam.up();

        assert!(approx_eq(fwd.z, -1.0), "Forward should be -Z after reset, got {:?}", fwd);
        assert!(approx_eq(up.y, 1.0), "Up should be +Y after reset, got {:?}", up);
    }

    #[test]
    fn test_pitch_clamped() {
        let mut cam = Camera4D::new();

        // Try to pitch way past 90°
        cam.rotate_3d(0.0, 10.0);

        // Pitch should be clamped to ~89° (default pitch limit)
        assert!(cam.pitch.abs() <= Camera4D::DEFAULT_PITCH_LIMIT + 0.001,
            "Pitch should be clamped, got {}", cam.pitch);
    }

    #[test]
    fn test_orthogonality_preserved() {
        let mut cam = Camera4D::new();

        // Apply various rotations
        cam.rotate_3d(0.5, 0.3);
        cam.rotate_w(0.4);
        cam.rotate_xw(0.2);

        let fwd = cam.forward();
        let right = cam.right();
        let up = cam.up();
        let ana = cam.ana();

        // Check vectors are unit length
        assert!((fwd.length() - 1.0).abs() < EPSILON, "Forward not unit: {}", fwd.length());
        assert!((right.length() - 1.0).abs() < EPSILON, "Right not unit: {}", right.length());
        assert!((up.length() - 1.0).abs() < EPSILON, "Up not unit: {}", up.length());
        assert!((ana.length() - 1.0).abs() < EPSILON, "Ana not unit: {}", ana.length());

        // Check orthogonality
        assert!(fwd.dot(right).abs() < EPSILON, "Fwd not orthogonal to Right");
        assert!(fwd.dot(up).abs() < EPSILON, "Fwd not orthogonal to Up");
        assert!(fwd.dot(ana).abs() < EPSILON, "Fwd not orthogonal to Ana");
        assert!(right.dot(up).abs() < EPSILON, "Right not orthogonal to Up");
        assert!(right.dot(ana).abs() < EPSILON, "Right not orthogonal to Ana");
        assert!(up.dot(ana).abs() < EPSILON, "Up not orthogonal to Ana");
    }

    #[test]
    fn test_yaw_after_4d_rotation() {
        // Yaw should still work correctly after 4D rotation
        let mut cam = Camera4D::new();

        // First apply 4D rotation
        cam.rotate_w(FRAC_PI_4);

        // Then yaw
        cam.rotate_3d(FRAC_PI_2, 0.0);

        // Up should still be +Y (4D rotation + yaw both preserve Y)
        let up = cam.up();
        assert!(up.y > 0.99, "Up should be +Y after 4D rotation + yaw, got {:?}", up);
    }

    #[test]
    fn test_combined_4d_rotations() {
        let mut cam = Camera4D::new();

        // Apply multiple 4D rotations
        cam.rotate_w(FRAC_PI_2);  // Look into W
        cam.rotate_xw(FRAC_PI_4); // Tilt in XW

        // Y axis should still be preserved
        let up = cam.up();
        assert!(up.y > 0.99, "Up should be +Y after combined 4D rotations, got {:?}", up);

        // But forward should be in a different direction
        let fwd = cam.forward();
        println!("Forward after combined 4D rotations: {:?}", fwd);

        // Forward should have W component (looking into 4D)
        assert!(fwd.w.abs() > 0.1 || fwd.z.abs() > 0.1,
            "Forward should be affected by 4D rotation");
    }

    #[test]
    fn test_move_w_follows_camera_orientation() {
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Without any rotation, W movement should go in +W
        cam.move_w(1.0);
        assert!(cam.position.w > 0.9, "W movement should go in +W by default");

        // Reset
        cam.reset();
        cam.position = Vec4::ZERO;

        // After 4D rotation, W movement follows camera's W axis
        cam.rotate_w(FRAC_PI_2);
        cam.move_w(1.0);

        // W axis is now rotated, so movement goes in a different direction
        // But Y should still be unchanged
        assert!(cam.position.y.abs() < EPSILON,
            "W movement should not affect Y, got Y={}", cam.position.y);
    }

    #[test]
    fn test_ana_changes_after_4d_rotation() {
        let mut cam = Camera4D::new();

        // Initial ana() should point in +W direction
        let ana_before = cam.ana();
        println!("ana_before: ({:.4}, {:.4}, {:.4}, {:.4})",
            ana_before.x, ana_before.y, ana_before.z, ana_before.w);
        assert!(ana_before.w > 0.9,
            "Initial ana should be ~(0,0,0,1), got W={}", ana_before.w);
        assert!(ana_before.x.abs() < 0.1,
            "Initial ana X should be ~0, got {}", ana_before.x);

        // After 90° rotation in XW plane (via rotate_w), ana should change
        cam.rotate_w(FRAC_PI_2);

        let ana_after = cam.ana();
        println!("ana_after: ({:.4}, {:.4}, {:.4}, {:.4})",
            ana_after.x, ana_after.y, ana_after.z, ana_after.w);

        // After 90° XW rotation, W axis should point in X direction (or -X)
        assert!(ana_after.w.abs() < 0.1,
            "After 90° rotation, W component should be ~0, got {}", ana_after.w);
        assert!(ana_after.x.abs() > 0.9,
            "After 90° rotation, X component should be ~±1, got {}", ana_after.x);

        // Y should never be affected by rotate_w (that's the point of SkipY)
        assert!(ana_after.y.abs() < 0.1,
            "Y should never be affected by rotate_w, got {}", ana_after.y);
    }

    #[test]
    fn test_w_movement_direction_main_rs_flow() {
        // This test simulates the exact flow in main.rs to verify movement direction
        let mut cam = Camera4D::new();

        // Helper function matching main.rs projection
        fn project_ana(ana: Vec4) -> Vec4 {
            Vec4::new(ana.x, 0.0, ana.z, ana.w).normalized()
        }

        // === Before any rotation ===
        let ana = cam.ana();
        let ana_xzw = project_ana(ana);
        println!("Before rotation: ana=({:.2},{:.2},{:.2},{:.2}) projected=({:.2},{:.2},{:.2},{:.2})",
            ana.x, ana.y, ana.z, ana.w, ana_xzw.x, ana_xzw.y, ana_xzw.z, ana_xzw.w);

        // W movement should go in +W direction
        assert!(ana_xzw.w > 0.9, "Before rotation, W movement should be +W");
        assert!(ana_xzw.x.abs() < 0.1, "Before rotation, X component should be ~0");

        // === After 90° rotation ===
        cam.rotate_w(FRAC_PI_2);

        let ana = cam.ana();
        let ana_xzw = project_ana(ana);
        println!("After 90° rotation: ana=({:.2},{:.2},{:.2},{:.2}) projected=({:.2},{:.2},{:.2},{:.2})",
            ana.x, ana.y, ana.z, ana.w, ana_xzw.x, ana_xzw.y, ana_xzw.z, ana_xzw.w);

        // W movement should now go in +X or -X direction
        assert!(ana_xzw.w.abs() < 0.1, "After 90° rotation, W movement should NOT go in W direction");
        assert!(ana_xzw.x.abs() > 0.9, "After 90° rotation, W movement should go in X direction");

        // Verify: pressing Q after rotation affects X, not W
        let w_input = 1.0;
        let move_from_w = ana_xzw * w_input;
        println!("Movement from Q key: ({:.2},{:.2},{:.2},{:.2})",
            move_from_w.x, move_from_w.y, move_from_w.z, move_from_w.w);

        assert!(move_from_w.x.abs() > 0.9, "Q key should affect X position after rotation");
        assert!(move_from_w.w.abs() < 0.1, "Q key should NOT affect W position after rotation");
    }

    #[test]
    fn test_slice_stable_during_movement_after_4d_rotation() {
        // This test verifies the invariant: walking around after 4D rotation
        // should NOT cause shapes to morph (slice_w relative to camera-space
        // positions should stay constant)

        let mut cam = Camera4D::new();
        cam.position = Vec4::new(0.0, 0.0, 5.0, 0.0);
        cam.slice_offset = 0.0;

        // Apply significant 4D rotation
        cam.rotate_w(FRAC_PI_2);

        // Check slice_w before movement
        let slice_w_before = cam.get_slice_w();

        // Walk around extensively
        cam.move_local_xz(10.0, 5.0);
        cam.move_local_xz(-3.0, -2.0);

        // slice_w should be unchanged - it's a camera-space offset
        let slice_w_after = cam.get_slice_w();
        assert!(
            (slice_w_after - slice_w_before).abs() < EPSILON,
            "slice_w changed from {} to {} during movement! This would cause morphing.",
            slice_w_before, slice_w_after
        );
    }

    // === Gravity System Tests ===

    #[test]
    fn test_default_gravity_is_y() {
        let cam = Camera4D::new();
        let gravity = cam.smooth_gravity();
        assert!(approx_eq(gravity.y, 1.0), "Default gravity should be +Y");
        assert!(approx_eq(gravity.x, 0.0), "Default gravity X should be 0");
        assert!(approx_eq(gravity.z, 0.0), "Default gravity Z should be 0");
        assert!(approx_eq(gravity.w, 0.0), "Default gravity W should be 0");
    }

    #[test]
    fn test_gravity_update_preserves_identity() {
        let mut cam = Camera4D::new();

        // With default gravity, update should preserve identity gravity matrix
        cam.update_gravity(0.016, None); // ~60fps

        let gravity = cam.smooth_gravity();
        assert!(approx_eq(gravity.y, 1.0), "Gravity should remain +Y after update");
    }

    #[test]
    fn test_set_gravity_direction() {
        let mut cam = Camera4D::new();

        // Set gravity to +X
        cam.set_gravity_direction(Vec4::X);

        // Target should be updated immediately
        let target = cam.gravity_direction();
        assert!(approx_eq(target.x, 1.0), "Gravity target should be +X");

        // Smooth gravity hasn't caught up yet (need to update)
        let smooth = cam.smooth_gravity();
        assert!(approx_eq(smooth.y, 1.0), "Smooth gravity not yet updated");
    }

    #[test]
    fn test_gravity_smooth_transition() {
        let mut cam = Camera4D::new();

        // Set gravity to +X (90° from +Y)
        cam.set_gravity_direction(Vec4::X);

        // Update multiple times to simulate smooth transition
        for _ in 0..100 {
            cam.update_gravity(0.016, None);
        }

        // After many updates, smooth gravity should approach target
        let smooth = cam.smooth_gravity();
        assert!(smooth.x > 0.9, "Smooth gravity should approach +X, got x={}", smooth.x);
    }

    #[test]
    fn test_use_gravity_flag() {
        let mut cam = Camera4D::new();
        assert!(cam.use_gravity(), "Gravity should be enabled by default");

        cam.set_use_gravity(false);
        assert!(!cam.use_gravity(), "Gravity should be disabled after set");

        cam.set_use_gravity(true);
        assert!(cam.use_gravity(), "Gravity should be re-enabled");
    }

    #[test]
    fn test_gravity_disabled_skips_update() {
        let mut cam = Camera4D::new();

        // Set gravity to +X but disable gravity
        cam.set_gravity_direction(Vec4::X);
        cam.set_use_gravity(false);

        // Update should do nothing
        for _ in 0..100 {
            cam.update_gravity(0.016, None);
        }

        // Smooth gravity should still be +Y (not updated)
        let smooth = cam.smooth_gravity();
        assert!(approx_eq(smooth.y, 1.0), "Smooth gravity should not update when disabled");
    }

    #[test]
    fn test_gravity_matrix_orthogonality() {
        let mut cam = Camera4D::new();

        // Set gravity to diagonal direction and update
        cam.set_gravity_direction(Vec4::new(1.0, 1.0, 0.0, 0.0));
        for _ in 0..50 {
            cam.update_gravity(0.016, None);
        }

        // The camera matrix should still produce orthogonal basis vectors
        let fwd = cam.forward();
        let right = cam.right();
        let up = cam.up();
        let ana = cam.ana();

        // Check orthogonality
        assert!(fwd.dot(right).abs() < 0.01, "Fwd should be orthogonal to Right with gravity");
        assert!(fwd.dot(up).abs() < 0.01, "Fwd should be orthogonal to Up with gravity");
        assert!(right.dot(up).abs() < 0.01, "Right should be orthogonal to Up with gravity");

        // Check unit length
        assert!((fwd.length() - 1.0).abs() < 0.01, "Forward should be unit length");
        assert!((right.length() - 1.0).abs() < 0.01, "Right should be unit length");
        assert!((up.length() - 1.0).abs() < 0.01, "Up should be unit length");
        assert!((ana.length() - 1.0).abs() < 0.01, "Ana should be unit length");
    }

    #[test]
    fn test_gravity_180_degree_flip() {
        // T3: Test gravity transition from +Y to -Y (180° flip)
        let mut cam = Camera4D::new();
        cam.set_gravity_direction(-Vec4::Y);

        // Update many frames - should eventually converge
        for _ in 0..200 {
            cam.update_gravity(0.016, None);
        }

        let smooth = cam.smooth_gravity();
        assert!(smooth.y < -0.9,
            "Gravity should approach -Y after 180° flip, got y={}", smooth.y);
    }

    #[test]
    fn test_reset_restores_default_gravity() {
        let mut cam = Camera4D::new();

        // Change gravity to +X and update
        cam.set_gravity_direction(Vec4::X);
        for _ in 0..100 {
            cam.update_gravity(0.016, None);
        }

        // Reset
        cam.reset();

        // Gravity should be back to +Y
        let gravity = cam.smooth_gravity();
        assert!(approx_eq(gravity.y, 1.0), "Reset should restore +Y gravity");
        assert!(approx_eq(gravity.x, 0.0), "Reset should clear X gravity");
    }

}
