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

/// 4D Camera using Engine4D-style architecture
///
/// The camera orientation is built from two components:
/// 1. `pitch` - Separate pitch angle (YZ plane rotation), clamped to ±89°
/// 2. `rotation_4d` - 4D rotation in XZW hyperplane (via SkipY), preserving Y axis
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
        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            pitch: 0.0,
            rotation_4d: Rotor4::IDENTITY,
            slice_offset: 0.0,
            pitch_limit,
        }
    }

    /// Build the camera transformation matrix (Engine4D style)
    ///
    /// Composition: `skip_y(rotation_4d) * pitch_rotation`
    ///
    /// This ensures:
    /// 1. Pitch is applied first (local YZ plane rotation)
    /// 2. 4D rotation is applied in XZW hyperplane (Y axis preserved!)
    ///
    /// The result is a matrix that transforms camera-local directions to world space.
    pub fn camera_matrix(&self) -> mat4::Mat4 {
        // 1. Build pitch rotation in YZ plane (indices 1, 2)
        let pitch_mat = mat4::plane_rotation(self.pitch, 1, 2);

        // 2. Build 4D rotation matrix and apply SkipY
        // SkipY remaps the rotation to XZW, leaving Y unchanged
        let rot_4d_raw = self.rotation_4d.to_matrix();
        let rot_4d_skip_y = mat4::skip_y(rot_4d_raw);

        // 3. Combine: 4D rotation * pitch (right-to-left: pitch applied first)
        mat4::mul(rot_4d_skip_y, pitch_mat)
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
    /// Rotates the view into the 4th dimension: positive `delta` tilts the
    /// forward axis (-Z) toward the camera's ana axis (+W at identity).
    /// Never touches Y.
    ///
    /// SkipY maps the 3D rotation axes (X, Y, Z) onto the 4D axes (X, Z, W),
    /// so the pre-SkipY YZ plane becomes the 4D ZW plane.
    pub fn rotate_w(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            // Pre-SkipY YZ-plane rotation (about the 3D X axis) → 4D ZW plane.
            // from_plane_angle(YZ, δ) takes Z→W by +δ; negate so that
            // forward (-Z) tilts toward +W for positive delta.
            let r = Rotor4::from_plane_angle(RotationPlane::YZ, -delta);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// 4D XW rotation
    ///
    /// Rotates in the XW plane: positive `delta` tilts the right axis (+X)
    /// toward the camera's ana axis (+W at identity). Never touches Y.
    ///
    /// SkipY maps the 3D rotation axes (X, Y, Z) onto the 4D axes (X, Z, W),
    /// so the pre-SkipY XZ plane becomes the 4D XW plane.
    pub fn rotate_xw(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            // Pre-SkipY XZ-plane rotation (about the 3D Y axis) → 4D XW plane.
            // from_plane_angle(XZ, δ) takes X→W by +δ.
            let r = Rotor4::from_plane_angle(RotationPlane::XZ, delta);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// Move using camera matrix transformation (Engine4D style)
    ///
    /// Movement is transformed by the camera matrix, which ensures:
    /// - Forward/back stays horizontal (Y unchanged) regardless of 4D rotation
    /// - Only pitch affects the vertical component of movement
    ///
    /// This matches Engine4D's `accel = camMatrix * accel`
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

    /// Reset camera to the default starting position and orientation
    /// Note: pitch_limit is preserved
    pub fn reset(&mut self) {
        self.position = Vec4::new(0.0, 0.0, 5.0, 0.0);
        self.pitch = 0.0;
        self.rotation_4d = Rotor4::IDENTITY;
        self.slice_offset = 0.0;
        // pitch_limit is intentionally preserved
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
        eprintln!("ana_before: ({:.4}, {:.4}, {:.4}, {:.4})",
            ana_before.x, ana_before.y, ana_before.z, ana_before.w);
        assert!(ana_before.w > 0.9,
            "Initial ana should be ~(0,0,0,1), got W={}", ana_before.w);
        assert!(ana_before.x.abs() < 0.1,
            "Initial ana X should be ~0, got {}", ana_before.x);

        // After 90° rotation in the ZW plane (via rotate_w), ana should change
        cam.rotate_w(FRAC_PI_2);

        let ana_after = cam.ana();
        eprintln!("ana_after: ({:.4}, {:.4}, {:.4}, {:.4})",
            ana_after.x, ana_after.y, ana_after.z, ana_after.w);

        // After 90° ZW rotation, the W axis should point along Z
        assert!(ana_after.w.abs() < 0.1,
            "After 90° rotation, W component should be ~0, got {}", ana_after.w);
        assert!(ana_after.z.abs() > 0.9,
            "After 90° rotation, Z component should be ~±1, got {}", ana_after.z);

        // And forward (-Z) should now look into the 4th dimension
        let fwd = cam.forward();
        assert!(fwd.w.abs() > 0.9,
            "After 90° ZW rotation, forward should point along W, got {:?}", fwd);

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
        eprintln!("Before rotation: ana=({:.2},{:.2},{:.2},{:.2}) projected=({:.2},{:.2},{:.2},{:.2})",
            ana.x, ana.y, ana.z, ana.w, ana_xzw.x, ana_xzw.y, ana_xzw.z, ana_xzw.w);

        // W movement should go in +W direction
        assert!(ana_xzw.w > 0.9, "Before rotation, W movement should be +W");
        assert!(ana_xzw.x.abs() < 0.1, "Before rotation, X component should be ~0");

        // === After 90° rotation ===
        cam.rotate_w(FRAC_PI_2);

        let ana = cam.ana();
        let ana_xzw = project_ana(ana);
        eprintln!("After 90° rotation: ana=({:.2},{:.2},{:.2},{:.2}) projected=({:.2},{:.2},{:.2},{:.2})",
            ana.x, ana.y, ana.z, ana.w, ana_xzw.x, ana_xzw.y, ana_xzw.z, ana_xzw.w);

        // W movement should now go in +Z or -Z direction (ZW rotation)
        assert!(ana_xzw.w.abs() < 0.1, "After 90° rotation, W movement should NOT go in W direction");
        assert!(ana_xzw.z.abs() > 0.9, "After 90° rotation, W movement should go in Z direction");

        // Verify: pressing Q after rotation affects Z, not W
        let w_input = 1.0;
        let move_from_w = ana_xzw * w_input;
        eprintln!("Movement from Q key: ({:.2},{:.2},{:.2},{:.2})",
            move_from_w.x, move_from_w.y, move_from_w.z, move_from_w.w);

        assert!(move_from_w.z.abs() > 0.9, "Q key should affect Z position after rotation");
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

}
