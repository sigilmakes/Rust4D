//! Tesseract (4D Hypercube) geometry
//!
//! A tesseract has 16 vertices (all combinations of ±h for x,y,z,w),
//! 32 edges, 24 faces (squares), and 8 cells (cubes).
//!
//! For cross-section rendering, we decompose it into tetrahedra (3-simplices).

use crate::{
    shape::{ConvexShape4D, Tetrahedron},
    Vec4,
};
use std::collections::HashSet;

/// A tesseract (4D hypercube) - pure geometry without colors
#[derive(Clone)]
pub struct Tesseract4D {
    /// Half the side length
    half_size: f32,
    /// The 16 vertices of the tesseract
    vertices: [Vec4; 16],
    /// Tetrahedra decomposition
    tetrahedra: Vec<Tetrahedron>,
}

impl Tesseract4D {
    /// Create a new tesseract centered at origin with given size
    ///
    /// # Arguments
    /// * `size` - The full side length of the tesseract
    pub fn new(size: f32) -> Self {
        let h = size * 0.5;

        // All 16 vertices are combinations of ±h for each coordinate
        // Using binary counting: vertex i has coordinates based on bits of i
        let vertices = [
            Vec4::new(-h, -h, -h, -h), // 0  = 0b0000
            Vec4::new(h, -h, -h, -h),  // 1  = 0b0001
            Vec4::new(-h, h, -h, -h),  // 2  = 0b0010
            Vec4::new(h, h, -h, -h),   // 3  = 0b0011
            Vec4::new(-h, -h, h, -h),  // 4  = 0b0100
            Vec4::new(h, -h, h, -h),   // 5  = 0b0101
            Vec4::new(-h, h, h, -h),   // 6  = 0b0110
            Vec4::new(h, h, h, -h),    // 7  = 0b0111
            Vec4::new(-h, -h, -h, h),  // 8  = 0b1000
            Vec4::new(h, -h, -h, h),   // 9  = 0b1001
            Vec4::new(-h, h, -h, h),   // 10 = 0b1010
            Vec4::new(h, h, -h, h),    // 11 = 0b1011
            Vec4::new(-h, -h, h, h),   // 12 = 0b1100
            Vec4::new(h, -h, h, h),    // 13 = 0b1101
            Vec4::new(-h, h, h, h),    // 14 = 0b1110
            Vec4::new(h, h, h, h),     // 15 = 0b1111
        ];

        // Compute tetrahedra decomposition using Kuhn triangulation
        let tetrahedra = Self::compute_tetrahedra();

        Self {
            half_size: h,
            vertices,
            tetrahedra,
        }
    }

    /// Get the half-size (half the side length)
    #[inline]
    pub fn half_size(&self) -> f32 {
        self.half_size
    }

    /// Get the full size (side length)
    #[inline]
    pub fn size(&self) -> f32 {
        self.half_size * 2.0
    }

    /// Get the vertices of a specific tetrahedron
    pub fn get_tetrahedron_vertices(&self, tet_idx: usize) -> [Vec4; 4] {
        let indices = self.tetrahedra[tet_idx].indices;
        [
            self.vertices[indices[0]],
            self.vertices[indices[1]],
            self.vertices[indices[2]],
            self.vertices[indices[3]],
        ]
    }

    /// Compute the boundary tetrahedra using Kuhn triangulation
    ///
    /// The Kuhn triangulation decomposes the solid hypercube into 24 5-cells
    /// (one per permutation of the four axes). Taking every tetrahedral face
    /// of every 5-cell yields 84 distinct tetrahedra — but only the ones
    /// lying on the tesseract's **boundary** (its eight cubic facets) matter
    /// for slicing; the rest are internal membranes that waste GPU work and
    /// would render as spurious interior walls if the camera entered the
    /// shape.
    ///
    /// A tetrahedron lies on a facet exactly when its four vertices agree on
    /// one coordinate — with bitmask vertex indices, when all four indices
    /// share a bit value. Filtering leaves the 48 boundary tetrahedra
    /// (8 cubic facets × 6 Kuhn tetrahedra), and because each facet's
    /// triangulation is induced by the same solid Kuhn triangulation,
    /// adjacent facets agree on shared face diagonals: the result is a
    /// closed, watertight 3-manifold (pinned by `Mesh4D::is_watertight`
    /// tests).
    fn compute_tetrahedra() -> Vec<Tetrahedron> {
        // Generate all permutations of [0, 1, 2, 3] for Kuhn triangulation
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

        // Generate 5-cells from permutations
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

        // Decompose 5-cells into tetrahedra (deduplicated)
        let mut seen: HashSet<[usize; 4]> = HashSet::new();
        let mut tetrahedra = Vec::new();

        for simplex in &simplices {
            // A 5-cell with vertices {v0,v1,v2,v3,v4} decomposes into 5 tetrahedra
            // by omitting each vertex in turn
            for omit in 0..5 {
                let mut tet_verts = [0usize; 4];
                let mut idx = 0;
                for (i, &vert) in simplex.iter().enumerate() {
                    if i != omit {
                        tet_verts[idx] = vert;
                        idx += 1;
                    }
                }

                // Sort for canonical form (deduplication)
                let mut canonical = tet_verts;
                canonical.sort();

                if seen.insert(canonical) {
                    // Keep only boundary tetrahedra: all four vertex-index
                    // bitmasks agree on at least one axis bit.
                    let all_and = tet_verts.iter().fold(0b1111, |acc, &v| acc & v);
                    let all_or = tet_verts.iter().fold(0, |acc, &v| acc | v);
                    let on_boundary = all_and != 0 || all_or != 0b1111;
                    if on_boundary {
                        tetrahedra.push(Tetrahedron::new(tet_verts));
                    }
                }
            }
        }

        tetrahedra
    }
}

