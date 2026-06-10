//! 4D Rendering Library
//!
//! This crate provides the wgpu-based rendering pipeline for displaying
//! 3D cross-sections of 4D geometry.
//!
//! ## Key Components
//!
//! - [`context::RenderContext`] - WGPU device, queue, and surface management
//! - [`camera4d::Camera4D`] - 4D camera with position and rotation
//! - [`pipeline::SlicePipeline`] - Compute shader for 4D->3D slicing
//! - [`pipeline::RenderPipeline`] - 3D rendering with lighting
//! - [`renderable::RenderableGeometry`] - Converts World/Entity to GPU buffers
//! - [`sprite::SpriteBatch`] - 2D billboard sprites in 4D space with W-fade
//! - [`particle::ParticleSystem`] - 4D particle effects with physics
//! - [`egui_overlay::EguiRenderer`] - 2D HUD overlay with egui
//!
//! ## Shapes
//!
//! Shape geometry is defined in `rust4d_math`. This crate re-exports the shapes
//! for convenience, but you can also import them directly from `rust4d_math`.

pub mod camera4d;
pub mod context;
pub mod egui_overlay;
pub mod particle;
pub mod pipeline;
pub mod renderable;
pub mod sprite;

// Re-export core types for convenience
pub use rust4d_core::{ConvexShape4D, Hyperplane4D, Tesseract4D, Tetrahedron};
pub use rust4d_core::{DirtyFlags, Material, ShapeRef, Tags, Transform4D, World};
pub use rust4d_core::{RotationPlane, Rotor4, Vec4};

// Re-export renderable for easy access
pub use renderable::{position_gradient_color, CheckerboardGeometry, RenderableGeometry};

// Re-export sprite types for easy access
pub use sprite::{Sprite, SpriteBatch, SpriteSheet, WFadeConfig};

// Re-export particle types for easy access
pub use particle::{
    BlendMode, BurstConfig, EmitterConfig, Particle, ParticleEmitter, ParticleSystem,
};

// Re-export egui overlay types for easy access
pub use egui_overlay::{EguiRenderer, HudContext, RawInput, ScreenDescriptor};
