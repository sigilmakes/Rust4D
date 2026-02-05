//! Sprite and SpriteSheet types for 4D billboard rendering
//!
//! Sprites are 2D billboards positioned in 4D space. They always face the camera
//! in XYZ space (classic billboard behavior) while being positioned in 4D. Their
//! visibility fades based on W distance from the camera's current W slice.

use rust4d_math::Vec4;

/// A sprite sheet (atlas of frames)
///
/// Contains metadata about a texture atlas that can be used for animations
/// or tile-based sprite sets. The actual texture data is managed separately
/// by the rendering backend.
#[derive(Clone, Debug)]
pub struct SpriteSheet {
    /// Unique identifier for this sheet
    pub name: String,
    /// Width of each frame in pixels
    pub frame_width: u32,
    /// Height of each frame in pixels
    pub frame_height: u32,
    /// Number of columns in the atlas
    pub columns: u32,
    /// Number of rows in the atlas
    pub rows: u32,
    // Note: texture handle would be added when GPU resources are integrated
}

impl SpriteSheet {
    /// Create a new sprite sheet with the given parameters
    pub fn new(name: impl Into<String>, frame_width: u32, frame_height: u32, columns: u32, rows: u32) -> Self {
        Self {
            name: name.into(),
            frame_width,
            frame_height,
            columns,
            rows,
        }
    }

    /// Total number of frames in this sheet
    pub fn frame_count(&self) -> u32 {
        self.columns * self.rows
    }

    /// Get UV coordinates for a specific frame
    ///
    /// Returns (u_min, v_min, u_max, v_max) normalized to [0, 1]
    pub fn frame_uvs(&self, frame: u32) -> [f32; 4] {
        let frame = frame % self.frame_count();
        let col = frame % self.columns;
        let row = frame / self.columns;

        let u_step = 1.0 / self.columns as f32;
        let v_step = 1.0 / self.rows as f32;

        let u_min = col as f32 * u_step;
        let v_min = row as f32 * v_step;
        let u_max = u_min + u_step;
        let v_max = v_min + v_step;

        [u_min, v_min, u_max, v_max]
    }
}

/// A single sprite instance to render
///
/// Sprites are billboards that always face the camera in XYZ space. They are
/// positioned in 4D and fade based on their W distance from the current camera slice.
#[derive(Clone, Debug)]
pub struct Sprite {
    /// 4D position (center of the sprite)
    pub position: Vec4,
    /// Which sprite sheet to use
    pub sheet_name: String,
    /// Frame index within the sprite sheet
    pub frame: u32,
    /// Size in world units [width, height]
    pub size: [f32; 2],
    /// Distance in W before fade starts (0.0 = no fade, fade linearly to transparent at this distance)
    pub w_fade_range: f32,
    /// RGBA color tint/modulate
    pub color_tint: [f32; 4],
}

impl Sprite {
    /// Create a new sprite with default settings
    pub fn new(position: Vec4, sheet_name: impl Into<String>) -> Self {
        Self {
            position,
            sheet_name: sheet_name.into(),
            frame: 0,
            size: [1.0, 1.0],
            w_fade_range: 2.0, // Default fade range of 2 world units
            color_tint: [1.0, 1.0, 1.0, 1.0], // White (no tint)
        }
    }

    /// Set the frame index
    pub fn with_frame(mut self, frame: u32) -> Self {
        self.frame = frame;
        self
    }

