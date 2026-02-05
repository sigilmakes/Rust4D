//! 4D spatial audio calculations

use rust4d_math::Vec4;

/// Configuration for spatial audio playback
#[derive(Debug, Clone, Copy)]
pub struct SpatialConfig {
    /// Position of the sound source in 4D space
    pub position: Vec4,
    /// Distance at which volume is at full (1.0)
    pub min_distance: f32,
    /// Distance at which volume reaches zero
    pub max_distance: f32,
}

impl SpatialConfig {
    /// Default min distance for spatial sounds
    pub const DEFAULT_MIN_DISTANCE: f32 = 1.0;
    /// Default max distance for spatial sounds
    pub const DEFAULT_MAX_DISTANCE: f32 = 50.0;

    /// Create a new spatial config with default distance parameters
    pub fn new(position: Vec4) -> Self {
        Self {
            position,
            min_distance: Self::DEFAULT_MIN_DISTANCE,
            max_distance: Self::DEFAULT_MAX_DISTANCE,
        }
    }

    /// Set the minimum distance (full volume)
    pub fn with_min_distance(mut self, distance: f32) -> Self {
        self.min_distance = distance.max(0.0);
        self
    }

    /// Set the maximum distance (zero volume)
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance.max(self.min_distance);
        self
    }
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self::new(Vec4::ZERO)
    }
}

/// Calculate volume attenuation based on 4D distance
///
/// Uses linear falloff between min_distance and max_distance.
/// Returns a value between 0.0 (silent) and 1.0 (full volume).
pub fn calculate_attenuation(listener: Vec4, config: &SpatialConfig) -> f32 {
    let distance = listener.distance(config.position);

    if distance <= config.min_distance {
        1.0
    } else if distance >= config.max_distance {
        0.0
    } else {
        // Linear falloff
        let range = config.max_distance - config.min_distance;
        let relative_distance = distance - config.min_distance;
        1.0 - (relative_distance / range)
    }
}

/// Calculate stereo panning based on the XZ projection of the 4D direction
///
/// Returns a value between 0.0 (left) and 1.0 (right), with 0.5 being center.
pub fn calculate_panning(listener: Vec4, config: &SpatialConfig) -> f32 {
    let direction = config.position - listener;

    // Project to XZ plane (horizontal plane in typical game coordinates)
    let xz_length_sq = direction.x * direction.x + direction.z * direction.z;

    if xz_length_sq < 0.0001 {
        // Source is directly above/below or at same position - center panning
        return 0.5;
    }

    // Normalize the XZ direction
    let xz_length = xz_length_sq.sqrt();
    let normalized_x = direction.x / xz_length;

    // Convert from [-1, 1] to [0, 1]
    // Positive X is to the right
    (normalized_x + 1.0) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attenuation_at_source() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::ZERO);
        let attenuation = calculate_attenuation(listener, &config);
        assert!((attenuation - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_attenuation_within_min_distance() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(0.5, 0.0, 0.0, 0.0))
            .with_min_distance(1.0);
        let attenuation = calculate_attenuation(listener, &config);
        assert!((attenuation - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_attenuation_at_max_distance() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(50.0, 0.0, 0.0, 0.0))
            .with_max_distance(50.0);
        let attenuation = calculate_attenuation(listener, &config);
        assert!(attenuation < 0.0001);
    }

    #[test]
    fn test_attenuation_beyond_max_distance() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(100.0, 0.0, 0.0, 0.0))
            .with_max_distance(50.0);
        let attenuation = calculate_attenuation(listener, &config);
        assert_eq!(attenuation, 0.0);
    }

    #[test]
    fn test_attenuation_midpoint() {
        let listener = Vec4::ZERO;
        // Distance 25.5 is midpoint between 1 and 50
        let config = SpatialConfig::new(Vec4::new(25.5, 0.0, 0.0, 0.0))
            .with_min_distance(1.0)
            .with_max_distance(50.0);
        let attenuation = calculate_attenuation(listener, &config);
        assert!((attenuation - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_attenuation_4d_distance() {
        // Test that 4D distance is used (not just 3D)
        let listener = Vec4::ZERO;
        // A point that's 2 units away in 4D (1^2 + 1^2 + 1^2 + 1^2 = 4, sqrt(4) = 2)
        let config = SpatialConfig::new(Vec4::new(1.0, 1.0, 1.0, 1.0))
            .with_min_distance(1.0)
            .with_max_distance(3.0);
        let attenuation = calculate_attenuation(listener, &config);
        // Distance is 2, range is 2 (3-1), so attenuation should be 0.5
        assert!((attenuation - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_panning_center() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::ZERO);
        let panning = calculate_panning(listener, &config);
        assert!((panning - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_panning_right() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let panning = calculate_panning(listener, &config);
        assert!((panning - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_panning_left() {
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(-10.0, 0.0, 0.0, 0.0));
        let panning = calculate_panning(listener, &config);
        assert!(panning < 0.01);
    }

    #[test]
    fn test_panning_front() {
        // Sound directly in front (positive Z) should be centered
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(0.0, 0.0, 10.0, 0.0));
        let panning = calculate_panning(listener, &config);
        assert!((panning - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_panning_diagonal() {
        // Sound at 45 degrees to the right
        let listener = Vec4::ZERO;
        let config = SpatialConfig::new(Vec4::new(10.0, 0.0, 10.0, 0.0));
        let panning = calculate_panning(listener, &config);
        // Should be between 0.5 and 1.0
        assert!(panning > 0.5 && panning < 1.0);
        // 45 degrees -> sin(45) = 0.707, so panning = (0.707 + 1) / 2 = 0.854
        assert!((panning - 0.854).abs() < 0.01);
    }

    #[test]
    fn test_panning_ignores_y_and_w() {
        // Y and W components shouldn't affect XZ panning
        let listener = Vec4::ZERO;
        let config1 = SpatialConfig::new(Vec4::new(5.0, 0.0, 5.0, 0.0));
        let config2 = SpatialConfig::new(Vec4::new(5.0, 100.0, 5.0, 0.0));
        let config3 = SpatialConfig::new(Vec4::new(5.0, 0.0, 5.0, 100.0));

        let panning1 = calculate_panning(listener, &config1);
        let panning2 = calculate_panning(listener, &config2);
        let panning3 = calculate_panning(listener, &config3);

        assert!((panning1 - panning2).abs() < 0.01);
        assert!((panning1 - panning3).abs() < 0.01);
    }
}
