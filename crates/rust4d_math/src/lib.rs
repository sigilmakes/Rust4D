//! 4D Mathematics Library
//!
//! This crate provides 4D vector, rotation, and shape types for the Rust4D engine.
//!
//! ## Core Types
//!
//! - [`Vec4`] - 4D vector with x, y, z, w components
//! - [`Rotor4`] - 4D rotation using geometric algebra
//! - [`Mat4`] - 4x4 matrix for transformations
//!
//! ## Shape Types
//!
//! - [`ConvexShape4D`] - Trait for 4D shapes that can be sliced
//! - [`Tetrahedron`] - A 3-simplex defined by vertex indices
//! - [`Mesh4D`] - General tetrahedral mesh (merge, weld, validate, measure)
//! - [`Tesseract4D`] - A 4D hypercube
//! - [`Hyperplane4D`] - A floor/ground plane in 4D
//!
//! ## Primitive Catalog
//!
//! The [`primitives`] module constructs the regular 4-polytopes (5-cell,
//! 16-cell, 24-cell, 600-cell) and the curved shapes (hypersphere,
//! spherinder, cubinder, duocylinder) as watertight boundary meshes.

pub mod hyperplane;
pub mod interpolation;
pub mod mat4;
pub mod mesh4d;
pub mod primitives;
pub mod ray;
mod rotor4;
pub mod shape;
pub mod tesseract;
mod vec4;

pub use hyperplane::Hyperplane4D;
pub use interpolation::Interpolatable;
pub use mat4::Mat4;
pub use mesh4d::{Mesh4D, MeshError};
pub use ray::Ray4D;
pub use rotor4::{RotationPlane, Rotor4};
pub use shape::{ConvexShape4D, Tetrahedron};
pub use tesseract::Tesseract4D;
pub use vec4::Vec4;
