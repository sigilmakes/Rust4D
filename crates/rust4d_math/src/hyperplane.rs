//! 4D Hyperplane geometry (floor/ground plane)
//!
//! A hyperplane in 4D is a 3D subspace. For a "floor", we create a plane
//! that extends in X, Z, and W dimensions.
//!
//! The hyperplane is created in **local space** with the bottom surface at y=0
//! and thickness extending upward to y=thickness. The entity transform is used
//! to position it in world space.
//!
//! To be sliceable by the W-plane, the hyperplane must have extent in W.
//! We model it as a grid of "pillars" - each pillar is a rectangular prism
//! extending in W, decomposed into tetrahedra.

use crate::{
    shape::{ConvexShape4D, Tetrahedron},
    Vec4,
};
use std::collections::HashSet;

/// A hyperplane floor in local space - pure geometry without colors
///
/// The hyperplane is created in local space with the bottom surface at y=0
/// and extends upward by `thickness`. Use the entity transform to position
/// it in world space.
///
/// The hyperplane extends as a grid in X and Z, with extent in W for slicing.
#[derive(Clone)]
pub struct Hyperplane4D {
    /// Half-extent in X and Z
    half_size: f32,
    /// Grid size (cells per axis)
    grid_size: usize,
    /// Half-extent in W dimension
    w_extent: f32,
    /// All vertices
    vertices: Vec<Vec4>,
    /// Tetrahedra decomposition
    tetrahedra: Vec<Tetrahedron>,
}

impl Hyperplane4D {
    /// Create a new hyperplane in local space
    ///
    /// The hyperplane is created with the bottom surface at y=0 and extends
    /// upward by `thickness`. Use the entity transform to position it in world space.
    ///
    /// # Arguments
    /// * `size` - Half-extent in X and Z (total size is 2*size)
    /// * `grid_size` - Number of cells along each axis
    /// * `w_extent` - Half-extent in W dimension (for slicing visibility)
    /// * `thickness` - Y thickness (bottom at y=0, top at y=thickness)
    pub fn new(size: f32, grid_size: usize, w_extent: f32, thickness: f32) -> Self {
        let mut vertices = Vec::new();
        let mut tetrahedra = Vec::new();

        let step = size * 2.0 / grid_size as f32;
        let start = -size;

        // Create grid of cells, each cell is a rectangular prism in 4D
        for i in 0..grid_size {
            for j in 0..grid_size {
                let x0 = start + i as f32 * step;
                let x1 = x0 + step;
                let z0 = start + j as f32 * step;
                let z1 = z0 + step;

                let base_idx = vertices.len();

                // Local space: bottom at y=0, top at y=thickness
                let y0 = 0.0;
                let y1 = thickness;
                let w0 = -w_extent;
                let w1 = w_extent;

                // 16 vertices of the tesseract-shaped cell
                // Using binary indexing: bit 0 = x, bit 1 = y, bit 2 = z, bit 3 = w
                vertices.push(Vec4::new(x0, y0, z0, w0)); // 0 = 0b0000
                vertices.push(Vec4::new(x1, y0, z0, w0)); // 1 = 0b0001
                vertices.push(Vec4::new(x0, y1, z0, w0)); // 2 = 0b0010
                vertices.push(Vec4::new(x1, y1, z0, w0)); // 3 = 0b0011
                vertices.push(Vec4::new(x0, y0, z1, w0)); // 4 = 0b0100
                vertices.push(Vec4::new(x1, y0, z1, w0)); // 5 = 0b0101
                vertices.push(Vec4::new(x0, y1, z1, w0)); // 6 = 0b0110
                vertices.push(Vec4::new(x1, y1, z1, w0)); // 7 = 0b0111
                vertices.push(Vec4::new(x0, y0, z0, w1)); // 8 = 0b1000
                vertices.push(Vec4::new(x1, y0, z0, w1)); // 9 = 0b1001
                vertices.push(Vec4::new(x0, y1, z0, w1)); // 10 = 0b1010
                vertices.push(Vec4::new(x1, y1, z0, w1)); // 11 = 0b1011
                vertices.push(Vec4::new(x0, y0, z1, w1)); // 12 = 0b1100
                vertices.push(Vec4::new(x1, y0, z1, w1)); // 13 = 0b1101
                vertices.push(Vec4::new(x0, y1, z1, w1)); // 14 = 0b1110
                vertices.push(Vec4::new(x1, y1, z1, w1)); // 15 = 0b1111

                // Decompose the tesseract-shaped cell into tetrahedra
                let cell_tetrahedra = Self::decompose_cell_to_tetrahedra(base_idx);
                tetrahedra.extend(cell_tetrahedra);
            }
        }

        Self {
            half_size: size,
            grid_size,
            w_extent,
            vertices,
            tetrahedra,
        }
    }

    /// Get the half-size in X and Z
    #[inline]
    pub fn half_size(&self) -> f32 {
        self.half_size
    }

    /// Get the grid size
    #[inline]
    pub fn grid_size(&self) -> usize {
        self.grid_size
    }

