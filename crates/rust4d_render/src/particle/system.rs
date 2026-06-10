//! Particle system for managing particles and emitters
//!
//! # Design Note: Emitter Storage
//!
//! The particle system uses `Vec<Option<ParticleEmitter>>` with a free list rather than
//! a `slotmap` crate. This design choice was made for several reasons:
//!
//! 1. **Stability over external crates**: The free list pattern is simple and well-understood,
//!    with no external dependencies. It provides O(1) allocation and deallocation.
//!
//! 2. **Index stability**: Returned emitter indices remain valid as long as the emitter exists,
//!    and become invalid (point to `None`) after the emitter is killed. This matches the
//!    expected behavior for entity handles.
//!
//! 3. **Minimal overhead**: The free list is just a `Vec<usize>` that grows only as needed.
//!    Memory overhead is one `Option` discriminant per slot.
//!
//! 4. **Debuggability**: The storage is trivially inspectable. `Vec<Option<T>>` appears directly
//!    in debuggers without custom formatters.
//!
//! If the particle system grows to manage thousands of emitters with frequent churn, consider
//! migrating to `slotmap` for its generational indices (which detect use-after-free).

use rust4d_math::Vec4;
use super::emitter::{ParticleEmitter, spawn_burst};
use super::types::{Particle, BurstConfig, EmitterConfig};

/// The main particle system that manages all particles and emitters
///
/// Particles are stored in a flat `Vec` and updated each frame. Dead particles are
/// removed using `retain_mut`. Emitters are stored in a `Vec<Option<_>>` with a free
/// list for O(1) slot allocation and reuse.
#[derive(Debug)]
pub struct ParticleSystem {
    /// All active particles (dead particles are removed each update)
    particles: Vec<Particle>,
    /// All particle emitters (None = free slot)
    emitters: Vec<Option<ParticleEmitter>>,
    /// Counter for generating unique seeds for particle randomization
    seed_counter: u64,
    /// Stack of free emitter slot indices for O(1) allocation
    free_slots: Vec<usize>,
}

