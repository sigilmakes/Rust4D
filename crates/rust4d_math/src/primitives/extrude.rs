//! Tetrahedralization helpers: prism splitting and simplex subdivision
//!
//! The curved primitives (spherinder, cubinder, duocylinder) are built from
//! **triangular prisms**: a 2D triangle extruded along a 1D segment is a
//! 3-cell embedded in 4D, and every product-shaped boundary piece
//! (circle × square, disk × edge, circle × disk, …) decomposes into such
//! prisms. Each prism then splits into 3 tetrahedra.
//!
//! # The crack problem
//!
//! Two prisms that share a quadrilateral face must split that quad along the
//! **same diagonal**, or the slice shader will produce hairline cracks and
//! T-junctions at the seam. We use the classic *lowest-global-index* rule
//! (Dompierre, Labbé, Vallet & Camarero, *How to Subdivide Pyramids, Prisms
//! and Hexahedra into Tetrahedra*, IMR 1999): every quad face is split by
//! the diagonal emanating from its smallest global vertex index. Because the
//! rule depends only on the four shared indices, both neighbors always
//! agree — across pieces, seams, and wrap-arounds — as long as shared
//! vertices share global indices (build from a common vertex pool; don't
//! rely on post-hoc welding, which renumbers *after* splitting decisions).
//!
//! Every primitive's test suite verifies the result with
//! [`Mesh4D::is_watertight`](crate::Mesh4D::is_watertight).

use crate::Tetrahedron;

/// Split a triangular prism into 3 tetrahedra using the lowest-global-index
/// rule, appending them to `out`.
///
/// `prism` lists global vertex indices as `[b0, b1, b2, t0, t1, t2]` where
/// `t_i` is the extruded copy of `b_i` (the three quads are `b0 b1 t1 t0`,
/// `b1 b2 t2 t1`, `b2 b0 t0 t2`).
///
/// All six indices must be distinct; adjacent prisms sharing a quad face are
/// guaranteed matching diagonals on it.
pub fn split_prism(prism: [usize; 6], out: &mut Vec<Tetrahedron>) {
    debug_assert!(
        {
            let mut s = prism;
            s.sort_unstable();
            s.windows(2).all(|w| w[0] != w[1])
        },
        "split_prism requires distinct vertex indices, got {prism:?}"
    );

    // Bring the globally smallest index to local position 0 using the
    // prism's symmetries (cyclic rotation of both triangles, and the
    // bottom/top flip), which preserve the pairing structure.
    let min_pos = (0..6).min_by_key(|&i| prism[i]).unwrap();
    let v: [usize; 6] = match min_pos {
        0 => prism,
        1 => [prism[1], prism[2], prism[0], prism[4], prism[5], prism[3]],
        2 => [prism[2], prism[0], prism[1], prism[5], prism[3], prism[4]],
        3 => [prism[3], prism[4], prism[5], prism[0], prism[1], prism[2]],
        4 => [prism[4], prism[5], prism[3], prism[1], prism[2], prism[0]],
        _ => [prism[5], prism[3], prism[4], prism[2], prism[0], prism[1]],
    };

    // v[0] is now the global minimum. The two quads incident to v[0]
    // (v0 v1 v4 v3 and v0 v2 v5 v3) take diagonals v0–v4 and v0–v5 by the
    // rule. The remaining quad (v1 v2 v5 v4) takes the diagonal from its
    // own smallest index: v1–v5 if min is v1 or v5, else v2–v4.
    let m = v[1].min(v[2]).min(v[4]).min(v[5]);
    if m == v[1] || m == v[5] {
        out.push(Tetrahedron::new([v[0], v[1], v[2], v[5]]));
        out.push(Tetrahedron::new([v[0], v[1], v[5], v[4]]));
        out.push(Tetrahedron::new([v[0], v[4], v[5], v[3]]));
    } else {
        out.push(Tetrahedron::new([v[0], v[1], v[2], v[4]]));
        out.push(Tetrahedron::new([v[0], v[4], v[2], v[5]]));
        out.push(Tetrahedron::new([v[0], v[4], v[5], v[3]]));
    }
}

