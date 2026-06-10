//! Particle types and blend modes

use rust4d_math::{Interpolatable, Vec4};

/// Individual particle in the particle system
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    /// 4D position
    pub position: Vec4,
    /// 4D velocity
    pub velocity: Vec4,
    /// Remaining lifetime in seconds
    pub lifetime: f32,
    /// Initial lifetime (used for interpolation)
    pub max_lifetime: f32,
    /// Current size
    pub size: f32,
    /// RGBA color
    pub color: [f32; 4],
    /// Initial size (for interpolation)
    initial_size: f32,
    /// End size (for interpolation)
    end_size: f32,
    /// Initial color (for interpolation)
    initial_color: [f32; 4],
    /// End color (for interpolation)
    end_color: [f32; 4],
    /// Gravity applied to Y velocity
    gravity: f32,
    /// Velocity damping factor
    drag: f32,
    /// Blend mode for this particle
    pub blend_mode: BlendMode,
}

impl Particle {
    /// Create a new particle with the given parameters
    #[allow(clippy::too_many_arguments)] // construction site is the emitter; a builder adds no clarity
    pub fn new(
        position: Vec4,
        velocity: Vec4,
        lifetime: f32,
        initial_size: f32,
        end_size: f32,
        initial_color: [f32; 4],
        end_color: [f32; 4],
        gravity: f32,
        drag: f32,
        blend_mode: BlendMode,
    ) -> Self {
        Self {
            position,
            velocity,
            lifetime,
            max_lifetime: lifetime,
            size: initial_size,
            color: initial_color,
            initial_size,
            end_size,
            initial_color,
            end_color,
            gravity,
            drag,
            blend_mode,
        }
    }

    /// Update the particle physics and return true if still alive
    pub fn update(&mut self, dt: f32) -> bool {
        // Decrease lifetime
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            return false;
        }

        // Apply gravity to Y velocity
        self.velocity.y += self.gravity * dt;

        // Apply drag
        let drag_factor = (1.0 - self.drag * dt).max(0.0);
        self.velocity *= drag_factor;

        // Apply velocity to position
        self.position += self.velocity * dt;

        // Interpolate size and color based on lifetime ratio
        let life_ratio = self.lifetime / self.max_lifetime;
        self.size = f32::lerp(&self.end_size, &self.initial_size, life_ratio);
        self.color = lerp_color(&self.end_color, &self.initial_color, life_ratio);

        true
    }

    /// Get the lifetime ratio (0.0 = dead, 1.0 = just spawned)
    pub fn lifetime_ratio(&self) -> f32 {
        if self.max_lifetime > 0.0 {
            self.lifetime / self.max_lifetime
        } else {
            0.0
        }
    }
}

/// Blend mode for particle rendering
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard alpha blending
    #[default]
    Alpha,
    /// Additive blending (good for fire, sparks)
    Additive,
}

/// Minimum allowed lifetime for particles to avoid CPU waste on instantly-dying particles
pub const MIN_PARTICLE_LIFETIME: f32 = 0.001;

/// Configuration for particle bursts
#[derive(Clone, Debug)]
pub struct BurstConfig {
    /// Number of particles to spawn
    pub count: u32,
    /// Lifetime in seconds.
    ///
    /// **Warning**: Setting this to zero or negative values will waste CPU cycles on
    /// particles that die immediately. Use [`BurstConfig::effective_lifetime()`] to get
    /// the clamped value, or [`BurstConfig::with_lifetime()`] to set with automatic clamping.
    pub lifetime: f32,
    /// Starting color (RGBA)
    pub initial_color: [f32; 4],
    /// Ending color (RGBA)
    pub end_color: [f32; 4],
    /// Starting size
    pub initial_size: f32,
    /// Ending size
    pub end_size: f32,
    /// Initial speed
    pub speed: f32,
    /// Cone angle in radians for random direction spread
    pub spread: f32,
    /// Gravity applied to Y velocity (negative = down)
    pub gravity: f32,
    /// Velocity damping per second (0.0 = none, 1.0 = full stop)
    pub drag: f32,
    /// Blend mode for particles
    pub blend_mode: BlendMode,
}

impl BurstConfig {
    /// Get the effective particle lifetime, clamped to the minimum allowed value.
    ///
    /// This ensures particles don't die instantly, which would waste CPU cycles.
    /// Returns at least `MIN_PARTICLE_LIFETIME` (0.01s).
    #[inline]
    pub fn effective_lifetime(&self) -> f32 {
        self.lifetime.max(MIN_PARTICLE_LIFETIME)
    }

    /// Create a new BurstConfig with the given lifetime (clamped to minimum)
    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.lifetime = lifetime.max(MIN_PARTICLE_LIFETIME);
        self
    }
}

