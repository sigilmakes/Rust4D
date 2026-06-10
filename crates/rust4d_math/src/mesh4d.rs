//! General-purpose 4D tetrahedral mesh
//!
//! [`Mesh4D`] is the engine's universal geometry container: a list of 4D
//! vertices plus a list of [`Tetrahedron`] cells indexing into them. All
//! procedural primitives (see [`crate::primitives`]) produce a `Mesh4D`,
//! and anything that implements [`ConvexShape4D`] can be converted into one.
//!
//! # Why tetrahedra?
//!
//! The renderer visualizes 4D objects by slicing their **boundary** — a
//! closed 3-manifold embedded in 4D — with the camera's hyperplane. Just as
//! 3D renderers triangulate surfaces, Rust4D *tetrahedralizes* boundary
//! volumes. Slicing one tetrahedron with a hyperplane yields 1–2 triangles
//! (see the marching-tetrahedra tables in `rust4d_render`), so a closed
//! tetrahedral boundary mesh slices to a closed triangle surface.
//!
//! # Construction utilities
//!
//! - [`Mesh4D::merge`] — append another mesh (indices re-based automatically)
//! - [`Mesh4D::weld`] — deduplicate vertices within an epsilon (essential
//!   after subdivision, where shared midpoints are generated repeatedly)
//! - [`Mesh4D::transformed`] / [`Mesh4D::translated`] / [`Mesh4D::scaled`] —
//!   bake transforms into vertex data
//! - [`Mesh4D::validate`] — index bounds + degenerate-cell detection
//! - [`Mesh4D::surface_volume`] — total 3-volume of all cells, measured with
//!   Gram determinants (correct for any embedding in 4D); used heavily by
//!   the primitive test suites to pin constructions against closed forms

use crate::{ConvexShape4D, Mat4, Tetrahedron, Vec4, mat4};

/// A general 4D tetrahedral mesh: vertices + tetrahedral cells.
///
/// See the [module documentation](self) for the role this type plays in the
/// engine and the utilities it provides.
#[derive(Clone, Debug, Default)]
pub struct Mesh4D {
    vertices: Vec<Vec4>,
    tetrahedra: Vec<Tetrahedron>,
}

/// Errors reported by [`Mesh4D::validate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// A tetrahedron references a vertex index outside the vertex array.
    IndexOutOfBounds {
        /// Index of the offending tetrahedron in the cell list
        tet: usize,
        /// The out-of-bounds vertex index
        index: usize,
    },
    /// A tetrahedron uses the same vertex index more than once.
    DegenerateCell {
        /// Index of the offending tetrahedron in the cell list
        tet: usize,
    },
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::IndexOutOfBounds { tet, index } => {
                write!(f, "tetrahedron {tet} references out-of-bounds vertex {index}")
            }
            MeshError::DegenerateCell { tet } => {
                write!(f, "tetrahedron {tet} has repeated vertex indices")
            }
        }
    }
}

impl std::error::Error for MeshError {}

impl Mesh4D {
    /// Create an empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mesh from raw parts.
    ///
    /// Use [`Mesh4D::validate`] afterwards if the data comes from an
    /// untrusted source (e.g. a file).
    pub fn from_parts(vertices: Vec<Vec4>, tetrahedra: Vec<Tetrahedron>) -> Self {
        Self { vertices, tetrahedra }
    }