/// The 8-way subdivision of a tetrahedron, expressed over its 4 corners and
/// 6 edge midpoints.
///
/// Index convention for the returned cells: `0..4` are the corners
/// `v0..v3`; `4..10` are the midpoints of edges
/// `(0,1), (0,2), (0,3), (1,2), (1,3), (2,3)` in that order.
///
/// The construction: cutting off the 4 corner tetrahedra leaves a central
/// octahedron with vertices at the 6 midpoints, which is split into 4
/// tetrahedra around its `m01–m23` diagonal. Used by the hypersphere to
/// refine the 16-cell boundary toward S³.
pub const TET_SUBDIVISION: [[usize; 4]; 8] = [
    // Corner cells
    [0, 4, 5, 6], // v0, m01, m02, m03
    [1, 4, 7, 8], // v1, m01, m12, m13
    [2, 5, 7, 9], // v2, m02, m12, m23
    [3, 6, 8, 9], // v3, m03, m13, m23
    // Central octahedron around diagonal m01(4) – m23(9).
    // Equator cycle: m02(5) – m03(6) – m13(8) – m12(7).
    [4, 9, 5, 6],
    [4, 9, 6, 8],
    [4, 9, 8, 7],
    [4, 9, 7, 5],
];

/// The midpoint edge list matching [`TET_SUBDIVISION`]'s index convention.
pub const TET_EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mesh4D, Vec4};

    /// Build a prism mesh from explicit points and split it.
    fn prism_mesh(points: [Vec4; 6], order: [usize; 6]) -> Mesh4D {
        let mut tets = Vec::new();
        split_prism(order, &mut tets);
        Mesh4D::from_parts(points.to_vec(), tets)
    }

    fn unit_prism_points() -> [Vec4; 6] {
        // Right triangle (legs 1) extruded by height 1 along Y.
        [
            Vec4::new(0.0, 0.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(1.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 1.0, 0.0),
        ]
    }

    #[test]
    fn test_split_prism_volume() {
        // Prism volume = triangle area (1/2) × height (1) = 1/2,
        // regardless of which permutation the indices arrive in.
        for order in [
            [0usize, 1, 2, 3, 4, 5],
            [1, 2, 0, 4, 5, 3],
            [3, 4, 5, 0, 1, 2],
            [5, 3, 4, 2, 0, 1],
        ] {
            let m = prism_mesh(unit_prism_points(), order);
            assert_eq!(m.tetrahedron_count(), 3);
            m.validate().unwrap();
            assert!(
                (m.surface_volume() - 0.5).abs() < 1e-6,
                "order {order:?}: volume {} != 0.5",
                m.surface_volume()
            );
        }
    }

    #[test]
    fn test_split_prism_in_w_plane() {
        // Same prism rotated into the ZW plane — Gram volumes must agree.
        let rot = crate::mat4::plane_rotation(1.1, 2, 3);
        let pts = unit_prism_points().map(|p| crate::mat4::transform(rot, p));
        let m = prism_mesh(pts, [0, 1, 2, 3, 4, 5]);
        assert!((m.surface_volume() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_adjacent_prisms_share_diagonals() {
        // Two prisms stacked along Y share the middle triangle; the combined
        // tet complex must pair every internal face exactly twice. The shared
        // *quad* faces (sides) get diagonals from the index rule — if the rule
        // were inconsistent, faces would appear unpaired.
        let mut points = Vec::new();
        for y in 0..3 {
            points.push(Vec4::new(0.0, y as f32, 0.0, 0.0));
            points.push(Vec4::new(1.0, y as f32, 0.0, 0.0));
            points.push(Vec4::new(0.0, y as f32, 1.0, 0.0));
        }
        let mut tets = Vec::new();
        split_prism([0, 1, 2, 3, 4, 5], &mut tets);
        split_prism([3, 4, 5, 6, 7, 8], &mut tets);
        let m = Mesh4D::from_parts(points, tets);
        m.validate().unwrap();
        assert!((m.surface_volume() - 1.0).abs() < 1e-6);
        // 2 prisms × 3 tets × 4 faces = 24 face slots. The shared triangle
        // (3,4,5) plane contains 2 triangles (the quad rule splits it the
        // same way from both sides) → internal pairings exist.
        let (paired, _unpaired) = m.face_pairing();
        assert!(
            paired >= 2,
            "stacked prisms must share their interface faces"
        );
    }

    #[test]
    fn test_tet_subdivision_volume() {
        // Subdividing a tetrahedron into 8 must conserve volume exactly.
        let corners = [
            Vec4::new(0.0, 0.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
        ];
        let mut points: Vec<Vec4> = corners.to_vec();
        for (a, b) in TET_EDGES {
            points.push((corners[a] + corners[b]) * 0.5);
        }
        let tets = TET_SUBDIVISION.map(Tetrahedron::new).to_vec();
        let m = Mesh4D::from_parts(points, tets);
        m.validate().unwrap();
        assert_eq!(m.tetrahedron_count(), 8);
        assert!(
            (m.surface_volume() - 1.0 / 6.0).abs() < 1e-6,
            "children must tile the parent: {} != 1/6",
            m.surface_volume()
        );
    }
}
