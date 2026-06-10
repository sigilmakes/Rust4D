//! SpriteBatch for efficient sprite rendering
//!
//! Collects sprites during a frame and provides sorted output for rendering
//! with proper transparency handling.

use super::types::{Sprite, SpriteSheet};
use rust4d_math::Vec4;
use std::collections::HashMap;

/// Batch renderer for sprites
///
/// Collects sprites during a frame, manages sprite sheets, and provides
/// sorted output for rendering with correct transparency ordering.
pub struct SpriteBatch {
    /// Sprites queued for rendering this frame
    sprites: Vec<Sprite>,
    /// Registered sprite sheets
    sheets: HashMap<String, SpriteSheet>,
}

impl SpriteBatch {
    /// Create a new empty sprite batch
    pub fn new() -> Self {
        Self {
            sprites: Vec::new(),
            sheets: HashMap::new(),
        }
    }

    /// Create a sprite batch with pre-allocated capacity
    pub fn with_capacity(sprite_capacity: usize) -> Self {
        Self {
            sprites: Vec::with_capacity(sprite_capacity),
            sheets: HashMap::new(),
        }
    }

    /// Register a sprite sheet for use with sprites
    pub fn register_sheet(&mut self, sheet: SpriteSheet) {
        self.sheets.insert(sheet.name.clone(), sheet);
    }

    /// Get a registered sprite sheet by name
    pub fn get_sheet(&self, name: &str) -> Option<&SpriteSheet> {
        self.sheets.get(name)
    }

    /// Check if a sprite sheet is registered
    pub fn has_sheet(&self, name: &str) -> bool {
        self.sheets.contains_key(name)
    }