    /// Set the size in world units
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }

    /// Set the W fade range
    pub fn with_w_fade_range(mut self, range: f32) -> Self {
        self.w_fade_range = range;
        self
    }

    /// Set the color tint
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color_tint = [r, g, b, a];
        self
    }

    /// Get the 3D distance from a camera position (for depth sorting)
    ///
    /// Uses only XYZ components since sprites are billboards
    pub fn distance_3d(&self, camera_pos: Vec4) -> f32 {
        let dx = self.position.x - camera_pos.x;
        let dy = self.position.y - camera_pos.y;
        let dz = self.position.z - camera_pos.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// W-fade configuration for sprites
///
/// Controls how sprites fade based on their W distance from the camera's
/// current viewing slice.
#[derive(Clone, Debug)]
pub struct WFadeConfig {
    /// Current camera W slice position
    pub current_w: f32,
    /// W distance where fade begins (closer sprites are fully opaque)
    pub fade_start: f32,
    /// W distance where sprites become fully transparent
    pub fade_end: f32,
}

impl WFadeConfig {
    /// Create a new W fade configuration
    pub fn new(current_w: f32, fade_start: f32, fade_end: f32) -> Self {
        Self {
            current_w,
            fade_start,
            fade_end,
        }
    }

    /// Calculate the alpha multiplier for a sprite at the given W position
    pub fn calculate_alpha(&self, sprite_w: f32) -> f32 {
        let w_distance = (sprite_w - self.current_w).abs();

        if w_distance <= self.fade_start {
            1.0 // Fully opaque
        } else if w_distance >= self.fade_end {
            0.0 // Fully transparent
        } else {
            // Linear interpolation between fade_start and fade_end
            let fade_range = self.fade_end - self.fade_start;
            1.0 - (w_distance - self.fade_start) / fade_range
        }
    }
}

impl Default for WFadeConfig {
    fn default() -> Self {
        Self {
            current_w: 0.0,
            fade_start: 0.0,
            fade_end: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_sheet_frame_count() {
        let sheet = SpriteSheet::new("test", 32, 32, 4, 4);
        assert_eq!(sheet.frame_count(), 16);
    }

    #[test]
    fn test_sprite_sheet_uvs() {
        let sheet = SpriteSheet::new("test", 32, 32, 4, 4);

        // First frame (top-left)
        let uvs = sheet.frame_uvs(0);
        assert_eq!(uvs[0], 0.0);  // u_min
        assert_eq!(uvs[1], 0.0);  // v_min
        assert_eq!(uvs[2], 0.25); // u_max
        assert_eq!(uvs[3], 0.25); // v_max

        // Second frame (second column, first row)
        let uvs = sheet.frame_uvs(1);
        assert_eq!(uvs[0], 0.25);
        assert_eq!(uvs[1], 0.0);
        assert_eq!(uvs[2], 0.5);
        assert_eq!(uvs[3], 0.25);

        // Fifth frame (first column, second row)
        let uvs = sheet.frame_uvs(4);
        assert_eq!(uvs[0], 0.0);
        assert_eq!(uvs[1], 0.25);
        assert_eq!(uvs[2], 0.25);
        assert_eq!(uvs[3], 0.5);
    }

    #[test]
    fn test_sprite_sheet_uvs_wrapping() {
        let sheet = SpriteSheet::new("test", 32, 32, 4, 4);

        // Frame 16 should wrap to frame 0
        let uvs_0 = sheet.frame_uvs(0);
        let uvs_16 = sheet.frame_uvs(16);
        assert_eq!(uvs_0, uvs_16);
    }

    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite::new(Vec4::new(1.0, 2.0, 3.0, 4.0), "test_sheet");
        assert_eq!(sprite.position.x, 1.0);
        assert_eq!(sprite.position.y, 2.0);
        assert_eq!(sprite.position.z, 3.0);
        assert_eq!(sprite.position.w, 4.0);
        assert_eq!(sprite.sheet_name, "test_sheet");
        assert_eq!(sprite.frame, 0);
    }

    #[test]
    fn test_sprite_builder_pattern() {
        let sprite = Sprite::new(Vec4::ZERO, "sheet")
            .with_frame(5)
            .with_size(2.0, 3.0)
            .with_w_fade_range(5.0)
            .with_color(1.0, 0.0, 0.0, 0.5);

        assert_eq!(sprite.frame, 5);
        assert_eq!(sprite.size, [2.0, 3.0]);
        assert_eq!(sprite.w_fade_range, 5.0);
        assert_eq!(sprite.color_tint, [1.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn test_sprite_distance_3d() {
        let sprite = Sprite::new(Vec4::new(3.0, 0.0, 4.0, 100.0), "sheet");
        let camera = Vec4::new(0.0, 0.0, 0.0, 0.0);

        // 3-4-5 triangle in XZ, W is ignored for 3D distance
        assert!((sprite.distance_3d(camera) - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_w_fade_config_same_w() {
        let config = WFadeConfig::new(5.0, 0.0, 2.0);

        // Sprite at same W as camera = fully opaque
        let alpha = config.calculate_alpha(5.0);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_w_fade_config_beyond_range() {
        let config = WFadeConfig::new(0.0, 0.0, 2.0);

        // Sprite at W distance >= fade_end = fully transparent
        let alpha = config.calculate_alpha(3.0);
        assert_eq!(alpha, 0.0);

        let alpha = config.calculate_alpha(-2.5);
        assert_eq!(alpha, 0.0);
    }

    #[test]
    fn test_w_fade_config_linear_interpolation() {
        let config = WFadeConfig::new(0.0, 0.0, 2.0);

        // Sprite at W=1.0 (half of fade range) = 50% alpha
        let alpha = config.calculate_alpha(1.0);
        assert!((alpha - 0.5).abs() < 0.0001);

        // Sprite at W=0.5 = 75% alpha
        let alpha = config.calculate_alpha(0.5);
        assert!((alpha - 0.75).abs() < 0.0001);

        // Sprite at W=1.5 = 25% alpha
        let alpha = config.calculate_alpha(1.5);
        assert!((alpha - 0.25).abs() < 0.0001);
    }

    #[test]
    fn test_w_fade_config_with_fade_start() {
        // Fade only starts at W distance of 1.0, ends at 3.0
        let config = WFadeConfig::new(0.0, 1.0, 3.0);

        // Within fade_start = fully opaque
        let alpha = config.calculate_alpha(0.5);
        assert_eq!(alpha, 1.0);

        // At fade_start = fully opaque
        let alpha = config.calculate_alpha(1.0);
        assert_eq!(alpha, 1.0);

        // Halfway through fade range (1.0 to 3.0, midpoint at 2.0)
        let alpha = config.calculate_alpha(2.0);
        assert!((alpha - 0.5).abs() < 0.0001);

        // At fade_end = fully transparent
        let alpha = config.calculate_alpha(3.0);
        assert_eq!(alpha, 0.0);
    }
}
