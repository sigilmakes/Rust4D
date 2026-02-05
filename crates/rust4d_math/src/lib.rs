//! 4D Mathematics Library
//!
//! This crate provides 4D vector, rotation, and shape types for the Rust4D engine.
//!
//! ## Core Types
//!
//! - [`Vec4`] - 4D vector with x, y, z, w components
//! - [`Rotor4`] - 4D rotation using geometric algebra
//! - [`Mat4`] - 4x4 matrix for transformations
//! - [`Ray4D`] - 4D ray for raycasting
//!
//! ## Shape Types
//!
//! - [`ConvexShape4D`] - Trait for 4D shapes that can be sliced
//! - [`Tetrahedron`] - A 3-simplex defined by vertex indices
//! - [`Tesseract4D`] - A 4D hypercube
//! - [`Hyperplane4D`] - A floor/ground plane in 4D

mod vec4;
mod rotor4;
pub mod mat4;
pub mod shape;
pub mod tesseract;
pub mod hyperplane;
pub mod ray;

pub use vec4::Vec4;
pub use rotor4::{Rotor4, RotationPlane};
pub use mat4::Mat4;
pub use shape::{ConvexShape4D, Tetrahedron};
pub use tesseract::Tesseract4D;
pub use hyperplane::Hyperplane4D;
pub use ray::Ray4D;
