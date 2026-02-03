//! 4D Physics simulation for Rust4D
//!
//! This crate provides physics simulation for 4D rigid bodies, including:
//! - Collision shapes (spheres, AABBs, planes)
//! - Collision detection
//! - Rigid body dynamics with gravity
//! - Player physics for FPS-style movement

pub mod body;
pub mod collision;
pub mod material;
pub mod raycast;
pub mod shapes;
pub mod world;

// Re-export commonly used types
pub use body::{BodyKey, BodyType, RigidBody4D, StaticCollider};
pub use collision::{aabb_vs_aabb, aabb_vs_plane, sphere_vs_aabb, sphere_vs_plane, sphere_vs_sphere, CollisionFilter, CollisionLayer, Contact};
pub use material::PhysicsMaterial;
pub use shapes::{Collider, Plane4D, Sphere4D, AABB4D};
pub use raycast::{RayHit, ray_vs_sphere, ray_vs_aabb, ray_vs_plane, ray_vs_collider};
pub use world::{PhysicsConfig, PhysicsWorld, RayTarget, WorldRayHit};
