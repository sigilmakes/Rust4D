//! Particle emitter for continuous particle emission

use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rust4d_math::Vec4;
use super::types::{Particle, EmitterConfig, BurstConfig};

/// A continuous particle emitter
#[derive(Clone, Debug)]
pub struct ParticleEmitter {
    /// Configuration for this emitter
    config: EmitterConfig,
    /// Current position in 4D space
    position: Vec4,
    /// Accumulator for rate-based emission
    accumulator: f32,
    /// Whether the emitter is actively spawning particles
    active: bool,
    /// Whether the emitter should be removed
    dead: bool,
    /// Random number generator (SmallRng is fast and non-cryptographic, suitable for particles)
    rng: SmallRng,
}

impl ParticleEmitter {
    /// Create a new particle emitter at the given position
    ///
    /// Uses the position to derive a seed for the RNG, providing
    /// variation between emitters at different locations.
    pub fn new(position: Vec4, config: EmitterConfig) -> Self {
        // Use position to derive a seed for variation
        let seed = Self::derive_seed_from_position(position);
        Self::with_seed(position, config, seed)
    }

    /// Create a new particle emitter with a specific seed for deterministic effects
    ///
    /// Use this when you need reproducible particle behavior (e.g., for replays
    /// or synchronized effects across clients).
    pub fn with_seed(position: Vec4, config: EmitterConfig, seed: u64) -> Self {
        Self {
            config,
            position,
            accumulator: 0.0,
            active: true,
            dead: false,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Derive a seed from a 4D position for RNG initialization
    fn derive_seed_from_position(position: Vec4) -> u64 {
        (position.x.to_bits() as u64)
            .wrapping_mul(31)
            .wrapping_add(position.y.to_bits() as u64)
            .wrapping_mul(37)
            .wrapping_add(position.z.to_bits() as u64)
            .wrapping_mul(41)
            .wrapping_add(position.w.to_bits() as u64)
            .wrapping_add(1) // Ensure non-zero
    }

    /// Update the emitter and return newly spawned particles
    pub fn update(&mut self, dt: f32) -> Vec<Particle> {
        if !self.active {
            return Vec::new();
        }

        // Accumulate time for rate-based emission
        self.accumulator += dt * self.config.rate;

        // Spawn particles based on accumulated time
        let particles_to_spawn = self.accumulator.floor() as u32;
        self.accumulator -= particles_to_spawn as f32;

        // Spawn the particles
        let mut particles = Vec::with_capacity((particles_to_spawn * self.config.burst.count) as usize);
        for _ in 0..particles_to_spawn {
            particles.extend(self.spawn_burst_internal(&self.config.burst.clone()));
        }

        particles
    }

    /// Spawn a burst of particles (used internally and for one-shot bursts)
    fn spawn_burst_internal(&mut self, config: &BurstConfig) -> Vec<Particle> {
        let mut particles = Vec::with_capacity(config.count as usize);

        for _ in 0..config.count {
            // Generate random direction in 4D
            let direction = self.random_direction_4d(config.spread);
            let velocity = direction * config.speed;

            let particle = Particle::new(
                self.position,
                velocity,
                config.effective_lifetime(),
                config.initial_size,
                config.end_size,
                config.initial_color,
                config.end_color,
                config.gravity,
                config.drag,
                config.blend_mode,
            );

            particles.push(particle);
        }

        particles
    }

    /// Generate a random direction in 4D within a cone of the given spread angle
    fn random_direction_4d(&mut self, spread: f32) -> Vec4 {
        // Generate a random unit vector in 4D
        // Using spherical coordinates extended to 4D (hyperspherical)
        // For simplicity, we generate a random vector and normalize

        // gen_range with f32 uses the standard uniform distribution with proper precision
        let x = self.rng.gen_range(-1.0f32..1.0f32);
        let y = self.rng.gen_range(-1.0f32..1.0f32);
        let z = self.rng.gen_range(-1.0f32..1.0f32);
        let w = self.rng.gen_range(-1.0f32..1.0f32);

        let random_dir = Vec4::new(x, y, z, w).normalized();

        // If spread is PI or more, use fully random direction
        if spread >= std::f32::consts::PI {
            return random_dir;
        }

        // For smaller spread, interpolate between up direction and random
        // spread = 0 means straight up, spread = PI means any direction
        let spread_factor = spread / std::f32::consts::PI;

        // Default forward direction is +Y (up)
        let forward = Vec4::Y;

        // Lerp between forward and random based on spread
        let result = forward.lerp(random_dir, spread_factor);

        // Normalize to ensure unit vector
        if result.length_squared() > 0.0001 {
            result.normalized()
        } else {
            forward
        }
    }

    /// Set the emitter position
    pub fn set_position(&mut self, position: Vec4) {
        self.position = position;
    }

    /// Get the current position
    pub fn position(&self) -> Vec4 {
        self.position
    }

    /// Stop emitting new particles (existing particles continue)
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Start emitting particles again
    pub fn start(&mut self) {
        self.active = true;
    }

    /// Stop and mark the emitter for removal
    pub fn kill(&mut self) {
        self.active = false;
        self.dead = true;
    }

    /// Check if the emitter is actively spawning particles
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Check if the emitter should be removed
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Get the emitter configuration
    pub fn config(&self) -> &EmitterConfig {
        &self.config
    }

    /// Get mutable access to the emitter configuration
    pub fn config_mut(&mut self) -> &mut EmitterConfig {
        &mut self.config
    }
}

/// Spawn a one-time burst of particles at a position
///
/// The `seed` parameter allows for deterministic particle effects.
/// For non-deterministic behavior, you can use any value (e.g., a frame counter).
pub fn spawn_burst(position: Vec4, config: &BurstConfig, seed: u64) -> Vec<Particle> {
    let mut emitter = ParticleEmitter::with_seed(
        position,
        EmitterConfig {
            rate: 0.0,
            burst: config.clone(),
        },
        seed,
    );

    emitter.spawn_burst_internal(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_emitter_creation() {
        let config = EmitterConfig::default();
        let emitter = ParticleEmitter::new(Vec4::ZERO, config);
        assert!(emitter.is_active());
        assert!(!emitter.is_dead());
    }

    #[test]
    fn test_emitter_rate_spawning() {
        let config = EmitterConfig {
            rate: 10.0, // 10 particles per second
            burst: BurstConfig {
                count: 1,
                ..BurstConfig::default()
            },
        };
        let mut emitter = ParticleEmitter::new(Vec4::ZERO, config);

        // After 0.5 seconds at 10/sec, should spawn ~5 particles
        let particles = emitter.update(0.5);
        assert_eq!(particles.len(), 5);
    }

    #[test]
    fn test_emitter_stop() {
        let config = EmitterConfig::default();
        let mut emitter = ParticleEmitter::new(Vec4::ZERO, config);

        emitter.stop();
        assert!(!emitter.is_active());

        // Should not spawn particles when stopped
        let particles = emitter.update(1.0);
        assert!(particles.is_empty());
    }

    #[test]
    fn test_emitter_kill() {
        let config = EmitterConfig::default();
        let mut emitter = ParticleEmitter::new(Vec4::ZERO, config);

        emitter.kill();
        assert!(!emitter.is_active());
        assert!(emitter.is_dead());
    }

    #[test]
    fn test_burst_spawn() {
        let config = BurstConfig {
            count: 20,
            ..BurstConfig::default()
        };
        let particles = spawn_burst(Vec4::ZERO, &config, 12345);
        assert_eq!(particles.len(), 20);
    }

    #[test]
    fn test_emitter_position_update() {
        let config = EmitterConfig::default();
        let mut emitter = ParticleEmitter::new(Vec4::ZERO, config);

        let new_pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        emitter.set_position(new_pos);
        assert_eq!(emitter.position(), new_pos);
    }

    #[test]
    fn test_particles_spawn_at_emitter_position() {
        let pos = Vec4::new(5.0, 10.0, 15.0, 20.0);
        let config = BurstConfig {
            count: 5,
            speed: 0.0, // No velocity so position stays the same
            ..BurstConfig::default()
        };
        let particles = spawn_burst(pos, &config, 42);

        for particle in particles {
            assert_eq!(particle.position, pos);
        }
    }

    #[test]
    fn test_rng_produces_values() {
        let mut rng = SmallRng::seed_from_u64(12345);
        let v1: f32 = rng.gen_range(0.0..1.0);
        let v2: f32 = rng.gen_range(0.0..1.0);

        // Values should be in [0, 1)
        assert!((0.0..1.0).contains(&v1));
        assert!((0.0..1.0).contains(&v2));

        // Values should be different
        assert!((v1 - v2).abs() > 0.0001);
    }

    #[test]
    fn test_deterministic_with_seed() {
        let config = EmitterConfig {
            rate: 10.0,
            burst: BurstConfig {
                count: 5,
                ..BurstConfig::default()
            },
        };

        // Two emitters with the same seed should produce identical particles
        let mut emitter1 = ParticleEmitter::with_seed(Vec4::ZERO, config.clone(), 42);
        let mut emitter2 = ParticleEmitter::with_seed(Vec4::ZERO, config, 42);

        let particles1 = emitter1.update(0.5);
        let particles2 = emitter2.update(0.5);

        assert_eq!(particles1.len(), particles2.len());
        for (p1, p2) in particles1.iter().zip(particles2.iter()) {
            assert_eq!(p1.position, p2.position);
            assert_eq!(p1.velocity, p2.velocity);
        }
    }
}