    /// Get the W extent
    #[inline]
    pub fn w_extent(&self) -> f32 {
        self.w_extent
    }

    /// Get the grid cell coordinates for a given cell index
    ///
    /// Returns (i, j) grid coordinates for checkerboard patterns
    pub fn cell_coords(&self, cell_index: usize) -> (usize, usize) {
        let i = cell_index / self.grid_size;
        let j = cell_index % self.grid_size;
        (i, j)
    }

    /// Get the number of cells
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.grid_size * self.grid_size
    }

    /// Decompose a single cell (mini-tesseract) into tetrahedra using Kuhn triangulation
    fn decompose_cell_to_tetrahedra(base_idx: usize) -> Vec<Tetrahedron> {
        let permutations = [
            [0, 1, 2, 3],
            [0, 1, 3, 2],
            [0, 2, 1, 3],
            [0, 2, 3, 1],
            [0, 3, 1, 2],
            [0, 3, 2, 1],
            [1, 0, 2, 3],
            [1, 0, 3, 2],
            [1, 2, 0, 3],
            [1, 2, 3, 0],
            [1, 3, 0, 2],
            [1, 3, 2, 0],
            [2, 0, 1, 3],
            [2, 0, 3, 1],
            [2, 1, 0, 3],
            [2, 1, 3, 0],
            [2, 3, 0, 1],
            [2, 3, 1, 0],
            [3, 0, 1, 2],
            [3, 0, 2, 1],
            [3, 1, 0, 2],
            [3, 1, 2, 0],
            [3, 2, 0, 1],
            [3, 2, 1, 0],
        ];

        let mut simplices = Vec::with_capacity(24);
        for perm in &permutations {
            let mut vertex_indices = [0usize; 5];
            let mut current = 0usize;
            vertex_indices[0] = current;
            for (i, &dim) in perm.iter().enumerate() {
                current |= 1 << dim;
                vertex_indices[i + 1] = current;
            }
            simplices.push(vertex_indices);
        }

        let mut tetrahedra = Vec::new();
        let mut seen: HashSet<[usize; 4]> = HashSet::new();

        for simplex in &simplices {
            for omit in 0..5 {
                let mut tet_verts = [0usize; 4];
                let mut idx = 0;
                for (i, &vert) in simplex.iter().enumerate() {
                    if i != omit {
                        tet_verts[idx] = base_idx + vert;
                        idx += 1;
                    }
                }

                let mut canonical = tet_verts;
                canonical.sort();

                if seen.insert(canonical) {
                    tetrahedra.push(Tetrahedron::new(tet_verts));
                }
            }
        }

        tetrahedra
    }
}

impl ConvexShape4D for Hyperplane4D {
    fn vertices(&self) -> &[Vec4] {
        &self.vertices
    }

    fn tetrahedra(&self) -> &[Tetrahedron] {
        &self.tetrahedra
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperplane_creation() {
        let plane = Hyperplane4D::new(4.0, 4, 2.0, 0.01);

        // 4x4 grid = 16 cells, each with 16 vertices
        assert_eq!(plane.vertices().len(), 16 * 16);
        assert!(!plane.tetrahedra().is_empty());
    }

    #[test]
    fn test_hyperplane_vertex_positions_local_space() {
        let plane = Hyperplane4D::new(4.0, 2, 2.0, 0.1);

        // Check that all vertices are in local space: y=0 to y=thickness
        for v in plane.vertices() {
            assert!(
                v.y >= 0.0 && v.y <= 0.1,
                "Vertex Y should be between 0 and thickness, got {}",
                v.y
            );
        }
    }

    #[test]
    fn test_hyperplane_cell_coords() {
        let plane = Hyperplane4D::new(4.0, 4, 2.0, 0.01);

        assert_eq!(plane.cell_coords(0), (0, 0));
        assert_eq!(plane.cell_coords(1), (0, 1));
        assert_eq!(plane.cell_coords(4), (1, 0));
        assert_eq!(plane.cell_coords(15), (3, 3));
    }

    #[test]
    fn test_hyperplane_accessors() {
        let plane = Hyperplane4D::new(4.0, 4, 2.0, 0.01);

        assert_eq!(plane.half_size(), 4.0);
        assert_eq!(plane.grid_size(), 4);
        assert_eq!(plane.w_extent(), 2.0);
        assert_eq!(plane.cell_count(), 16);
    }

    #[test]
    fn test_hyperplane_implements_convex_shape() {
        let plane = Hyperplane4D::new(4.0, 2, 2.0, 0.01);

        assert_eq!(plane.vertex_count(), 4 * 16); // 4 cells * 16 verts
        assert!(plane.tetrahedron_count() > 0);
    }

    #[test]
    fn test_hyperplane_clone() {
        let p1 = Hyperplane4D::new(4.0, 2, 2.0, 0.01);
        let p2 = p1.clone();

        assert_eq!(p1.vertices().len(), p2.vertices().len());
        assert_eq!(p1.tetrahedra().len(), p2.tetrahedra().len());
    }
}
