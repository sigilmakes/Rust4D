//! Particle emitter for continuous particle emission

use rust4d_math::Vec4;
use super::types::{Particle, EmitterConfig, BurstConfig};

/// Random number generator state (simple xorshift for portability)
#[derive(Clone, Debug)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a random f32 in [0, 1)
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f64 / u64::MAX as f64) as f32
    }

    /// Generate a random f32 in [-1, 1)
    fn next_f32_signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

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
    /// Random number generator
    rng: Rng,
}

impl ParticleEmitter {
    /// Create a new particle emitter at the given position
    pub fn new(position: Vec4, config: EmitterConfig) -> Self {
        // Use position as seed for some variation
        let seed = (position.x.to_bits() as u64)
            .wrapping_mul(31)
            .wrapping_add(position.y.to_bits() as u64)
            .wrapping_mul(37)
            .wrapping_add(position.z.to_bits() as u64)
            .wrapping_mul(41)
            .wrapping_add(position.w.to_bits() as u64);

        Self {
            config,
            position,
            accumulator: 0.0,
            active: true,
            dead: false,
            rng: Rng::new(seed.wrapping_add(1)),
        }
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
                config.lifetime,
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

        let x = self.rng.next_f32_signed();
        let y = self.rng.next_f32_signed();
        let z = self.rng.next_f32_signed();
        let w = self.rng.next_f32_signed();

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
pub fn spawn_burst(position: Vec4, config: &BurstConfig, seed: u64) -> Vec<Particle> {
    let mut emitter = ParticleEmitter {
        config: EmitterConfig {
            rate: 0.0,
            burst: config.clone(),
        },
        position,
        accumulator: 0.0,
        active: true,
        dead: false,
        rng: Rng::new(seed.wrapping_add(1)),
    };

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
        let mut rng = Rng::new(12345);
        let v1 = rng.next_f32();
        let v2 = rng.next_f32();

        // Values should be in [0, 1)
        assert!(v1 >= 0.0 && v1 < 1.0);
        assert!(v2 >= 0.0 && v2 < 1.0);

        // Values should be different
        assert!((v1 - v2).abs() > 0.0001);
    }
}