    /// Create an empty mesh with pre-allocated capacity.
    pub fn with_capacity(vertices: usize, tetrahedra: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertices),
            tetrahedra: Vec::with_capacity(tetrahedra),
        }
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of tetrahedral cells.
    pub fn tetrahedron_count(&self) -> usize {
        self.tetrahedra.len()
    }

    /// Append a vertex, returning its index.
    pub fn push_vertex(&mut self, v: Vec4) -> usize {
        self.vertices.push(v);
        self.vertices.len() - 1
    }

    /// Append a tetrahedral cell from four vertex indices.
    pub fn push_tetrahedron(&mut self, indices: [usize; 4]) {
        self.tetrahedra.push(Tetrahedron::new(indices));
    }

    /// Append all geometry from `other`, re-basing its indices.
    pub fn merge(&mut self, other: &Mesh4D) {
        let base = self.vertices.len();
        self.vertices.extend_from_slice(&other.vertices);
        self.tetrahedra.extend(other.tetrahedra.iter().map(|t| {
            Tetrahedron::new([
                t.indices[0] + base,
                t.indices[1] + base,
                t.indices[2] + base,
                t.indices[3] + base,
            ])
        }));
    }

    /// Return a copy with every vertex transformed by `rotation` (a `Mat4`)
    /// then offset by `translation`.
    pub fn transformed(&self, rotation: &Mat4, translation: Vec4) -> Self {
        Self {
            vertices: self
                .vertices
                .iter()
                .map(|v| mat4::transform(*rotation, *v) + translation)
                .collect(),
            tetrahedra: self.tetrahedra.clone(),
        }
    }

    /// Return a copy with every vertex offset by `delta`.
    pub fn translated(&self, delta: Vec4) -> Self {
        Self {
            vertices: self.vertices.iter().map(|v| *v + delta).collect(),
            tetrahedra: self.tetrahedra.clone(),
        }
    }

    /// Return a copy with every vertex scaled uniformly about the origin.
    pub fn scaled(&self, factor: f32) -> Self {
        Self {
            vertices: self.vertices.iter().map(|v| *v * factor).collect(),
            tetrahedra: self.tetrahedra.clone(),
        }
    }

    /// Deduplicate vertices that lie within `epsilon` of each other,
    /// rewriting cell indices to the surviving vertex.
    ///
    /// Subdivision algorithms generate shared midpoints once per parent
    /// cell; welding restores connectivity so the mesh slices without
    /// cracks and uploads fewer vertices. Uses a quantized spatial hash:
    /// O(n) for meshes without pathological epsilon-chains.
    pub fn weld(&mut self, epsilon: f32) {
        use std::collections::HashMap;

        let inv = 1.0 / epsilon.max(1e-12);
        let quantize = |v: &Vec4| -> (i64, i64, i64, i64) {
            (
                (v.x * inv).round() as i64,
                (v.y * inv).round() as i64,
                (v.z * inv).round() as i64,
                (v.w * inv).round() as i64,
            )
        };

        let mut buckets: HashMap<(i64, i64, i64, i64), Vec<usize>> = HashMap::new();
        let mut remap: Vec<usize> = Vec::with_capacity(self.vertices.len());
        let mut kept: Vec<Vec4> = Vec::with_capacity(self.vertices.len());

        'outer: for v in &self.vertices {
            let key = quantize(v);
            // Check this bucket and all 80 neighbors for an existing match.
            // (Neighbor check handles points straddling a quantization edge.)
            for dx in -1..=1_i64 {
                for dy in -1..=1_i64 {
                    for dz in -1..=1_i64 {
                        for dw in -1..=1_i64 {
                            let k = (key.0 + dx, key.1 + dy, key.2 + dz, key.3 + dw);
                            if let Some(candidates) = buckets.get(&k) {
                                for &ci in candidates {
                                    if (kept[ci] - *v).length() <= epsilon {
                                        remap.push(ci);
                                        continue 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let idx = kept.len();
            kept.push(*v);
            buckets.entry(key).or_default().push(idx);
            remap.push(idx);
        }

        for tet in &mut self.tetrahedra {
            for i in &mut tet.indices {
                *i = remap[*i];
            }
        }
        self.vertices = kept;
    }

    /// Check index bounds and reject cells with repeated vertices.
    pub fn validate(&self) -> Result<(), MeshError> {
        let n = self.vertices.len();
        for (ti, tet) in self.tetrahedra.iter().enumerate() {
            for &i in &tet.indices {
                if i >= n {
                    return Err(MeshError::IndexOutOfBounds { tet: ti, index: i });
                }
            }
            let c = tet.canonical();
            if c[0] == c[1] || c[1] == c[2] || c[2] == c[3] {
                return Err(MeshError::DegenerateCell { tet: ti });
            }
        }
        Ok(())
    }

    /// Axis-aligned bounding box as `(min, max)`.
    ///
    /// Returns `None` for an empty mesh.
    pub fn bounding_box(&self) -> Option<(Vec4, Vec4)> {
        let first = *self.vertices.first()?;
        let mut min = first;
        let mut max = first;
        for v in &self.vertices[1..] {
            min = Vec4::new(min.x.min(v.x), min.y.min(v.y), min.z.min(v.z), min.w.min(v.w));
            max = Vec4::new(max.x.max(v.x), max.y.max(v.y), max.z.max(v.z), max.w.max(v.w));
        }
        Some((min, max))
    }

    /// Radius of the smallest origin-centered ball containing all vertices.
    pub fn bounding_radius(&self) -> f32 {
        self.vertices
            .iter()
            .map(|v| v.length())
            .fold(0.0, f32::max)
    }

    /// 3-volume of a single cell.
    ///
    /// The cell is a tetrahedron embedded in 4D, so the usual
    /// `det/6` formula doesn't apply directly. Instead we use the Gram
    /// determinant of its edge vectors, which measures k-volume in any
    /// embedding dimension:
    ///
    /// ```text
    /// V = sqrt(det(G)) / 3!     where  G[i][j] = e_i · e_j
    /// ```
    pub fn cell_volume(&self, tet: usize) -> f32 {
        let t = &self.tetrahedra[tet];
        let p0 = self.vertices[t.indices[0]];
        let e = [
            self.vertices[t.indices[1]] - p0,
            self.vertices[t.indices[2]] - p0,
            self.vertices[t.indices[3]] - p0,
        ];
        let mut g = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                g[i][j] = e[i].dot(e[j]) as f64;
            }
        }
        let det = g[0][0] * (g[1][1] * g[2][2] - g[1][2] * g[2][1])
            - g[0][1] * (g[1][0] * g[2][2] - g[1][2] * g[2][0])
            + g[0][2] * (g[1][0] * g[2][1] - g[1][1] * g[2][0]);
        (det.max(0.0).sqrt() / 6.0) as f32
    }

    /// Count triangular faces by occurrence: returns `(paired, unpaired)`.
    ///
    /// Each tetrahedron has four triangular faces. In a mesh representing a
    /// **closed** boundary 3-manifold (the boundary of a 4D solid), every
    /// face must be shared by exactly two cells. Faces are compared by
    /// canonical (sorted) vertex indices, so run [`Mesh4D::weld`] first if
    /// the mesh was built from unshared vertices.
    pub fn face_pairing(&self) -> (usize, usize) {
        use std::collections::HashMap;
        let mut counts: HashMap<[usize; 3], usize> = HashMap::new();
        for tet in &self.tetrahedra {
            let c = tet.canonical();
            for skip in 0..4 {
                let mut face = [0usize; 3];
                let mut k = 0;
                for (i, &v) in c.iter().enumerate() {
                    if i != skip {
                        face[k] = v;
                        k += 1;
                    }
                }
                *counts.entry(face).or_insert(0) += 1;
            }
        }
        let mut paired = 0;
        let mut unpaired = 0;
        for &n in counts.values() {
            if n == 2 {
                paired += 1;
            } else {
                unpaired += 1;
            }
        }
        (paired, unpaired)
    }

    /// True if every triangular face is shared by exactly two cells — i.e.
    /// the mesh is a closed 3-manifold with no cracks, gaps, T-junctions,
    /// or duplicated cells.
    ///
    /// This is the strongest cheap structural test we have; every primitive
    /// in [`crate::primitives`] is pinned watertight by its test suite.
    pub fn is_watertight(&self) -> bool {
        !self.tetrahedra.is_empty() && self.face_pairing().1 == 0
    }

    /// Total 3-volume of all cells — the "surface volume" of a boundary
    /// mesh, analogous to surface *area* in 3D.
    ///
    /// Primitive tests compare this against closed-form values to prove
    /// constructions cover the boundary exactly once (no gaps, no overlaps).
    pub fn surface_volume(&self) -> f32 {
        (0..self.tetrahedra.len())
            .map(|i| self.cell_volume(i) as f64)
            .sum::<f64>() as f32
    }
}

impl ConvexShape4D for Mesh4D {
    fn vertices(&self) -> &[Vec4] {
        &self.vertices
    }

    fn tetrahedra(&self) -> &[Tetrahedron] {
        &self.tetrahedra
    }
}

impl<S: ConvexShape4D + ?Sized> From<&S> for Mesh4D {
    fn from(shape: &S) -> Self {
        Self {
            vertices: shape.vertices().to_vec(),
            tetrahedra: shape.tetrahedra().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_tet() -> Mesh4D {
        Mesh4D::from_parts(
            vec![
                Vec4::new(0.0, 0.0, 0.0, 0.0),
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 1.0, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 1.0, 0.0),
            ],
            vec![Tetrahedron::new([0, 1, 2, 3])],
        )
    }

    #[test]
    fn test_cell_volume_unit_tet() {
        // Right-corner tetrahedron with unit legs: V = 1/6
        let m = unit_tet();
        assert!((m.cell_volume(0) - 1.0 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_cell_volume_invariant_under_rotation_into_w() {
        // Gram-determinant volume must not change when the cell is rotated
        // out of the XYZ subspace into W.
        let m = unit_tet();
        let rot = crate::mat4::plane_rotation(0.7, 2, 3); // ZW rotation
        let r = m.transformed(&rot, Vec4::new(5.0, -3.0, 2.0, 1.0));
        assert!((r.cell_volume(0) - 1.0 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_merge_rebases_indices() {
        let mut a = unit_tet();
        let b = unit_tet().translated(Vec4::new(10.0, 0.0, 0.0, 0.0));
        a.merge(&b);
        assert_eq!(a.vertex_count(), 8);
        assert_eq!(a.tetrahedron_count(), 2);
        assert_eq!(a.tetrahedra()[1].indices, [4, 5, 6, 7]);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn test_weld_merges_coincident_vertices() {
        let mut a = unit_tet();
        let b = unit_tet(); // exact duplicate
        a.merge(&b);
        assert_eq!(a.vertex_count(), 8);
        a.weld(1e-5);
        assert_eq!(a.vertex_count(), 4);
        assert!(a.validate().is_ok());
        // Both cells now reference the same 4 vertices
        assert_eq!(a.tetrahedra()[0].canonical(), a.tetrahedra()[1].canonical());
    }

    #[test]
    fn test_weld_keeps_distinct_vertices() {
        let mut m = unit_tet();
        m.weld(1e-5);
        assert_eq!(m.vertex_count(), 4);
    }

    #[test]
    fn test_validate_catches_out_of_bounds() {
        let m = Mesh4D::from_parts(
            vec![Vec4::ZERO],
            vec![Tetrahedron::new([0, 0, 0, 7])],
        );
        assert!(matches!(
            m.validate(),
            Err(MeshError::IndexOutOfBounds { index: 7, .. })
        ));
    }

    #[test]
    fn test_validate_catches_degenerate() {
        let m = Mesh4D::from_parts(
            vec![Vec4::ZERO; 4],
            vec![Tetrahedron::new([0, 1, 2, 2])],
        );
        assert!(matches!(m.validate(), Err(MeshError::DegenerateCell { tet: 0 })));
    }

    #[test]
    fn test_bounding_box_and_radius() {
        let m = unit_tet().translated(Vec4::new(0.0, 0.0, 0.0, 2.0));
        let (min, max) = m.bounding_box().unwrap();
        assert_eq!(min.w, 2.0);
        assert_eq!(max.x, 1.0);
        assert!(m.bounding_radius() >= 2.0);
    }

    #[test]
    fn test_tesseract_boundary_is_watertight() {
        // The tesseract's 8 cubic cells × 6 tets form a closed 3-manifold.
        let tess = crate::Tesseract4D::new(2.0);
        let m: Mesh4D = (&tess as &dyn ConvexShape4D).into();
        assert!(m.is_watertight(), "tesseract boundary should be watertight");
    }

    #[test]
    fn test_single_tet_is_not_watertight() {
        let m = unit_tet();
        assert!(!m.is_watertight());
        let (paired, unpaired) = m.face_pairing();
        assert_eq!((paired, unpaired), (0, 4));
    }

    #[test]
    fn test_from_convex_shape() {
        let tess = crate::Tesseract4D::new(2.0);
        let m: Mesh4D = (&tess as &dyn ConvexShape4D).into();
        assert_eq!(m.vertex_count(), 16);
        assert!(m.validate().is_ok());
    }
}
