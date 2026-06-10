//! Rendering pipeline components
//!
//! This module contains the compute and render pipelines for
//! 4D cross-section rendering.

pub mod lookup_tables;
pub mod render_pipeline;
pub mod slice_pipeline;
pub mod types;

// Re-export lookup tables (tetrahedra tables only)
pub use lookup_tables::{
    tetra_crossed_edges, tetra_edge_count, TETRA_EDGES, TETRA_EDGE_TABLE, TETRA_TRI_COUNT,
    TETRA_TRI_TABLE,
};

// Re-export types
pub use types::{
    AtomicCounter, GpuTetrahedron, RenderUniforms, SliceParams, Vertex3D, Vertex4D,
    MAX_OUTPUT_TRIANGLES, TRIANGLE_VERTEX_COUNT,
};

// Re-export pipelines
pub use render_pipeline::{
    look_at_matrix, mat4_mul, perspective_matrix, DrawIndirectArgs, RenderPipeline,
};
pub use slice_pipeline::SlicePipeline;