impl Default for BurstConfig {
    fn default() -> Self {
        Self {
            count: 10,
            lifetime: 1.0,
            initial_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 0.0],
            initial_size: 0.1,
            end_size: 0.0,
            speed: 5.0,
            spread: std::f32::consts::PI,
            gravity: -9.8,
            drag: 0.1,
            blend_mode: BlendMode::Alpha,
        }
    }
}

/// Configuration for continuous particle emitters
#[derive(Clone, Debug)]
pub struct EmitterConfig {
    /// Particles emitted per second
    pub rate: f32,
    /// Configuration for each particle
    pub burst: BurstConfig,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            rate: 10.0,
            burst: BurstConfig {
                count: 1,
                ..BurstConfig::default()
            },
        }
    }
}

/// Linear interpolation between two colors
///
/// Uses `Interpolatable::lerp` from rust4d_math for each component.
#[inline]
fn lerp_color(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        f32::lerp(&a[0], &b[0], t),
        f32::lerp(&a[1], &b[1], t),
        f32::lerp(&a[2], &b[2], t),
        f32::lerp(&a[3], &b[3], t),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let particle = Particle::new(
            Vec4::ZERO,
            Vec4::Y,
            1.0,
            1.0,
            0.0,
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.0],
            -9.8,
            0.1,
            BlendMode::Alpha,
        );
        assert_eq!(particle.lifetime, 1.0);
        assert_eq!(particle.max_lifetime, 1.0);
        assert_eq!(particle.size, 1.0);
    }

    #[test]
    fn test_particle_lifetime_decreases() {
        let mut particle = Particle::new(
            Vec4::ZERO,
            Vec4::ZERO,
            1.0,
            1.0,
            0.0,
            [1.0, 1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 0.0],
            0.0,
            0.0,
            BlendMode::Alpha,
        );
        assert!(particle.update(0.5));
        assert!((particle.lifetime - 0.5).abs() < 0.001);
        // After 0.5s more (total 1.0s = lifetime), particle dies on this update
        assert!(!particle.update(0.5));
    }

    #[test]
    fn test_particle_removes_dead() {
        let mut particle = Particle::new(
            Vec4::ZERO,
            Vec4::ZERO,
            0.5,
            1.0,
            0.0,
            [1.0, 1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 0.0],
            0.0,
            0.0,
            BlendMode::Alpha,
        );
        assert!(particle.update(0.4)); // Still alive
        assert!(!particle.update(0.2)); // Dead
    }

    #[test]
    fn test_gravity_affects_velocity() {
        let mut particle = Particle::new(
            Vec4::ZERO,
            Vec4::ZERO,
            10.0,
            1.0,
            1.0,
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            -10.0, // gravity
            0.0,   // no drag
            BlendMode::Alpha,
        );
        particle.update(1.0);
        // After 1 second with -10 gravity, velocity.y should be -10
        assert!((particle.velocity.y - (-10.0)).abs() < 0.1);
    }

    #[test]
    fn test_color_size_interpolation() {
        let mut particle = Particle::new(
            Vec4::ZERO,
            Vec4::ZERO,
            1.0,
            1.0,                  // initial size
            0.0,                  // end size
            [1.0, 0.0, 0.0, 1.0], // initial color (red)
            [0.0, 1.0, 0.0, 0.0], // end color (green, transparent)
            0.0,
            0.0,
            BlendMode::Alpha,
        );

        // Update to 50% lifetime
        particle.update(0.5);

        // Size should be interpolated (at 50% life, should be ~0.5)
        assert!((particle.size - 0.5).abs() < 0.1);

        // Color should be interpolated
        assert!((particle.color[0] - 0.5).abs() < 0.1); // red fading
        assert!((particle.color[1] - 0.5).abs() < 0.1); // green increasing
    }

    #[test]
    fn test_blend_mode_default() {
        assert_eq!(BlendMode::default(), BlendMode::Alpha);
    }

    #[test]
    fn test_burst_config_default() {
        let config = BurstConfig::default();
        assert_eq!(config.count, 10);
        assert_eq!(config.lifetime, 1.0);
        assert!(config.gravity < 0.0); // Should pull down
    }

    #[test]
    fn test_burst_config_effective_lifetime() {
        // Zero lifetime should return minimum from effective_lifetime
        let config = BurstConfig {
            lifetime: 0.0,
            ..BurstConfig::default()
        };
        assert_eq!(config.effective_lifetime(), MIN_PARTICLE_LIFETIME);

        // Negative lifetime should return minimum from effective_lifetime
        let config = BurstConfig {
            lifetime: -1.0,
            ..BurstConfig::default()
        };
        assert_eq!(config.effective_lifetime(), MIN_PARTICLE_LIFETIME);

        // Valid lifetime should be preserved
        let config = BurstConfig::default().with_lifetime(2.5);
        assert_eq!(config.effective_lifetime(), 2.5);
        assert_eq!(config.lifetime, 2.5);
    }
}