impl ParticleSystem {
    /// Create a new empty particle system
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            emitters: Vec::new(),
            seed_counter: 1,
            free_slots: Vec::new(),
        }
    }

    /// Create a particle system with pre-allocated capacity
    pub fn with_capacity(particle_capacity: usize, emitter_capacity: usize) -> Self {
        Self {
            particles: Vec::with_capacity(particle_capacity),
            emitters: Vec::with_capacity(emitter_capacity),
            seed_counter: 1,
            free_slots: Vec::new(),
        }
    }

    /// Spawn a one-time burst of particles at a position
    pub fn spawn_burst(&mut self, position: Vec4, config: &BurstConfig) {
        let seed = self.next_seed();
        let new_particles = spawn_burst(position, config, seed);
        self.particles.extend(new_particles);
    }

    /// Create a continuous emitter and return its index
    pub fn spawn_emitter(&mut self, position: Vec4, config: EmitterConfig) -> usize {
        let emitter = ParticleEmitter::new(position, config);

        // Try to reuse a free slot
        if let Some(index) = self.free_slots.pop() {
            self.emitters[index] = Some(emitter);
            return index;
        }

        // Otherwise, add to the end
        let index = self.emitters.len();
        self.emitters.push(Some(emitter));
        index
    }

    /// Update the position of an emitter
    pub fn update_emitter_position(&mut self, index: usize, position: Vec4) {
        if let Some(Some(emitter)) = self.emitters.get_mut(index) {
            emitter.set_position(position);
        }
    }

    /// Stop an emitter (it will stop spawning but existing particles continue)
    pub fn stop_emitter(&mut self, index: usize) {
        if let Some(Some(emitter)) = self.emitters.get_mut(index) {
            emitter.stop();
        }
    }

    /// Start an emitter that was previously stopped
    pub fn start_emitter(&mut self, index: usize) {
        if let Some(Some(emitter)) = self.emitters.get_mut(index) {
            emitter.start();
        }
    }

    /// Kill an emitter (stops and marks for removal)
    pub fn kill_emitter(&mut self, index: usize) {
        if let Some(Some(emitter)) = self.emitters.get_mut(index) {
            emitter.kill();
        }
    }

    /// Get a reference to an emitter
    pub fn get_emitter(&self, index: usize) -> Option<&ParticleEmitter> {
        self.emitters.get(index).and_then(|e| e.as_ref())
    }

    /// Get a mutable reference to an emitter
    pub fn get_emitter_mut(&mut self, index: usize) -> Option<&mut ParticleEmitter> {
        self.emitters.get_mut(index).and_then(|e| e.as_mut())
    }

    /// Update all particles and emitters
    pub fn update(&mut self, dt: f32) {
        // Update all emitters and collect new particles
        for emitter in self.emitters.iter_mut().flatten() {
            let new_particles = emitter.update(dt);
            self.particles.extend(new_particles);
        }

        // Remove dead emitters and free their slots
        for (index, emitter_opt) in self.emitters.iter_mut().enumerate() {
            if let Some(emitter) = emitter_opt {
                if emitter.is_dead() {
                    *emitter_opt = None;
                    self.free_slots.push(index);
                }
            }
        }

        // Update all particles and remove dead ones
        self.particles.retain_mut(|particle| particle.update(dt));
    }

    /// Get all particles for rendering
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Get the number of active particles
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get the number of active emitters
    pub fn emitter_count(&self) -> usize {
        self.emitters.iter().filter(|e| e.is_some()).count()
    }

    /// Clear all particles and emitters
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emitters.clear();
        self.free_slots.clear();
    }

    /// Clear all particles but keep emitters
    pub fn clear_particles(&mut self) {
        self.particles.clear();
    }

    /// Generate a unique seed for randomization
    fn next_seed(&mut self) -> u64 {
        let seed = self.seed_counter;
        self.seed_counter = self.seed_counter.wrapping_add(1);
        seed
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_system_creation() {
        let system = ParticleSystem::new();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.emitter_count(), 0);
    }

    #[test]
    fn test_spawn_burst() {
        let mut system = ParticleSystem::new();
        let config = BurstConfig {
            count: 10,
            ..BurstConfig::default()
        };
        system.spawn_burst(Vec4::ZERO, &config);
        assert_eq!(system.particle_count(), 10);
    }

    #[test]
    fn test_spawn_emitter() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig {
            rate: 10.0,
            burst: BurstConfig {
                count: 1,
                ..BurstConfig::default()
            },
        };
        let index = system.spawn_emitter(Vec4::ZERO, config);
        assert_eq!(system.emitter_count(), 1);
        assert!(system.get_emitter(index).is_some());
    }

    #[test]
    fn test_emitter_spawns_particles() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig {
            rate: 100.0, // High rate for testing
            burst: BurstConfig {
                count: 1,
                ..BurstConfig::default()
            },
        };
        system.spawn_emitter(Vec4::ZERO, config);

        // Update for 0.5 seconds at 100/sec = 50 particles
        system.update(0.5);
        assert_eq!(system.particle_count(), 50);
    }

    #[test]
    fn test_particles_die_over_time() {
        let mut system = ParticleSystem::new();
        let config = BurstConfig {
            count: 10,
            lifetime: 0.5,
            ..BurstConfig::default()
        };
        system.spawn_burst(Vec4::ZERO, &config);
        assert_eq!(system.particle_count(), 10);

        // Update past lifetime
        system.update(1.0);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_update_emitter_position() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig::default();
        let index = system.spawn_emitter(Vec4::ZERO, config);

        let new_pos = Vec4::new(5.0, 5.0, 5.0, 5.0);
        system.update_emitter_position(index, new_pos);

        let emitter = system.get_emitter(index).unwrap();
        assert_eq!(emitter.position(), new_pos);
    }

    #[test]
    fn test_stop_emitter() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig {
            rate: 100.0,
            burst: BurstConfig {
                count: 1,
                ..BurstConfig::default()
            },
        };
        let index = system.spawn_emitter(Vec4::ZERO, config);

        // Stop the emitter
        system.stop_emitter(index);

        // Update should not spawn particles
        system.update(1.0);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_kill_emitter() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig::default();
        let index = system.spawn_emitter(Vec4::ZERO, config);
        assert_eq!(system.emitter_count(), 1);

        // Kill the emitter
        system.kill_emitter(index);

        // Update to process the kill
        system.update(0.01);
        assert_eq!(system.emitter_count(), 0);
    }

    #[test]
    fn test_emitter_slot_reuse() {
        let mut system = ParticleSystem::new();
        let config = EmitterConfig::default();

        // Create and kill first emitter
        let index1 = system.spawn_emitter(Vec4::ZERO, config.clone());
        system.kill_emitter(index1);
        system.update(0.01);

        // Create second emitter - should reuse slot
        let index2 = system.spawn_emitter(Vec4::Y, config);
        assert_eq!(index1, index2); // Should reuse the same index
    }

    #[test]
    fn test_clear() {
        let mut system = ParticleSystem::new();
        let burst_config = BurstConfig {
            count: 10,
            ..BurstConfig::default()
        };
        system.spawn_burst(Vec4::ZERO, &burst_config);
        system.spawn_emitter(Vec4::ZERO, EmitterConfig::default());

        assert!(system.particle_count() > 0);
        assert!(system.emitter_count() > 0);

        system.clear();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.emitter_count(), 0);
    }

    #[test]
    fn test_gravity_integration() {
        let mut system = ParticleSystem::new();
        let config = BurstConfig {
            count: 1,
            lifetime: 10.0,
            gravity: -10.0,
            speed: 0.0, // Start stationary
            drag: 0.0,
            ..BurstConfig::default()
        };
        system.spawn_burst(Vec4::new(0.0, 10.0, 0.0, 0.0), &config);

        // Update for 1 second
        system.update(1.0);

        // Particle should have fallen (y should be less than 10)
        let particle = &system.particles()[0];
        assert!(particle.position.y < 10.0);
        assert!(particle.velocity.y < 0.0);
    }
}
