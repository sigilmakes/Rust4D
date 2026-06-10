//! 4D Physics simulation for Rust4D
//!
//! This crate provides physics simulation for 4D rigid bodies, including:
//! - Collision shapes (spheres, AABBs, planes)
//! - Collision detection
//! - Rigid body dynamics with gravity
//! - Player physics for FPS-style movement
//! - Spatial queries (sphere searches, area effects, line of sight)
//!
//! # Module Organization
//!
//! The crate is organized into focused modules:
//!
//! - [`body`] - Rigid body and static collider types
//! - [`collision`] - Collision detection and layer filtering
//! - [`material`] - Physics material properties (friction, restitution)
//! - [`raycast`] - Ray intersection tests
//! - [`shapes`] - Collision shape primitives
//! - [`spatial`] - Spatial query result types
//! - [`world`] - The main `PhysicsWorld` simulation
//!
//! # Re-export Strategy
//!
//! Commonly used types are re-exported at the crate root for convenience.
//! This flat re-export style prioritizes ergonomics for typical use cases:
//!
//! ```ignore
//! // Convenient flat import
//! use rust4d_physics::{PhysicsWorld, RigidBody4D, CollisionLayer};
//!
//! // Module path still available for disambiguation or organization
//! use rust4d_physics::collision::CollisionFilter;
//! use rust4d_physics::shapes::{Sphere4D, AABB4D};
//! ```
//!
//! The re-exports are grouped by category:
//! - **Bodies**: `BodyKey`, `BodyType`, `RigidBody4D`, `StaticCollider`
//! - **Collision**: Detection functions, `CollisionEvent`, `CollisionFilter`, `CollisionLayer`, `Contact`
//! - **Materials**: `PhysicsMaterial`
//! - **Shapes**: `Collider`, `Plane4D`, `Sphere4D`, `AABB4D`
//! - **Raycasting**: `RayHit`, intersection functions
//! - **Spatial queries**: `SpatialQueryResult`, `AreaEffectHit`
//! - **World**: `PhysicsConfig`, `PhysicsWorld`, `RayTarget`, `WorldRayHit`

pub mod body;
pub mod collision;
pub mod material;
pub mod raycast;
pub mod shapes;
pub mod spatial;
pub mod world;

// Re-export commonly used types (see module doc for rationale)
pub use body::{BodyKey, BodyType, RigidBody4D, StaticCollider};
pub use collision::{
    aabb_vs_aabb, aabb_vs_plane, sphere_vs_aabb, sphere_vs_plane, sphere_vs_sphere, CollisionEvent,
    CollisionEventKind, CollisionFilter, CollisionLayer, Contact,
};
pub use material::PhysicsMaterial;
pub use raycast::{ray_vs_aabb, ray_vs_collider, ray_vs_plane, ray_vs_sphere, RayHit};
pub use shapes::{Collider, Plane4D, Sphere4D, AABB4D};
pub use spatial::{AreaEffectHit, SpatialQueryResult};
pub use world::{PhysicsConfig, PhysicsWorld, RayTarget, WorldRayHit};
