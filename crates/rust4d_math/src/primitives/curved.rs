//! Curved 4D primitives: hypersphere, spherinder, cubinder, duocylinder
//!
//! Unlike the [polytopes](super::polytopes), these shapes approximate curved
//! boundaries, so each takes resolution parameters. They are the 4D
//! analogues of the sphere and cylinder family:
//!
//! | Function | Shape | Boundary structure |
//! |----------|-------|--------------------|
//! | [`hypersphere`] | solid 4-ball | S³, refined from the 16-cell |
//! | [`spherinder`] | ball × segment | two 3-ball caps + S² × segment tube |
//! | [`cubinder`] | disk × square | S¹ × square + disk × 4 edges |
//! | [`duocylinder`] | disk × disk | **two interlocked solid-torus pieces** meeting at a Clifford torus |
//!
//! All construction goes through a shared vertex pool keyed by logical
//! coordinates, so adjacent pieces reference identical global indices and
//! the [lowest-index prism rule](super::extrude) produces matching diagonals
//! at every seam — the test suites pin each mesh watertight.

use super::extrude::{split_prism, TET_EDGES, TET_SUBDIVISION};
use crate::{ConvexShape4D, Mesh4D, Tetrahedron, Vec4};
use std::collections::HashMap;

/// Solid 4-ball of the given `radius`: its boundary 3-sphere (S³, the
/// *glome*) tetrahedralized by recursive refinement of the
/// [16-cell](super::polytopes::hexadecachoron)'s 16 boundary cells.
///
/// Each subdivision level splits every tetrahedron into 8 and reprojects
/// vertices onto the sphere, so level *d* yields `16·8^d` cells:
///
/// | `subdivisions` | cells | character |
/// |----------------|-------|-----------|
/// | 0 | 16 | the bare 16-cell |
/// | 1 | 128 | visibly round |
/// | 2 | 1 024 | smooth (default scene quality) |
/// | 3 | 8 192 | hero-object quality |
///
/// `subdivisions` is clamped to 4 (65 536 cells) to keep accidental
/// `u32`-ish blowups out of the GPU buffers. The cross-section of a
/// hypersphere is the engine's "hello world" of 4D intuition: a sphere that
/// grows and shrinks as the slice plane sweeps through it.
pub fn hypersphere(radius: f32, subdivisions: u32) -> Mesh4D {
    let subdivisions = subdivisions.min(4);
    let mut mesh = super::polytopes::hexadecachoron(radius);

    for _ in 0..subdivisions {
        let mut refined =
            Mesh4D::with_capacity(mesh.vertex_count() * 4, mesh.tetrahedron_count() * 8);
        for tet in mesh.tetrahedra() {
            // Gather the 10 points of the subdivision (4 corners + 6 edge
            // midpoints reprojected to the sphere). Midpoints are computed
            // identically from both sides of a shared edge (IEEE addition
            // is commutative), so the final weld restores connectivity.
            let corners = tet.indices.map(|i| mesh.vertices()[i]);
            let mut points = [Vec4::ZERO; 10];
            points[..4].copy_from_slice(&corners);
            for (k, (a, b)) in TET_EDGES.iter().enumerate() {
                let mid = (corners[*a] + corners[*b]) * 0.5;
                points[4 + k] = mid.normalized() * radius;
            }
            let base = refined.vertex_count();
            for p in points {
                refined.push_vertex(p);
            }
            for child in TET_SUBDIVISION {
                refined.push_tetrahedron(child.map(|i| base + i));
            }
        }
        refined.weld(radius * 1e-6);
        mesh = refined;
    }
    mesh
}

