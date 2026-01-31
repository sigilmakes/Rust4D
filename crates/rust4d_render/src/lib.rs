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
//!
//! ## Shapes
//!
//! Shape geometry is defined in `rust4d_math`. This crate re-exports the shapes
//! for convenience, but you can also import them directly from `rust4d_math`.

pub mod context;
pub mod camera4d;
pub mod pipeline;
pub mod renderable;

// Re-export core types for convenience
pub use rust4d_core::{World, Entity, Transform4D, Material, ShapeRef, DirtyFlags, Tags};
pub use rust4d_core::{ConvexShape4D, Tetrahedron, Tesseract4D, Hyperplane4D};
pub use rust4d_core::{Vec4, Rotor4, RotationPlane};

// Re-export renderable for easy access
pub use renderable::{RenderableGeometry, CheckerboardGeometry, position_gradient_color};