    /// Add a sprite to render this frame
    pub fn add(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Add a sprite with automatic W-fade alpha applied
    ///
    /// This calculates the W-fade and multiplies it with the sprite's
    /// existing alpha value before adding to the batch.
    pub fn add_4d(&mut self, position: Vec4, current_w_slice: f32, mut sprite: Sprite) {
        sprite.position = position;
        let fade_alpha = Self::calculate_w_fade(position.w, current_w_slice, sprite.w_fade_range);
        sprite.color_tint[3] *= fade_alpha;
        self.sprites.push(sprite);
    }

    /// Clear all sprites for the next frame
    ///
    /// Does not clear registered sprite sheets.
    pub fn clear(&mut self) {
        self.sprites.clear();
    }

    /// Get the number of sprites currently in the batch
    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    /// Get sprites sorted for rendering (back to front)
    ///
    /// Sorts sprites by 3D distance from the camera (farthest first)
    /// so that transparency blending works correctly.
    pub fn get_sorted(&self, camera_pos: Vec4) -> Vec<&Sprite> {
        let mut sorted: Vec<&Sprite> = self.sprites.iter().collect();
        sorted.sort_by(|a, b| {
            let dist_a = a.distance_3d(camera_pos);
            let dist_b = b.distance_3d(camera_pos);
            // Sort by distance descending (far sprites first)
            dist_b
                .partial_cmp(&dist_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    /// Get sprites sorted and filtered by visibility
    ///
    /// Returns sprites that have non-zero alpha after W-fade,
    /// sorted back to front.
    pub fn get_visible_sorted(&self, camera_pos: Vec4, current_w: f32) -> Vec<(&Sprite, f32)> {
        let mut visible: Vec<(&Sprite, f32)> = self
            .sprites
            .iter()
            .filter_map(|sprite| {
                let w_fade =
                    Self::calculate_w_fade(sprite.position.w, current_w, sprite.w_fade_range);
                let final_alpha = sprite.color_tint[3] * w_fade;
                if final_alpha > 0.001 {
                    Some((sprite, final_alpha))
                } else {
                    None
                }
            })
            .collect();

        visible.sort_by(|a, b| {
            let dist_a = a.0.distance_3d(camera_pos);
            let dist_b = b.0.distance_3d(camera_pos);
            dist_b
                .partial_cmp(&dist_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        visible
    }

    /// Calculate alpha value for W-fade effect
    ///
    /// Returns 1.0 when sprite is at the same W as camera,
    /// linearly decreasing to 0.0 at the fade_range distance.
    ///
    /// # Arguments
    /// * `sprite_w` - The W coordinate of the sprite
    /// * `camera_w` - The W coordinate of the camera/slice
    /// * `fade_range` - The W distance at which the sprite becomes fully transparent
    pub fn calculate_w_fade(sprite_w: f32, camera_w: f32, fade_range: f32) -> f32 {
        if fade_range <= 0.0 {
            return 1.0; // No fade, always fully opaque
        }

        let w_distance = (sprite_w - camera_w).abs();
        if w_distance >= fade_range {
            0.0 // Fully transparent
        } else {
            1.0 - (w_distance / fade_range) // Linear fade
        }
    }

    /// Iterate over all sprites in insertion order
    pub fn iter(&self) -> impl Iterator<Item = &Sprite> {
        self.sprites.iter()
    }

    /// Get all sprites as a slice (unsorted)
    pub fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }

    /// Get all registered sheets
    pub fn sheets(&self) -> &HashMap<String, SpriteSheet> {
        &self.sheets
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_w_fade_same_w() {
        // Sprite at same W as camera = fully opaque
        let alpha = SpriteBatch::calculate_w_fade(5.0, 5.0, 2.0);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_w_fade_beyond_range() {
        // Sprite at W distance >= fade_range = fully transparent
        let alpha = SpriteBatch::calculate_w_fade(5.0, 0.0, 2.0);
        assert_eq!(alpha, 0.0);

        let alpha = SpriteBatch::calculate_w_fade(-3.0, 0.0, 2.0);
        assert_eq!(alpha, 0.0);
    }

    #[test]
    fn test_w_fade_linear_interpolation() {
        // Sprite at half fade distance = 50% alpha
        let alpha = SpriteBatch::calculate_w_fade(1.0, 0.0, 2.0);
        assert!((alpha - 0.5).abs() < 0.0001);

        // Sprite at quarter fade distance = 75% alpha
        let alpha = SpriteBatch::calculate_w_fade(0.5, 0.0, 2.0);
        assert!((alpha - 0.75).abs() < 0.0001);

        // Sprite at three-quarter fade distance = 25% alpha
        let alpha = SpriteBatch::calculate_w_fade(1.5, 0.0, 2.0);
        assert!((alpha - 0.25).abs() < 0.0001);
    }

    #[test]
    fn test_w_fade_negative_direction() {
        // W-fade should work in both directions
        let alpha_pos = SpriteBatch::calculate_w_fade(1.0, 0.0, 2.0);
        let alpha_neg = SpriteBatch::calculate_w_fade(-1.0, 0.0, 2.0);
        assert!((alpha_pos - alpha_neg).abs() < 0.0001);
    }

    #[test]
    fn test_w_fade_zero_range() {
        // Zero fade range = always opaque
        let alpha = SpriteBatch::calculate_w_fade(100.0, 0.0, 0.0);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_sprite_batch_add_and_clear() {
        let mut batch = SpriteBatch::new();

        batch.add(Sprite::new(Vec4::ZERO, "sheet1"));
        batch.add(Sprite::new(Vec4::new(1.0, 0.0, 0.0, 0.0), "sheet1"));
        assert_eq!(batch.len(), 2);

        batch.clear();
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_sprite_batch_sorting_far_first() {
        let mut batch = SpriteBatch::new();

        // Add sprites at different distances
        batch.add(Sprite::new(Vec4::new(1.0, 0.0, 0.0, 0.0), "sheet")); // Distance 1
        batch.add(Sprite::new(Vec4::new(5.0, 0.0, 0.0, 0.0), "sheet")); // Distance 5
        batch.add(Sprite::new(Vec4::new(3.0, 0.0, 0.0, 0.0), "sheet")); // Distance 3

        let camera = Vec4::ZERO;
        let sorted = batch.get_sorted(camera);

        // Far sprites should come first
        assert_eq!(sorted[0].position.x, 5.0);
        assert_eq!(sorted[1].position.x, 3.0);
        assert_eq!(sorted[2].position.x, 1.0);
    }

    #[test]
    fn test_sprite_batch_sorting_ignores_w() {
        let mut batch = SpriteBatch::new();

        // Same XYZ but different W - should have same sort order based on XYZ
        batch.add(Sprite::new(Vec4::new(1.0, 0.0, 0.0, 100.0), "sheet"));
        batch.add(Sprite::new(Vec4::new(5.0, 0.0, 0.0, 0.0), "sheet"));

        let camera = Vec4::ZERO;
        let sorted = batch.get_sorted(camera);

        // W=100 sprite is at XYZ distance 1, should be second
        // W=0 sprite is at XYZ distance 5, should be first
        assert_eq!(sorted[0].position.x, 5.0);
        assert_eq!(sorted[1].position.x, 1.0);
    }

    #[test]
    fn test_sheet_registration() {
        let mut batch = SpriteBatch::new();

        let sheet = SpriteSheet::new("enemies", 32, 32, 4, 4);
        batch.register_sheet(sheet);

        assert!(batch.has_sheet("enemies"));
        assert!(!batch.has_sheet("nonexistent"));

        let retrieved = batch.get_sheet("enemies").unwrap();
        assert_eq!(retrieved.name, "enemies");
        assert_eq!(retrieved.frame_width, 32);
    }

    #[test]
    fn test_add_4d_applies_w_fade() {
        let mut batch = SpriteBatch::new();

        let sprite = Sprite::new(Vec4::ZERO, "sheet")
            .with_w_fade_range(2.0)
            .with_color(1.0, 1.0, 1.0, 1.0);

        // Position at W=1.0, camera at W=0.0, fade_range=2.0
        // Expected alpha: 1.0 - (1.0 / 2.0) = 0.5
        let position = Vec4::new(0.0, 0.0, 0.0, 1.0);
        batch.add_4d(position, 0.0, sprite);

        assert_eq!(batch.sprites[0].position.w, 1.0);
        assert!((batch.sprites[0].color_tint[3] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_get_visible_sorted_filters_transparent() {
        let mut batch = SpriteBatch::new();

        // Visible sprite (W=0 at camera W=0)
        batch.add(Sprite::new(Vec4::new(1.0, 0.0, 0.0, 0.0), "sheet").with_w_fade_range(2.0));

        // Invisible sprite (W=5 at camera W=0, fade_range=2)
        batch.add(Sprite::new(Vec4::new(2.0, 0.0, 0.0, 5.0), "sheet").with_w_fade_range(2.0));

        let camera = Vec4::ZERO;
        let visible = batch.get_visible_sorted(camera, 0.0);

        // Only one sprite should be visible
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].0.position.x, 1.0);
        assert!((visible[0].1 - 1.0).abs() < 0.0001); // Full alpha
    }

    #[test]
    fn test_with_capacity() {
        let batch = SpriteBatch::with_capacity(100);
        assert!(batch.is_empty());
        // Capacity is an implementation detail, just verify it doesn't crash
    }
}