/// **Spherinder**: a solid 3-ball of `radius` extruded along W over
/// `[-half_height, +half_height]` — the most literal 4D analogue of the
/// cylinder ("a ball, dragged through the 4th dimension").
///
/// Boundary pieces:
/// - two flat **caps**: solid 3-balls at `w = ±half_height` (icosphere
///   surface fanned to a center vertex),
/// - the **tube**: S² × segment, every icosphere surface triangle extruded
///   into a prism.
///
/// `subdivisions` controls the icosphere refinement (0 → 20 triangles,
/// each level ×4; default scenes use 2 → 320 triangles, 1 600 cells total).
///
/// Sliced face-on it looks like a ball; sliced along W it is a ball that
/// pops into existence, stays *constant-sized* for `2·half_height` of
/// travel, then vanishes — the signature difference from a hypersphere.
pub fn spherinder(radius: f32, half_height: f32, subdivisions: u32) -> Mesh4D {
    let (sphere_verts, sphere_tris) = icosphere(radius, subdivisions.min(5));
    let n = sphere_verts.len();

    let mut mesh = Mesh4D::with_capacity(2 * n + 2, sphere_tris.len() * 5);

    // Vertex layout: [0..n) bottom shell (w = -h), [n..2n) top shell,
    // 2n bottom center, 2n+1 top center.
    for w in [-half_height, half_height] {
        for v in &sphere_verts {
            mesh.push_vertex(Vec4::new(v[0], v[1], v[2], w));
        }
    }
    let bottom_center = mesh.push_vertex(Vec4::new(0.0, 0.0, 0.0, -half_height));
    let top_center = mesh.push_vertex(Vec4::new(0.0, 0.0, 0.0, half_height));

    let mut tets: Vec<Tetrahedron> = Vec::with_capacity(sphere_tris.len() * 5);
    for &[a, b, c] in &sphere_tris {
        // Caps: fan every surface triangle to the cap center.
        tets.push(Tetrahedron::new([bottom_center, a, b, c]));
        tets.push(Tetrahedron::new([top_center, n + a, n + b, n + c]));
        // Tube: prism between the two shells.
        split_prism([a, b, c, n + a, n + b, n + c], &mut tets);
    }

    for t in tets {
        mesh.push_tetrahedron(t.indices);
    }
    mesh
}

/// **Cubinder**: a disk of `radius` (XY plane) × a square of half-extent
/// `half_size` (ZW plane) — the "other" 4D cylinder, with a curved direction
/// pair and a flat direction pair.
///
/// Boundary pieces:
/// - **shell**: S¹ × square (the curved side),
/// - **plates**: disk × each of the square's 4 edges.
///
/// `segments` is the circle resolution (≥ 3, default-quality scenes use 24).
/// Sliced at w = 0 it is a cylinder; rotate it in ZW and the cross-section
/// stays a cylinder of changing height — flat directions don't foreshorten
/// the disk.
pub fn cubinder(radius: f32, half_size: f32, segments: u32) -> Mesh4D {
    let segments = segments.max(3) as usize;
    let mut pool = VertexPool::new();

    // Logical coordinates: ring index 0..segments on the circle (or CENTER),
    // corner index 0..4 on the square (z,w) = (±s, ±s).
    const CENTER: usize = usize::MAX;
    let square = [
        (-half_size, -half_size),
        (half_size, -half_size),
        (half_size, half_size),
        (-half_size, half_size),
    ];
    let circle_point = |i: usize| -> (f32, f32) {
        let theta = (i % segments) as f32 / segments as f32 * std::f32::consts::TAU;
        (radius * theta.cos(), radius * theta.sin())
    };
    let vert = |pool: &mut VertexPool, ring: usize, corner: usize| -> usize {
        let key = (
            if ring == CENTER {
                u32::MAX
            } else {
                (ring % segments) as u32
            },
            corner as u32,
        );
        let (z, w) = square[corner];
        let (x, y) = if ring == CENTER {
            (0.0, 0.0)
        } else {
            circle_point(ring)
        };
        pool.get(key, Vec4::new(x, y, z, w))
    };

    let mut tets: Vec<Tetrahedron> = Vec::new();

    // Shell: S¹ × square. Split the square into triangles (0,1,2) and
    // (0,2,3); each circle segment × triangle is a prism.
    for i in 0..segments {
        for tri in [[0usize, 1, 2], [0, 2, 3]] {
            let bottom = tri.map(|c| vert(&mut pool, i, c));
            let top = tri.map(|c| vert(&mut pool, i + 1, c));
            split_prism(
                [bottom[0], bottom[1], bottom[2], top[0], top[1], top[2]],
                &mut tets,
            );
        }
    }

    // Plates: disk × each square edge. The disk is a fan of `segments`
    // triangles (center, i, i+1); each fanned triangle × edge is a prism.
    for edge in 0..4usize {
        let (c0, c1) = (edge, (edge + 1) % 4);
        for i in 0..segments {
            let bottom = [
                vert(&mut pool, CENTER, c0),
                vert(&mut pool, i, c0),
                vert(&mut pool, i + 1, c0),
            ];
            let top = [
                vert(&mut pool, CENTER, c1),
                vert(&mut pool, i, c1),
                vert(&mut pool, i + 1, c1),
            ];
            split_prism(
                [bottom[0], bottom[1], bottom[2], top[0], top[1], top[2]],
                &mut tets,
            );
        }
    }

    let mut mesh = Mesh4D::from_parts(pool.vertices, Vec::new());
    for t in tets {
        mesh.push_tetrahedron(t.indices);
    }
    mesh
}

