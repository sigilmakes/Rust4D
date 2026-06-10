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

mod asset_cache;
mod asset_error;
mod components;
mod entity;
mod scene;
mod scene_loader;
mod scene_manager;
mod scene_transition;
mod scene_validator;
mod shapes;
mod transform;
mod world;

pub use asset_cache::{Asset, AssetCache, AssetHandle, AssetId};
pub use asset_error::AssetError;
pub use components::{Children, Name, Parent, PhysicsBody, Tags};
pub use entity::{DirtyFlags, EntityTemplate, Material, ShapeRef};
pub use scene::{ActiveScene, Scene, SceneError, SceneLoadError, SceneSaveError};
pub use scene_loader::{LoadResult, SceneLoader};
pub use scene_manager::SceneManager;
pub use scene_transition::{SceneTransition, SlideDirection, TransitionEffect};
pub use scene_validator::{SceneValidator, ValidationError};
pub use shapes::{ColliderHint, ShapeTemplate};
pub use transform::Transform4D;
pub use world::{HierarchyError, World};

/// Type alias for entity handles (hecs::Entity)
pub type EntityId = hecs::Entity;

// Re-export commonly used types from rust4d_math for convenience
pub use rust4d_math::{ConvexShape4D, RotationPlane, Rotor4, Tetrahedron, Vec4};
pub use rust4d_math::{Hyperplane4D, Tesseract4D};

// Re-export physics types for convenient access through rust4d_core
pub use rust4d_physics::{BodyKey, PhysicsConfig, PhysicsWorld, RigidBody4D, StaticCollider};

// Re-export hecs::Entity for consumers
pub use hecs;
