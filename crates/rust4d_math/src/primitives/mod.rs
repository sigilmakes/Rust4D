//! Procedural 4D primitive shapes
//!
//! This module is the engine's shape catalog. Every function returns a
//! [`Mesh4D`](crate::Mesh4D) containing a **tetrahedralized boundary** —
//! a closed 3-manifold ready for the slice pipeline (see `docs/shapes.md`
//! in the repository for the full catalog with pictures and the underlying
//! math).
//!
//! # Regular polytopes ([`polytopes`])
//!
//! - [`pentachoron`] — 5-cell, the 4D tetrahedron
//! - [`hexadecachoron`] — 16-cell, the 4D octahedron
//! - [`icositetrachoron`] — 24-cell, 4D's unique extra regular solid
//! - [`hexacosichoron`] — 600-cell, the 4D icosahedron (600 cells, built
//!   from the binary icosahedral group)
//!
//! (The tesseract predates this module and lives at [`crate::Tesseract4D`].)
//!
//! # Curved shapes ([`curved`])
//!
//! - [`hypersphere`] — solid 4-ball bounded by S³
//! - [`spherinder`] — ball × segment
//! - [`cubinder`] — disk × square
//! - [`duocylinder`] — disk × disk, bounded by two solid tori
//!
//! # Quality guarantees
//!
//! Every primitive is pinned by tests on three properties:
//!
//! 1. **Structure** — exact vertex/cell counts,
//! 2. **Watertightness** — every triangular face shared by exactly two
//!    cells ([`Mesh4D::is_watertight`](crate::Mesh4D::is_watertight)), so
//!    slices never show cracks or T-junctions,
//! 3. **Measure** — total boundary 3-volume matches the closed-form value
//!    (exactly for polytopes, convergent from below for curved shapes).

pub mod curved;
pub mod extrude;
pub mod polytopes;

pub use curved::{cubinder, duocylinder, hypersphere, spherinder};
pub use extrude::split_prism;
pub use polytopes::{hexacosichoron, hexadecachoron, icositetrachoron, pentachoron};
