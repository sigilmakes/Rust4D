//! Lookup tables for 4D cross-section rendering
//!
//! A tetrahedron has 4 vertices and 6 edges. When sliced by a hyperplane,
//! we get 2^4 = 16 possible configurations:
//! - 0 or 4 vertices above: no intersection
//! - 1 or 3 vertices above: triangle (3 edges crossed)
//! - 2 vertices above: quadrilateral (4 edges crossed, split into 2 triangles)

/// Edge definitions for a tetrahedron (4 vertices, 6 edges)
/// Each edge connects two vertices (indexed 0-3)
pub const TETRA_EDGES: [[usize; 2]; 6] = [
    [0, 1], // Edge 0
    [0, 2], // Edge 1
    [0, 3], // Edge 2
    [1, 2], // Edge 3
    [1, 3], // Edge 4
    [2, 3], // Edge 5
];

/// For each case (0-15), which edges are crossed by the slice plane.
/// Bit i is set if edge i is crossed.
pub const TETRA_EDGE_TABLE: [u8; 16] = compute_tetra_edge_table();

/// Triangle table for tetrahedra: for each case, how to form triangles.
/// Each entry has up to 6 indices (2 triangles max), with -1 indicating end.
///
/// Cases with 1 or 3 vertices above: 1 triangle (3 points)
/// Cases with 2 vertices above: 2 triangles (4 points forming a quad)
pub const TETRA_TRI_TABLE: [[i8; 6]; 16] = compute_tetra_tri_table();

/// Number of triangles produced by each case
pub const TETRA_TRI_COUNT: [u8; 16] = [
    0, // Case 0: all below
    1, // Case 1: v0 above
    1, // Case 2: v1 above
    2, // Case 3: v0,v1 above (quad)
    1, // Case 4: v2 above
    2, // Case 5: v0,v2 above (quad)
    2, // Case 6: v1,v2 above (quad)
    1, // Case 7: v0,v1,v2 above
    1, // Case 8: v3 above
    2, // Case 9: v0,v3 above (quad)
    2, // Case 10: v1,v3 above (quad)
    1, // Case 11: v0,v1,v3 above
    2, // Case 12: v2,v3 above (quad)
    1, // Case 13: v0,v2,v3 above
    1, // Case 14: v1,v2,v3 above
    0, // Case 15: all above
];

/// Compute the tetrahedra edge table at compile time
const fn compute_tetra_edge_table() -> [u8; 16] {
    let mut table = [0u8; 16];
    let mut case_idx: usize = 0;

    while case_idx < 16 {
        let mut edge_mask = 0u8;
        let mut edge_idx = 0;

        while edge_idx < 6 {
            let v0 = TETRA_EDGES[edge_idx][0];
            let v1 = TETRA_EDGES[edge_idx][1];

            let v0_above = (case_idx >> v0) & 1;
            let v1_above = (case_idx >> v1) & 1;

            if v0_above != v1_above {
                edge_mask |= 1 << edge_idx;
            }

            edge_idx += 1;
        }

        table[case_idx] = edge_mask;
        case_idx += 1;
    }

    table
}