/// **Duocylinder**: the product of two disks, D²(`r1`) in XY × D²(`r2`) in
/// ZW — the most alien object in the catalog. Its boundary is exactly two
/// pieces, each a solid torus (S¹ × D²), glued along the **Clifford torus**
/// S¹ × S¹ where they meet. It has no edges and no vertices: just two
/// smooth curved 3-faces.
///
/// `segments1`/`segments2` set the resolution of the two circles (≥ 3).
/// Rolling a duocylinder on its XY ridge while watching the 3D slice is one
/// of the great "feel the 4th dimension" demos.
pub fn duocylinder(r1: f32, r2: f32, segments1: u32, segments2: u32) -> Mesh4D {
    let n1 = segments1.max(3) as usize;
    let n2 = segments2.max(3) as usize;
    let mut pool = VertexPool::new();

    // Logical coordinates (i, j): i on circle 1 (or AXIS1 = its center),
    // j on circle 2 (or AXIS2). The Clifford torus is the (i, j) grid;
    // each solid-torus piece fans toward its own axis circle.
    const AXIS: u32 = u32::MAX;
    let vert =
        |pool: &mut VertexPool, i: usize, j: usize, on_axis_1: bool, on_axis_2: bool| -> usize {
            let ki = if on_axis_1 { AXIS } else { (i % n1) as u32 };
            let kj = if on_axis_2 { AXIS } else { (j % n2) as u32 };
            let (x, y) = if on_axis_1 {
                (0.0, 0.0)
            } else {
                let a = (i % n1) as f32 / n1 as f32 * std::f32::consts::TAU;
                (r1 * a.cos(), r1 * a.sin())
            };
            let (z, w) = if on_axis_2 {
                (0.0, 0.0)
            } else {
                let b = (j % n2) as f32 / n2 as f32 * std::f32::consts::TAU;
                (r2 * b.cos(), r2 * b.sin())
            };
            pool.get((ki, kj), Vec4::new(x, y, z, w))
        };

    let mut tets: Vec<Tetrahedron> = Vec::new();

    // Piece 1: S¹(r1) × D²(r2). Disk 2 fans (axis2, j, j+1); extrude each
    // fan triangle along circle 1.
    for i in 0..n1 {
        for j in 0..n2 {
            let bottom = [
                vert(&mut pool, i, 0, false, true),
                vert(&mut pool, i, j, false, false),
                vert(&mut pool, i, j + 1, false, false),
            ];
            let top = [
                vert(&mut pool, i + 1, 0, false, true),
                vert(&mut pool, i + 1, j, false, false),
                vert(&mut pool, i + 1, j + 1, false, false),
            ];
            split_prism(
                [bottom[0], bottom[1], bottom[2], top[0], top[1], top[2]],
                &mut tets,
            );
        }
    }

    // Piece 2: D²(r1) × S¹(r2) — the mirror construction.
    for j in 0..n2 {
        for i in 0..n1 {
            let bottom = [
                vert(&mut pool, 0, j, true, false),
                vert(&mut pool, i, j, false, false),
                vert(&mut pool, i + 1, j, false, false),
            ];
            let top = [
                vert(&mut pool, 0, j + 1, true, false),
                vert(&mut pool, i, j + 1, false, false),
                vert(&mut pool, i + 1, j + 1, false, false),
            ];
            split_prism(
                [bottom[0], bottom[1], bottom[2], top[0], top[1], top[2]],
                &mut tets,
            );
        }
    }

    let mut mesh = Mesh4D::from_parts(pool.vertices, Vec::new());
    for t in tets {
        mesh.push_tetrahedron(t.indices);
    }
    mesh
}

