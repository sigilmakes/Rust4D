//! Core types for the Rust4D engine
//!
//! This crate provides the foundational types for building 4D worlds:
//!
//! - [`Transform4D`] - Position, rotation, and scale in 4D space
//! - [`Material`] - Visual properties of an entity
//! - [`ShapeRef`] - Reference to a shape (shared or owned)
//! - [`World`] - ECS-backed container for all entities
//! - [`Name`], [`Tags`], [`PhysicsBody`] - ECS components
//! - [`ShapeTemplate`] - Serializable shape template
//! - [`EntityTemplate`] - Serializable entity template
//! - [`Scene`] - Loadable/saveable scene containing entities

mod transform;
mod entity;
mod world;
mod components;
mod shapes;
mod scene;
mod scene_manager;
mod asset_error;
mod asset_cache;
mod scene_transition;
mod scene_loader;
mod scene_validator;

pub use transform::Transform4D;
pub use entity::{Material, ShapeRef, DirtyFlags, EntityTemplate};
pub use world::{World, HierarchyError};
pub use components::{Name, Tags, PhysicsBody, Parent, Children};
pub use shapes::{ColliderHint, ShapeTemplate};
pub use scene::{Scene, SceneLoadError, SceneSaveError, SceneError, ActiveScene};
pub use scene_manager::SceneManager;
pub use asset_error::AssetError;
pub use asset_cache::{AssetId, AssetHandle, Asset, AssetCache};
pub use scene_transition::{SceneTransition, TransitionEffect, SlideDirection};
pub use scene_loader::{SceneLoader, LoadResult};
pub use scene_validator::{SceneValidator, ValidationError};

/// Type alias for entity handles (hecs::Entity)
pub type EntityId = hecs::Entity;

// Re-export commonly used types from rust4d_math for convenience
pub use rust4d_math::{Vec4, Rotor4, RotationPlane, ConvexShape4D, Tetrahedron};
pub use rust4d_math::{Tesseract4D, Hyperplane4D};

// Re-export physics types for convenient access through rust4d_core
pub use rust4d_physics::{BodyKey, PhysicsConfig, PhysicsWorld, RigidBody4D, StaticCollider};

// Re-export hecs::Entity for consumers
pub use hecs;
