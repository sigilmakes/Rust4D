//! Particle system for 4D visual effects
//!
//! This module provides a CPU-based particle system for rendering effects like
//! muzzle flashes, explosions, sparks, and other visual effects in 4D space.
//!
//! ## Overview
//!
//! The particle system supports:
//! - **One-shot bursts**: Spawn a group of particles at a position (e.g., explosions)
//! - **Continuous emitters**: Spawn particles over time (e.g., fire, smoke trails)
//! - **4D physics**: Particles exist in 4D space with velocity, gravity, and drag
//! - **Color/size interpolation**: Particles fade and shrink over their lifetime
//! - **Blend modes**: Alpha blending or additive blending for different effects
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rust4d_render::particle::{ParticleSystem, BurstConfig, EmitterConfig, BlendMode};
//! use rust4d_math::Vec4;
//!
//! // Create the particle system
//! let mut system = ParticleSystem::new();
//!
//! // Spawn a one-shot burst (e.g., muzzle flash)
//! let burst_config = BurstConfig {
//!     count: 20,
//!     lifetime: 0.5,
//!     initial_color: [1.0, 0.8, 0.2, 1.0], // Orange
//!     end_color: [1.0, 0.2, 0.0, 0.0],     // Fade to transparent red
//!     initial_size: 0.2,
//!     end_size: 0.0,
//!     speed: 10.0,
//!     spread: 0.5, // Cone angle in radians
//!     gravity: 0.0,
//!     drag: 0.5,
//!     blend_mode: BlendMode::Additive,
//! };
//! system.spawn_burst(Vec4::new(0.0, 0.0, 0.0, 0.0), &burst_config);
//!
//! // Create a continuous emitter (e.g., torch flame)
//! let emitter_config = EmitterConfig {
//!     rate: 50.0, // Particles per second
//!     burst: BurstConfig {
//!         count: 1,
//!         lifetime: 1.0,
//!         initial_color: [1.0, 0.5, 0.0, 1.0],
//!         end_color: [0.5, 0.0, 0.0, 0.0],
//!         initial_size: 0.1,
//!         end_size: 0.3,
//!         speed: 3.0,
//!         spread: 0.2,
//!         gravity: 2.0, // Fire rises
//!         drag: 0.3,
//!         blend_mode: BlendMode::Additive,
//!     },
//! };
//! let emitter_id = system.spawn_emitter(Vec4::ZERO, emitter_config);
//!
//! // Each frame, update the system
//! let dt = 1.0 / 60.0;
//! system.update(dt);
//!
//! // Get particles for rendering
//! for particle in system.particles() {
//!     // Render the particle at particle.position with particle.color and particle.size
//! }
//!
//! // Move the emitter (e.g., attached to a moving object)
//! system.update_emitter_position(emitter_id, Vec4::new(1.0, 0.0, 0.0, 0.0));
//!
//! // Stop the emitter (existing particles continue until they die)
//! system.stop_emitter(emitter_id);
//!
//! // Kill the emitter (removes it entirely)
//! system.kill_emitter(emitter_id);
//! ```
//!
//! ## 4D Considerations
//!
//! Particles exist in full 4D space. When rendering, they will need to be sliced
//! to 3D like other geometry. Particles close to the current W-slice will be
//! visible; those far away will be clipped or faded.

mod emitter;
mod system;
mod types;

// Re-export public types
pub use emitter::ParticleEmitter;
pub use system::ParticleSystem;
pub use types::{BlendMode, BurstConfig, EmitterConfig, Particle};