impl ConvexShape4D for Tesseract4D {
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
    fn test_tesseract_vertex_count() {
        let t = Tesseract4D::new(2.0);
        assert_eq!(t.vertices().len(), 16);
    }

    #[test]
    fn test_tesseract_tetrahedron_count() {
        let t = Tesseract4D::new(2.0);
        // Exactly the boundary: 8 cubic facets × 6 Kuhn tetrahedra.
        assert_eq!(t.tetrahedra().len(), 48);
    }

    #[test]
    fn test_tesseract_tetrahedra_lie_on_facets() {
        // Every tetrahedron's four vertices must share a fixed coordinate
        // (±h on some axis) — i.e. lie on one of the 8 cubic facets.
        let t = Tesseract4D::new(2.0);
        for tet in t.tetrahedra() {
            let verts = tet.indices.map(|i| t.vertices()[i]);
            let on_facet = (0..4).any(|axis| {
                let c = |v: &Vec4| match axis {
                    0 => v.x,
                    1 => v.y,
                    2 => v.z,
                    _ => v.w,
                };
                verts.iter().all(|v| c(v) == c(&verts[0]))
            });
            assert!(on_facet, "tet {:?} is not on a facet", tet.indices);
        }
    }

    #[test]
    fn test_tesseract_boundary_volume() {
        // Boundary 3-volume of a tesseract with side s: 8 cubic cells of
        // volume s³.
        use crate::Mesh4D;
        let t = Tesseract4D::new(2.0);
        let m: Mesh4D = (&t as &dyn crate::ConvexShape4D).into();
        assert!((m.surface_volume() - 8.0 * 8.0).abs() < 1e-4);
    }

    #[test]
    fn test_tesseract_vertices_positions() {
        let t = Tesseract4D::new(2.0);
        let h = 1.0;

        // Check corner vertices
        assert_eq!(t.vertices[0].x, -h);
        assert_eq!(t.vertices[0].y, -h);
        assert_eq!(t.vertices[0].z, -h);
        assert_eq!(t.vertices[0].w, -h);

        assert_eq!(t.vertices[15].x, h);
        assert_eq!(t.vertices[15].y, h);
        assert_eq!(t.vertices[15].z, h);
        assert_eq!(t.vertices[15].w, h);
    }

    #[test]
    fn test_tesseract_size() {
        let t = Tesseract4D::new(4.0);
        assert_eq!(t.size(), 4.0);
        assert_eq!(t.half_size(), 2.0);
    }

    #[test]
    fn test_tetrahedra_have_four_vertices() {
        let t = Tesseract4D::new(2.0);
        for tet in t.tetrahedra() {
            assert_eq!(tet.indices.len(), 4);
            for &idx in &tet.indices {
                assert!(idx < 16, "Vertex index {} out of range", idx);
            }
        }
    }

    #[test]
    fn test_tetrahedra_cover_tesseract_edges() {
        // All 32 tesseract edges should appear in at least one tetrahedron
        let t = Tesseract4D::new(2.0);

        // Collect all edges from tetrahedra
        let mut tet_edges: HashSet<(usize, usize)> = HashSet::new();
        for tet in t.tetrahedra() {
            for i in 0..4 {
                for j in (i + 1)..4 {
                    let (v0, v1) = if tet.indices[i] < tet.indices[j] {
                        (tet.indices[i], tet.indices[j])
                    } else {
                        (tet.indices[j], tet.indices[i])
                    };
                    tet_edges.insert((v0, v1));
                }
            }
        }

        // Check that all tesseract edges are covered
        for i in 0usize..16 {
            for j in (i + 1)..16 {
                if (i ^ j).count_ones() == 1 {
                    // This is a tesseract edge
                    assert!(
                        tet_edges.contains(&(i, j)),
                        "Tesseract edge ({}, {}) not in any tetrahedron",
                        i,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn test_tesseract_implements_convex_shape() {
        let t = Tesseract4D::new(2.0);

        // Test through trait methods
        assert_eq!(t.vertex_count(), 16);
        assert!(t.tetrahedron_count() > 0);
    }

    #[test]
    fn test_tesseract_clone() {
        let t1 = Tesseract4D::new(2.0);
        let t2 = t1.clone();

        assert_eq!(t1.vertices().len(), t2.vertices().len());
        assert_eq!(t1.tetrahedra().len(), t2.tetrahedra().len());
    }
}