/// Compute the tetrahedra triangle table at compile time
///
/// For each case, we need to output triangles from the intersection points.
/// The intersection points are indexed by which edge they came from.
///
/// Triangle cases (1 or 3 above): 3 edges crossed, output 1 triangle
/// Quad cases (2 above): 4 edges crossed, output 2 triangles
const fn compute_tetra_tri_table() -> [[i8; 6]; 16] {
    let empty: [i8; 6] = [-1, -1, -1, -1, -1, -1];

    // For triangle cases, we output vertices in the order the edges are crossed
    // (edges are ordered by index). This gives consistent winding.
    // Triangle: indices 0,1,2 (the 3 crossed edges in order)
    let tri: [i8; 6] = [0, 1, 2, -1, -1, -1];

    // For quad cases with 4 points, we need to split into 2 triangles.
    // Points come from 4 edges. We need to determine proper triangulation.
    //
    // When 2 vertices are above (say v0,v1) and 2 below (v2,v3):
    // - Crossed edges connect above vertices to below vertices
    // - Edge order: we get points in edge index order
    //
    // The quad vertices form a cycle. To triangulate correctly, we need
    // to identify which edges are "opposite" in the quad.
    //
    // General approach: for quad with vertices p0,p1,p2,p3 (in edge order),
    // we triangulate as (p0,p1,p2) and (p0,p2,p3).

    let mut table: [[i8; 6]; 16] = [empty; 16];

    // Case 0: all below - no intersection
    // Case 15: all above - no intersection

    // === Triangle cases (1 vertex above) ===
    // Case 1: v0 above - edges 0,1,2 crossed (v0-v1, v0-v2, v0-v3)
    table[1] = tri;
    // Case 2: v1 above - edges 0,3,4 crossed (v0-v1, v1-v2, v1-v3)
    table[2] = tri;
    // Case 4: v2 above - edges 1,3,5 crossed (v0-v2, v1-v2, v2-v3)
    table[4] = tri;
    // Case 8: v3 above - edges 2,4,5 crossed (v0-v3, v1-v3, v2-v3)
    table[8] = tri;

    // === Triangle cases (3 vertices above = 1 below) ===
    // Case 7: v0,v1,v2 above (v3 below) - edges 2,4,5 crossed
    table[7] = tri;
    // Case 11: v0,v1,v3 above (v2 below) - edges 1,3,5 crossed
    table[11] = tri;
    // Case 13: v0,v2,v3 above (v1 below) - edges 0,3,4 crossed
    table[13] = tri;
    // Case 14: v1,v2,v3 above (v0 below) - edges 0,1,2 crossed
    table[14] = tri;

    // === Quad cases (2 vertices above) ===
    // These need special handling to ensure correct triangulation.
    // When 2 vertices are above, the 4 crossed edges connect the 2 "above"
    // vertices to the 2 "below" vertices, forming a quadrilateral.
    //
    // For proper triangulation, we split the quad into 2 triangles along
    // one diagonal. The correct diagonal depends on the geometry.
    //
    // For a quad with intersection points p0,p1,p2,p3 (in edge index order),
    // we use triangles (0,1,2) and (0,2,3) as a consistent approach.

    let quad: [i8; 6] = [0, 1, 2, 0, 2, 3];

    // Case 3: v0,v1 above - edges 1,2,3,4 crossed (skip 0: v0-v1, skip 5: v2-v3)
    table[3] = quad;
    // Case 5: v0,v2 above - edges 0,2,3,5 crossed (skip 1: v0-v2, skip 4: v1-v3)
    table[5] = quad;
    // Case 6: v1,v2 above - edges 0,1,4,5 crossed (skip 3: v1-v2, skip 2: v0-v3)
    table[6] = quad;
    // Case 9: v0,v3 above - edges 0,1,4,5 crossed (skip 2: v0-v3, skip 3: v1-v2)
    table[9] = quad;
    // Case 10: v1,v3 above - edges 0,2,3,5 crossed (skip 4: v1-v3, skip 1: v0-v2)
    table[10] = quad;
    // Case 12: v2,v3 above - edges 1,2,3,4 crossed (skip 5: v2-v3, skip 0: v0-v1)
    table[12] = quad;

    table
}

/// Get the number of tetrahedra edges crossed for a given case
pub const fn tetra_edge_count(case_idx: usize) -> usize {
    TETRA_EDGE_TABLE[case_idx].count_ones() as usize
}

