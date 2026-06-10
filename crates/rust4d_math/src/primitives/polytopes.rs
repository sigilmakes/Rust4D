//! The regular convex 4-polytopes (boundary tetrahedralizations)
//!
//! Four-dimensional space has **six** regular convex polytopes — one more
//! than 3D's five Platonic solids. This module constructs four of them
//! (the tesseract lives in [`crate::tesseract`]; the 120-cell's dodecahedral
//! cells make it a poor fit for real-time slicing and it is omitted):
//!
//! | Function | Polytope | Cells | Our tetrahedra |
//! |----------|----------|-------|----------------|
//! | [`pentachoron`] | 5-cell (4-simplex) | 5 tetrahedra | 5 |
//! | [`hexadecachoron`] | 16-cell (4-orthoplex) | 16 tetrahedra | 16 |
//! | [`icositetrachoron`] | 24-cell | 24 octahedra | 96 (4 per cell) |
//! | [`hexacosichoron`] | 600-cell | 600 tetrahedra | 600 |
//!
//! All are centered at the origin and parameterized by **circumradius**
//! (every vertex lies at that distance from the center), which is the
//! natural size measure for objects you slice: it bounds how far the
//! cross-section can reach.
//!
//! Each construction is pinned by tests on cell count, watertightness
//! (every triangular face shared by exactly two cells), and total boundary
//! 3-volume against the closed-form value.

use crate::{Mesh4D, Tetrahedron, Vec4};

/// The golden ratio φ = (1 + √5)/2, the structural constant of the 600-cell.
const PHI: f64 = 1.618_033_988_749_895;

/// Regular **5-cell** (4-simplex, pentachoron): 5 vertices, 5 tetrahedral
/// cells — the 4D analogue of the tetrahedron, and the simplest possible
/// closed 4D solid.
///
/// Its boundary is every 4-subset of the 5 vertices. Vertices are placed at
/// a standard regular-simplex configuration and scaled to `circumradius`.
pub fn pentachoron(circumradius: f32) -> Mesh4D {
    // Four vertices of a regular tetrahedron at w = -1/√5, apex at w = 4/√5:
    // all pairwise distances 2√2, centroid at the origin, circumradius √(1+1+1+1/5) = ... uniform.
    let s5 = 5.0f64.sqrt();
    let raw = [
        [1.0, 1.0, 1.0, -1.0 / s5],
        [1.0, -1.0, -1.0, -1.0 / s5],
        [-1.0, 1.0, -1.0, -1.0 / s5],
        [-1.0, -1.0, 1.0, -1.0 / s5],
        [0.0, 0.0, 0.0, 4.0 / s5],
    ];
    // |v0| = sqrt(3 + 1/5) = sqrt(16/5) = 4/√5 — same for all five.
    let scale = circumradius as f64 / (4.0 / s5);
    let vertices: Vec<Vec4> = raw
        .iter()
        .map(|v| {
            Vec4::new(
                (v[0] * scale) as f32,
                (v[1] * scale) as f32,
                (v[2] * scale) as f32,
                (v[3] * scale) as f32,
            )
        })
        .collect();

    let tetrahedra = (0..5)
        .map(|skip| {
            let mut idx = [0usize; 4];
            let mut k = 0;
            for i in 0..5 {
                if i != skip {
                    idx[k] = i;
                    k += 1;
                }
            }
            Tetrahedron::new(idx)
        })
        .collect();

    Mesh4D::from_parts(vertices, tetrahedra)
}

/// Regular **16-cell** (4-orthoplex, hexadecachoron): 8 vertices at
/// `±circumradius` along each axis, 16 tetrahedral cells — the 4D analogue
/// of the octahedron, and the dual of the tesseract.
///
/// Each cell pairs one vertex from each axis: cell *(s₀,s₁,s₂,s₃)* =
/// *(s₀r·e₀, s₁r·e₁, s₂r·e₂, s₃r·e₃)* for the 16 sign combinations. This is
/// also the base mesh the [hypersphere](crate::primitives::hypersphere)
/// refines.
pub fn hexadecachoron(circumradius: f32) -> Mesh4D {
    let r = circumradius;
    let mut vertices = Vec::with_capacity(8);
    for axis in 0..4 {
        for sign in [1.0f32, -1.0] {
            let mut v = [0.0f32; 4];
            v[axis] = sign * r;
            vertices.push(Vec4::new(v[0], v[1], v[2], v[3]));
        }
    }
    // vertices[2*axis] = +e_axis, vertices[2*axis + 1] = -e_axis
    let mut tetrahedra = Vec::with_capacity(16);
    for signs in 0..16u32 {
        let idx = [
            (signs & 1) as usize,
            2 + ((signs >> 1) & 1) as usize,
            4 + ((signs >> 2) & 1) as usize,
            6 + ((signs >> 3) & 1) as usize,
        ];
        tetrahedra.push(Tetrahedron::new(idx));
    }
    Mesh4D::from_parts(vertices, tetrahedra)
}