// ============================================================================
// Internals
// ============================================================================

/// Vertex pool keyed by logical coordinates, so every piece of a composite
/// boundary references identical global indices for shared points (the
/// precondition for crack-free [`split_prism`] seams).
struct VertexPool {
    vertices: Vec<Vec4>,
    index: HashMap<(u32, u32), usize>,
}

impl VertexPool {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn get(&mut self, key: (u32, u32), position: Vec4) -> usize {
        *self.index.entry(key).or_insert_with(|| {
            self.vertices.push(position);
            self.vertices.len() - 1
        })
    }
}

/// Icosphere in the XYZ subspace: returns `(vertices, triangles)` with all
/// vertices at `radius` from the origin. `subdivisions = 0` is the bare
/// icosahedron (12 vertices, 20 faces); each level quadruples the faces.
fn icosphere(radius: f32, subdivisions: u32) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let phi = (1.0 + 5.0f32.sqrt()) / 2.0;
    let mut verts: Vec<[f32; 3]> = vec![
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ];
    let mut tris: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    let project = |v: [f32; 3]| -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [
            v[0] / len * radius,
            v[1] / len * radius,
            v[2] / len * radius,
        ]
    };
    for v in &mut verts {
        *v = project(*v);
    }

    for _ in 0..subdivisions {
        let mut midpoint_cache: HashMap<(usize, usize), usize> = HashMap::new();
        let mut next_tris = Vec::with_capacity(tris.len() * 4);
        for [a, b, c] in tris {
            let mut mid = |i: usize, j: usize, verts: &mut Vec<[f32; 3]>| -> usize {
                let key = (i.min(j), i.max(j));
                *midpoint_cache.entry(key).or_insert_with(|| {
                    let m = [
                        (verts[i][0] + verts[j][0]) * 0.5,
                        (verts[i][1] + verts[j][1]) * 0.5,
                        (verts[i][2] + verts[j][2]) * 0.5,
                    ];
                    verts.push(project(m));
                    verts.len() - 1
                })
            };
            let ab = mid(a, b, &mut verts);
            let bc = mid(b, c, &mut verts);
            let ca = mid(c, a, &mut verts);
            next_tris.push([a, ab, ca]);
            next_tris.push([b, bc, ab]);
            next_tris.push([c, ca, bc]);
            next_tris.push([ab, bc, ca]);
        }
        tris = next_tris;
    }

    (verts, tris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_hypersphere_structure() {
        for (d, cells) in [(0u32, 16usize), (1, 128), (2, 1024)] {
            let m = hypersphere(1.0, d);
            m.validate().unwrap();
            assert_eq!(m.tetrahedron_count(), cells, "subdivision {d}");
            assert!(m.is_watertight(), "hypersphere d={d} must be watertight");
            for v in m.vertices() {
                assert!((v.length() - 1.0).abs() < 1e-5, "all vertices on S³");
            }
        }
    }

    #[test]
    fn test_hypersphere_volume_converges_to_glome() {
        // Boundary 3-volume of S³(r) is 2π²r³. The inscribed approximation
        // approaches it from below as subdivision increases.
        let exact = 2.0 * PI * PI;
        let v1 = hypersphere(1.0, 1).surface_volume() as f64;
        let v2 = hypersphere(1.0, 2).surface_volume() as f64;
        let v3 = hypersphere(1.0, 3).surface_volume() as f64;
        assert!(
            v1 < v2 && v2 < v3 && v3 < exact,
            "monotone from below: {v1} {v2} {v3} {exact}"
        );
        assert!(
            (exact - v3) / exact < 0.05,
            "d=3 within 5% of 2π²: {v3} vs {exact}"
        );
        // Each subdivision halves edge length; an O(h²) scheme must shrink
        // the error by ~4× per level. Pin convergence *order*, not just size.
        let ratio = (exact - v2) / (exact - v3);
        assert!(
            ratio > 3.0,
            "expected quadratic convergence, error ratio {ratio}"
        );
    }

    #[test]
    fn test_hypersphere_subdivision_clamp() {
        // Levels above 4 are clamped — same mesh as 4.
        assert_eq!(hypersphere(1.0, 9).tetrahedron_count(), 16 * 8usize.pow(4));
    }

    #[test]
    fn test_spherinder_structure() {
        let m = spherinder(1.0, 0.75, 1);
        m.validate().unwrap();
        // 80 surface triangles → 2 caps × 80 + 80 prisms × 3 = 400 cells
        assert_eq!(m.tetrahedron_count(), 400);
        assert!(m.is_watertight(), "spherinder must be watertight");
    }

    #[test]
    fn test_spherinder_volume() {
        // Caps: 2 · (4/3)πr³.  Tube: 4πr² · 2h.  Inscribed → slightly below.
        let (r, h) = (1.0f64, 0.75f64);
        let exact = 2.0 * (4.0 / 3.0) * PI * r.powi(3) + 4.0 * PI * r * r * (2.0 * h);
        let measured = spherinder(r as f32, h as f32, 3).surface_volume() as f64;
        assert!(measured < exact, "inscribed mesh underestimates");
        assert!((exact - measured) / exact < 0.02, "{measured} vs {exact}");
    }

    #[test]
    fn test_cubinder_structure() {
        let m = cubinder(1.0, 0.5, 8);
        m.validate().unwrap();
        // Shell: 8 segments × 2 square-tris × 3. Plates: 4 edges × 8 fans × 3.
        assert_eq!(m.tetrahedron_count(), 8 * 2 * 3 + 4 * 8 * 3);
        assert!(m.is_watertight(), "cubinder must be watertight");
    }

    #[test]
    fn test_cubinder_volume() {
        // Shell: 2πr·(2s)². Plates: πr²·(8s). Circle inscribed → below exact.
        let (r, s) = (1.0f64, 0.5f64);
        let exact = 2.0 * PI * r * (2.0 * s) * (2.0 * s) + PI * r * r * 8.0 * s;
        let measured = cubinder(r as f32, s as f32, 64).surface_volume() as f64;
        assert!(measured < exact);
        assert!((exact - measured) / exact < 0.01, "{measured} vs {exact}");
    }

    #[test]
    fn test_duocylinder_structure() {
        let m = duocylinder(1.0, 0.8, 8, 6);
        m.validate().unwrap();
        // Piece 1: 8·6 prisms; piece 2: 6·8 prisms; ×3 tets.
        assert_eq!(m.tetrahedron_count(), 2 * 8 * 6 * 3);
        assert!(m.is_watertight(), "duocylinder must be watertight");
    }

    #[test]
    fn test_duocylinder_volume() {
        // Piece 1: 2πr1 · πr2². Piece 2: πr1² · 2πr2.
        let (r1, r2) = (1.0f64, 0.8f64);
        let exact = 2.0 * PI * r1 * PI * r2 * r2 + PI * r1 * r1 * 2.0 * PI * r2;
        let measured = duocylinder(r1 as f32, r2 as f32, 48, 48).surface_volume() as f64;
        assert!(measured < exact);
        assert!((exact - measured) / exact < 0.01, "{measured} vs {exact}");
    }

    #[test]
    fn test_curved_shapes_bounded() {
        // Bounding radius sanity: every shape fits its analytic bound.
        assert!(hypersphere(2.0, 1).bounding_radius() <= 2.0 + 1e-4);
        assert!(spherinder(1.0, 0.5, 1).bounding_radius() <= (1.0f32 + 0.25).sqrt() + 1e-4);
        assert!(cubinder(1.0, 0.5, 12).bounding_radius() <= (1.0f32 + 0.5).sqrt() + 1e-4);
        assert!(duocylinder(1.0, 1.0, 12, 12).bounding_radius() <= 2.0f32.sqrt() + 1e-4);
    }
}