/// Get the list of crossed tetrahedra edge indices for a given case
pub fn tetra_crossed_edges(case_idx: usize) -> Vec<usize> {
    let mask = TETRA_EDGE_TABLE[case_idx];
    (0..6).filter(|i| (mask >> i) & 1 == 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tetra_edge_table_empty_cases() {
        // Case 0 (all below) and case 15 (all above) have no edges crossed
        assert_eq!(TETRA_EDGE_TABLE[0], 0);
        assert_eq!(TETRA_EDGE_TABLE[15], 0);
    }

    #[test]
    fn test_tetra_edge_table_single_vertex() {
        // Case 1: v0 above - edges 0,1,2 crossed (connecting v0 to v1,v2,v3)
        assert_eq!(TETRA_EDGE_TABLE[1], 0b000111);

        // Case 2: v1 above - edges 0,3,4 crossed
        assert_eq!(TETRA_EDGE_TABLE[2], 0b011001);

        // Case 4: v2 above - edges 1,3,5 crossed
        assert_eq!(TETRA_EDGE_TABLE[4], 0b101010);

        // Case 8: v3 above - edges 2,4,5 crossed
        assert_eq!(TETRA_EDGE_TABLE[8], 0b110100);
    }

    #[test]
    fn test_tetra_edge_table_two_vertices() {
        // Case 3: v0,v1 above - 4 edges crossed (edges connecting {v0,v1} to {v2,v3})
        // Edge 0 (v0-v1): NOT crossed (both above)
        // Edge 5 (v2-v3): NOT crossed (both below)
        // Crossed: 1,2,3,4
        assert_eq!(TETRA_EDGE_TABLE[3], 0b011110);
        assert_eq!(TETRA_EDGE_TABLE[3].count_ones(), 4);
    }

    #[test]
    fn test_tetra_edge_count_distribution() {
        // 0 edges: cases 0 and 15
        // 3 edges: 1 or 3 vertices above (8 cases)
        // 4 edges: 2 vertices above (6 cases)

        let count_0 = (0..16).filter(|&i| TETRA_EDGE_TABLE[i].count_ones() == 0).count();
        let count_3 = (0..16).filter(|&i| TETRA_EDGE_TABLE[i].count_ones() == 3).count();
        let count_4 = (0..16).filter(|&i| TETRA_EDGE_TABLE[i].count_ones() == 4).count();

        assert_eq!(count_0, 2);  // cases 0 and 15
        assert_eq!(count_3, 8);  // C(4,1) + C(4,3) = 4 + 4
        assert_eq!(count_4, 6);  // C(4,2) = 6
    }

    #[test]
    fn test_tetra_tri_table_coverage() {
        for case_idx in 0..16 {
            let edge_count = TETRA_EDGE_TABLE[case_idx].count_ones();
            let expected_tris = TETRA_TRI_COUNT[case_idx] as u32;

            // Count triangles in table
            let mut tri_count = 0u32;
            for i in (0..6).step_by(3) {
                if TETRA_TRI_TABLE[case_idx][i] >= 0 {
                    tri_count += 1;
                } else {
                    break;
                }
            }

            assert_eq!(tri_count, expected_tris,
                "Case {} should have {} triangles, got {}", case_idx, expected_tris, tri_count);

            match edge_count {
                0 => assert_eq!(tri_count, 0),
                3 => assert_eq!(tri_count, 1, "3 edges = triangle = 1 output"),
                4 => assert_eq!(tri_count, 2, "4 edges = quad = 2 triangles"),
                _ => panic!("Unexpected edge count {} for case {}", edge_count, case_idx),
            }
        }
    }

    #[test]
    fn test_tetra_symmetry() {
        // Case i and case (15-i) should have same number of edges crossed
        for i in 0..8 {
            assert_eq!(
                TETRA_EDGE_TABLE[i].count_ones(),
                TETRA_EDGE_TABLE[15 - i].count_ones(),
                "Cases {} and {} should have same edge count", i, 15 - i
            );
        }
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // index used across two tables
    fn test_tetra_tri_indices_valid() {
        for case_idx in 0..16 {
            let edge_mask = TETRA_EDGE_TABLE[case_idx];
            let num_edges = edge_mask.count_ones() as i8;

            for i in 0..6 {
                let idx = TETRA_TRI_TABLE[case_idx][i];
                if idx >= 0 {
                    assert!(idx < num_edges,
                        "Case {}: triangle index {} out of range (only {} edges)",
                        case_idx, idx, num_edges);
                }
            }
        }
    }
}