/// Regular **24-cell** (icositetrachoron): 24 vertices, 24 octahedral cells.
///
/// The 24-cell is the one regular polytope with **no analogue in any other
/// dimension** — 4D's special snowflake. It is self-dual, tiles 4D space,
/// and its vertices are the unit Hurwitz quaternions.
///
/// Vertices: all permutations of *(±1, ±1, 0, 0)*, scaled to `circumradius`
/// (natural circumradius √2). Each octahedral cell is found as the set of
/// six vertices extremal along a cell-center direction (the 24 directions of
/// the dual 24-cell), then split into 4 tetrahedra around a main diagonal —
/// 96 tetrahedra total.
pub fn icositetrachoron(circumradius: f32) -> Mesh4D {
    let scale = circumradius / 2.0f32.sqrt();

    // 24 vertices: permutations of (±1, ±1, 0, 0).
    let mut vertices: Vec<Vec4> = Vec::with_capacity(24);
    for i in 0..3 {
        for j in (i + 1)..4 {
            for si in [1.0f32, -1.0] {
                for sj in [1.0f32, -1.0] {
                    let mut v = [0.0f32; 4];
                    v[i] = si;
                    v[j] = sj;
                    vertices.push(Vec4::new(v[0], v[1], v[2], v[3]) * scale);
                }
            }
        }
    }

    // 24 cell-center directions: the dual 24-cell's vertices.
    let mut directions: Vec<Vec4> = Vec::with_capacity(24);
    for axis in 0..4 {
        for sign in [1.0f32, -1.0] {
            let mut v = [0.0f32; 4];
            v[axis] = sign;
            directions.push(Vec4::new(v[0], v[1], v[2], v[3]));
        }
    }
    for bits in 0..16u32 {
        let s = |b: u32| if (bits >> b) & 1 == 0 { 0.5f32 } else { -0.5 };
        directions.push(Vec4::new(s(0), s(1), s(2), s(3)));
    }

    let mut tetrahedra = Vec::with_capacity(96);
    for dir in &directions {
        // The 6 vertices extremal along `dir` form one octahedral cell.
        let mut scored: Vec<(usize, f32)> = vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (i, v.dot(*dir)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let cell: Vec<usize> = scored[..6].iter().map(|(i, _)| *i).collect();
        debug_assert!(
            scored[5].1 - scored[6].1 > 1e-4,
            "cell selection must be unambiguous"
        );

        // Cell centroid; opposite vertex pairs satisfy a + b = 2c.
        let centroid = cell
            .iter()
            .fold(Vec4::ZERO, |acc, &i| acc + vertices[i])
            * (1.0 / 6.0);

        let antipode = |a: usize| -> usize {
            *cell
                .iter()
                .find(|&&b| {
                    b != a && (vertices[a] + vertices[b] - centroid * 2.0).length() < 1e-4 * circumradius.max(1.0)
                })
                .expect("octahedral cell vertex must have an antipode")
        };

        // Axis pair (a, a'), equator cycle e0 → e1 → e0' → e1'.
        let a = cell[0];
        let a2 = antipode(a);
        let equator: Vec<usize> = cell.iter().copied().filter(|&v| v != a && v != a2).collect();
        let e0 = equator[0];
        let e0p = antipode(e0);
        let e1 = *equator.iter().find(|&&v| v != e0 && v != e0p).unwrap();
        let e1p = antipode(e1);

        for (p, q) in [(e0, e1), (e1, e0p), (e0p, e1p), (e1p, e0)] {
            tetrahedra.push(Tetrahedron::new([a, a2, p, q]));
        }
    }

    Mesh4D::from_parts(vertices, tetrahedra)
}

/// Regular **600-cell** (hexacosichoron): 120 vertices, 600 tetrahedral
/// cells — the 4D analogue of the icosahedron and the most intricate object
/// this engine ships.
///
/// The 120 vertices are exactly the elements of the **binary icosahedral
/// group** 2I ⊂ unit quaternions:
///
/// - 8 unit-axis points: permutations of *(±1, 0, 0, 0)*
/// - 16 half-points: *(±½, ±½, ±½, ±½)*
/// - 96 golden points: **even** permutations of *(±φ/2, ±½, ±1/(2φ), 0)*
///
/// Every vertex has exactly 12 nearest neighbors at distance `r/φ` (its
/// vertex figure is an icosahedron), and the 600 cells are precisely the
/// 4-cliques of the nearest-neighbor graph, which we enumerate directly.
/// The cell count doubles as a proof of correctness: only the true edge
/// length yields exactly 600 maximal 4-cliques.
pub fn hexacosichoron(circumradius: f32) -> Mesh4D {
    // --- vertices (f64 for clean adjacency thresholds) ---
    let mut raw: Vec<[f64; 4]> = Vec::with_capacity(120);

    for axis in 0..4 {
        for sign in [1.0f64, -1.0] {
            let mut v = [0.0f64; 4];
            v[axis] = sign;
            raw.push(v);
        }
    }
    for bits in 0..16u32 {
        let s = |b: u32| if (bits >> b) & 1 == 0 { 0.5f64 } else { -0.5 };
        raw.push([s(0), s(1), s(2), s(3)]);
    }
    // Even permutations of (φ/2, 1/2, 1/(2φ), 0) with independent signs on
    // the three nonzero entries.
    let base = [PHI / 2.0, 0.5, 1.0 / (2.0 * PHI), 0.0];
    for perm in EVEN_PERMUTATIONS_4 {
        for bits in 0..8u32 {
            let mut v = [0.0f64; 4];
            let mut sign_slot = 0;
            for (dst, &src) in perm.iter().enumerate() {
                let mag = base[src];
                if mag != 0.0 {
                    let s = if (bits >> sign_slot) & 1 == 0 { 1.0 } else { -1.0 };
                    v[dst] = s * mag;
                    sign_slot += 1;
                } else {
                    v[dst] = 0.0;
                }
            }
            raw.push(v);
        }
    }
    debug_assert_eq!(raw.len(), 120);

    // --- adjacency: edge length is exactly 1/φ for unit circumradius ---
    let edge = 1.0 / PHI;
    let edge2_lo = (edge * edge) * 0.999;
    let edge2_hi = (edge * edge) * 1.001;
    let dist2 = |a: &[f64; 4], b: &[f64; 4]| -> f64 {
        (0..4).map(|k| (a[k] - b[k]) * (a[k] - b[k])).sum()
    };

    let n = raw.len();
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::with_capacity(12); n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d2 = dist2(&raw[i], &raw[j]);
            if d2 > edge2_lo && d2 < edge2_hi {
                neighbors[i].push(j);
                neighbors[j].push(i);
            }
        }
    }

    // --- cells: 4-cliques of the edge graph, enumerated with i<j<k<l ---
    let mut tetrahedra: Vec<Tetrahedron> = Vec::with_capacity(600);
    for i in 0..n {
        let ni = &neighbors[i];
        for (a_pos, &j) in ni.iter().enumerate() {
            if j < i {
                continue;
            }
            for &k in &ni[(a_pos + 1)..] {
                if k < j || !neighbors[j].contains(&k) {
                    continue;
                }
                for &l in ni {
                    if l > k && neighbors[j].contains(&l) && neighbors[k].contains(&l) {
                        tetrahedra.push(Tetrahedron::new([i, j, k, l]));
                    }
                }
            }
        }
    }

    let vertices = raw
        .iter()
        .map(|v| {
            Vec4::new(
                (v[0] * circumradius as f64) as f32,
                (v[1] * circumradius as f64) as f32,
                (v[2] * circumradius as f64) as f32,
                (v[3] * circumradius as f64) as f32,
            )
        })
        .collect();

    Mesh4D::from_parts(vertices, tetrahedra)
}

/// The 12 even permutations of `[0, 1, 2, 3]`, used to generate the
/// 600-cell's 96 golden vertices (odd permutations would produce the wrong
/// chirality and break the group structure).
const EVEN_PERMUTATIONS_4: [[usize; 4]; 12] = [
    [0, 1, 2, 3],
    [0, 2, 3, 1],
    [0, 3, 1, 2],
    [1, 0, 3, 2],
    [1, 2, 0, 3],
    [1, 3, 2, 0],
    [2, 0, 1, 3],
    [2, 1, 3, 0],
    [2, 3, 0, 1],
    [3, 0, 2, 1],
    [3, 1, 0, 2],
    [3, 2, 1, 0],
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConvexShape4D;

    /// Volume of a regular tetrahedron with edge `a`: a³ / (6√2).
    fn regular_tet_volume(a: f64) -> f64 {
        a.powi(3) / (6.0 * 2.0f64.sqrt())
    }

    #[test]
    fn test_pentachoron_structure() {
        let m = pentachoron(1.0);
        m.validate().unwrap();
        assert_eq!(m.vertex_count(), 5);
        assert_eq!(m.tetrahedron_count(), 5);
        assert!(m.is_watertight());
        // All vertices on the circumsphere
        for v in m.vertices() {
            assert!((v.length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_pentachoron_volume() {
        // Circumradius 1 → edge 2√2 · (√5/4) = √(5/2) · ... measure edge
        // directly and compare against 5 regular tets.
        let m = pentachoron(2.0);
        let edge = (m.vertices()[0] - m.vertices()[1]).length() as f64;
        let expected = 5.0 * regular_tet_volume(edge);
        assert!(
            ((m.surface_volume() as f64) - expected).abs() / expected < 1e-4,
            "5-cell boundary volume {} != {expected}",
            m.surface_volume()
        );
    }

    #[test]
    fn test_hexadecachoron_structure() {
        let m = hexadecachoron(1.5);
        m.validate().unwrap();
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.tetrahedron_count(), 16);
        assert!(m.is_watertight());
        for v in m.vertices() {
            assert!((v.length() - 1.5).abs() < 1e-5);
        }
    }

    #[test]
    fn test_hexadecachoron_volume() {
        // Closed form: 16 r³ / 3 for circumradius r.
        let r = 1.5f64;
        let m = hexadecachoron(r as f32);
        let expected = 16.0 * r.powi(3) / 3.0;
        assert!(
            ((m.surface_volume() as f64) - expected).abs() / expected < 1e-4,
            "16-cell boundary volume {} != {expected}",
            m.surface_volume()
        );
    }

    #[test]
    fn test_icositetrachoron_structure() {
        let m = icositetrachoron(2.0f32.sqrt());
        m.validate().unwrap();
        assert_eq!(m.vertex_count(), 24);
        assert_eq!(m.tetrahedron_count(), 96);
        assert!(m.is_watertight(), "24-cell boundary must be watertight");
        for v in m.vertices() {
            assert!((v.length() - 2.0f32.sqrt()).abs() < 1e-5);
        }
    }

    #[test]
    fn test_icositetrachoron_volume() {
        // 24 octahedra of edge a = r (for circumradius r = √2·scale, the
        // cell edge equals the scaled edge √2·s where s = r/√2 → a = r/√2·√2
        // ... measure the edge directly instead of deriving it.)
        let m = icositetrachoron(2.0f32.sqrt());
        // Octahedron volume: √2/3 · a³. Cell edge: distance between two
        // vertices sharing a cell — e.g. (1,1,0,0) and (1,0,1,0): √2.
        let a = 2.0f64.sqrt();
        let expected = 24.0 * (2.0f64.sqrt() / 3.0) * a.powi(3);
        assert!(
            ((m.surface_volume() as f64) - expected).abs() / expected < 1e-3,
            "24-cell boundary volume {} != {expected}",
            m.surface_volume()
        );
    }

    #[test]
    fn test_hexacosichoron_structure() {
        let m = hexacosichoron(1.0);
        m.validate().unwrap();
        assert_eq!(m.vertex_count(), 120, "binary icosahedral group has order 120");
        assert_eq!(m.tetrahedron_count(), 600, "the 600-cell must have 600 cells");
        assert!(m.is_watertight(), "600-cell boundary must be watertight");
        for v in m.vertices() {
            assert!((v.length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_hexacosichoron_volume() {
        // 600 regular tetrahedra of edge 1/φ (circumradius 1).
        let m = hexacosichoron(1.0);
        let expected = 600.0 * regular_tet_volume(1.0 / PHI);
        assert!(
            ((m.surface_volume() as f64) - expected).abs() / expected < 1e-3,
            "600-cell boundary volume {} != {expected}",
            m.surface_volume()
        );
    }

    #[test]
    fn test_hexacosichoron_vertex_figure() {
        // Every vertex of the 600-cell has exactly 20 incident cells and
        // 12 neighbors (its vertex figure is an icosahedron).
        let m = hexacosichoron(1.0);
        let mut incident = vec![0usize; m.vertex_count()];
        for t in m.tetrahedra() {
            for &i in &t.indices {
                incident[i] += 1;
            }
        }
        assert!(incident.iter().all(|&c| c == 20), "each vertex must touch 20 cells");
    }

    #[test]
    fn test_polytopes_scale_with_circumradius() {
        for ctor in [pentachoron, hexadecachoron, icositetrachoron, hexacosichoron] {
            let small = ctor(1.0);
            let big = ctor(2.0);
            // Boundary 3-volume scales with r³
            let ratio = big.surface_volume() / small.surface_volume();
            assert!((ratio - 8.0).abs() < 1e-2, "volume must scale as r³, got {ratio}");
        }
    }
}
