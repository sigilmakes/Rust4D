// 4D Cross-Section Compute Shader (Tetrahedra Version)
//
// This shader slices 4D tetrahedra with a W-hyperplane and produces
// 3D triangles for rendering.
//
// Algorithm:
// 1. For each tetrahedron, transform vertices by camera matrix
// 2. Compute which vertices are above the slice plane (case index 0-15)
// 3. Use lookup tables to determine which edges are crossed
// 4. Interpolate intersection points along crossed edges
// 5. Generate 0-2 triangles from intersection points
//
// Tetrahedra are simpler than 5-cells:
// - Only 16 cases (4 vertices) instead of 32 (5 vertices)
// - Maximum 4 intersection points (quad) instead of 6 (prism)
// - Output 0-2 triangles instead of 0-8

// ============================================================================
// Data Structures
// ============================================================================

/// A vertex in 4D space with color
struct Vertex4D {
    position: vec4<f32>,  // x, y, z, w
    color: vec4<f32>,     // r, g, b, a
}

/// A tetrahedron specified by 4 vertex indices
struct Tetrahedron {
    v0: u32,
    v1: u32,
    v2: u32,
    v3: u32,
}

/// A 3D triangle vertex for output
/// Layout must match Rust Vertex3D: 48 bytes total (12 floats)
struct Vertex3D {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    norm_x: f32,
    norm_y: f32,
    norm_z: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
    w_depth: f32,
    _padding: f32,
}

/// A 3D triangle (3 vertices)
struct Triangle3D {
    v0: Vertex3D,
    v1: Vertex3D,
    v2: Vertex3D,
}

/// Parameters for the slice operation
struct SliceParams {
    slice_w: f32,
    tetrahedron_count: u32,
    _pad0: f32,
    _pad1: f32,
    camera_matrix: mat4x4<f32>,  // Camera-local to world (needs transpose for view)
    camera_eye: vec3<f32>,
    _pad2: f32,
    camera_position: vec4<f32>,  // 4D camera position
}

// ============================================================================
// Buffers
// ============================================================================

// Group 0: Main buffers
@group(0) @binding(0) var<storage, read> vertices: array<Vertex4D>;
@group(0) @binding(1) var<storage, read> tetrahedra: array<Tetrahedron>;
@group(0) @binding(2) var<storage, read_write> triangles: array<Triangle3D>;
@group(0) @binding(3) var<storage, read_write> triangle_count: atomic<u32>;
@group(0) @binding(4) var<uniform> params: SliceParams;

// ============================================================================
// Constants - Tetrahedron Lookup Tables
// ============================================================================

// Edge definitions for a tetrahedron (6 edges connecting 4 vertices)
const TETRA_EDGE_V0: array<u32, 6> = array<u32, 6>(0u, 0u, 0u, 1u, 1u, 2u);
const TETRA_EDGE_V1: array<u32, 6> = array<u32, 6>(1u, 2u, 3u, 2u, 3u, 3u);

// Edge table: bit i set if edge i is crossed for this case
const TETRA_EDGE_TABLE: array<u32, 16> = array<u32, 16>(
    0x00u, // Case 0:  all below - no intersection
    0x07u, // Case 1:  v0 above - edges 0,1,2
    0x19u, // Case 2:  v1 above - edges 0,3,4
    0x1Eu, // Case 3:  v0,v1 above - edges 1,2,3,4 (quad)
    0x2Au, // Case 4:  v2 above - edges 1,3,5
    0x2Du, // Case 5:  v0,v2 above - edges 0,2,3,5 (quad)
    0x33u, // Case 6:  v1,v2 above - edges 0,1,4,5 (quad)
    0x34u, // Case 7:  v0,v1,v2 above - edges 2,4,5
    0x34u, // Case 8:  v3 above - edges 2,4,5
    0x33u, // Case 9:  v0,v3 above - edges 0,1,4,5 (quad)
    0x2Du, // Case 10: v1,v3 above - edges 0,2,3,5 (quad)
    0x2Au, // Case 11: v0,v1,v3 above - edges 1,3,5
    0x1Eu, // Case 12: v2,v3 above - edges 1,2,3,4 (quad)
    0x19u, // Case 13: v0,v2,v3 above - edges 0,3,4
    0x07u, // Case 14: v1,v2,v3 above - edges 0,1,2
    0x00u  // Case 15: all above - no intersection
);

// Triangle count for each case
const TETRA_TRI_COUNT: array<u32, 16> = array<u32, 16>(
    0u, 1u, 1u, 2u, 1u, 2u, 2u, 1u,
    1u, 2u, 2u, 1u, 2u, 1u, 1u, 0u
);

// Triangle indices for each case
// For triangle cases (3 edges): indices 0,1,2
// For quad cases (4 edges): points collected in edge order but cyclic order is 0,1,3,2
//   So triangles are (0,1,3) and (0,3,2) for proper fan triangulation
// Stored as: [t0_i0, t0_i1, t0_i2, t1_i0, t1_i1, t1_i2] (-1 for unused)
const TETRA_TRI_TABLE: array<array<i32, 6>, 16> = array<array<i32, 6>, 16>(
    array<i32, 6>(-1, -1, -1, -1, -1, -1), // Case 0
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 1
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 2
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 3 (quad) - cyclic order 0,1,3,2
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 4
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 5 (quad)
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 6 (quad)
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 7
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 8
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 9 (quad)
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 10 (quad)
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 11
    array<i32, 6>( 0,  1,  3,  0,  3,  2), // Case 12 (quad)
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 13
    array<i32, 6>( 0,  1,  2, -1, -1, -1), // Case 14
    array<i32, 6>(-1, -1, -1, -1, -1, -1)  // Case 15
);

// ============================================================================
// Helper Functions
// ============================================================================

/// Transform a 4D world position to camera space
/// 1. Translate by -camera_position (move camera to origin)
/// 2. Rotate by transpose(camera_matrix) (camera_matrix is camera→world, transpose gives world→camera)
fn transform_to_camera_space(world_pos: vec4<f32>, camera_pos: vec4<f32>, camera_mat: mat4x4<f32>) -> vec4<f32> {
    let relative_pos = world_pos - camera_pos;
    // camera_mat transforms camera-local to world, so transpose transforms world to camera-local
    return transpose(camera_mat) * relative_pos;
}

/// Compute the intersection point on an edge
fn edge_intersection(
    p0: vec4<f32>,
    p1: vec4<f32>,
    c0: vec4<f32>,
    c1: vec4<f32>,
    slice_w: f32
) -> Vertex3D {
    let w0 = p0.w;
    let w1 = p1.w;
    let dw = w1 - w0;
    let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);

    let pos = mix(p0, p1, t);
    let color = mix(c0, c1, t);

    var vertex: Vertex3D;
    vertex.pos_x = pos.x;
    vertex.pos_y = pos.y;
    vertex.pos_z = pos.z;
    vertex.norm_x = 0.0;
    vertex.norm_y = 0.0;
    vertex.norm_z = 0.0;
    vertex.color_r = color.r;
    vertex.color_g = color.g;
    vertex.color_b = color.b;
    vertex.color_a = color.a;
    vertex.w_depth = slice_w;
    vertex._padding = 0.0;
    return vertex;
}

fn vertex_position(v: Vertex3D) -> vec3<f32> {
    return vec3<f32>(v.pos_x, v.pos_y, v.pos_z);
}

fn vertex_with_normal(v: Vertex3D, normal: vec3<f32>) -> Vertex3D {
    var result = v;
    result.norm_x = normal.x;
    result.norm_y = normal.y;
    result.norm_z = normal.z;
    return result;
}

fn compute_normal(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> vec3<f32> {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    let n = cross(e1, e2);
    let len = length(n);
    if (len < 0.0001) {
        return vec3<f32>(0.0, 1.0, 0.0); // Degenerate triangle fallback
    }
    return n / len;
}

// ============================================================================
// Main Compute Shader
// ============================================================================

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tet_idx = global_id.x;

    if (tet_idx >= params.tetrahedron_count) {
        return;
    }

    let tet = tetrahedra[tet_idx];
    let slice_w = params.slice_w;
    let camera_mat = params.camera_matrix;
    let camera_pos = params.camera_position;

    // Load and transform vertices to camera space
    let v0 = vertices[tet.v0];
    let v1 = vertices[tet.v1];
    let v2 = vertices[tet.v2];
    let v3 = vertices[tet.v3];

    var pos: array<vec4<f32>, 4>;
    pos[0] = transform_to_camera_space(v0.position, camera_pos, camera_mat);
    pos[1] = transform_to_camera_space(v1.position, camera_pos, camera_mat);
    pos[2] = transform_to_camera_space(v2.position, camera_pos, camera_mat);
    pos[3] = transform_to_camera_space(v3.position, camera_pos, camera_mat);

    var col: array<vec4<f32>, 4>;
    col[0] = v0.color;
    col[1] = v1.color;
    col[2] = v2.color;
    col[3] = v3.color;

    // Compute case index (4 bits)
    var case_idx: u32 = 0u;
    if (pos[0].w > slice_w) { case_idx |= 1u; }
    if (pos[1].w > slice_w) { case_idx |= 2u; }
    if (pos[2].w > slice_w) { case_idx |= 4u; }
    if (pos[3].w > slice_w) { case_idx |= 8u; }

    // Skip if no intersection
    if (case_idx == 0u || case_idx == 15u) {
        return;
    }

    // Get edge mask and triangle count
    let edge_mask = TETRA_EDGE_TABLE[case_idx];
    let tri_count = TETRA_TRI_COUNT[case_idx];

    // Compute intersection points for crossed edges
    var points: array<Vertex3D, 4>;
    var point_idx: u32 = 0u;

    for (var edge: u32 = 0u; edge < 6u; edge++) {
        if ((edge_mask & (1u << edge)) != 0u) {
            let ev0 = TETRA_EDGE_V0[edge];
            let ev1 = TETRA_EDGE_V1[edge];
            points[point_idx] = edge_intersection(
                pos[ev0], pos[ev1],
                col[ev0], col[ev1],
                slice_w
            );
            point_idx++;
        }
    }

    // Output triangles
    let tri_indices = TETRA_TRI_TABLE[case_idx];

    for (var t: u32 = 0u; t < tri_count; t++) {
        let base = t * 3u;
        let i0 = u32(tri_indices[base]);
        let i1 = u32(tri_indices[base + 1u]);
        let i2 = u32(tri_indices[base + 2u]);

        var tv0 = points[i0];
        var tv1 = points[i1];
        var tv2 = points[i2];

        // Compute normal
        let p0 = vertex_position(tv0);
        let p1 = vertex_position(tv1);
        let p2 = vertex_position(tv2);
        var normal = compute_normal(p0, p1, p2);

        // Ensure normals face toward the camera.
        // In camera space the camera is at the origin, so the direction
        // from the triangle to the camera is simply -tri_center.
        // (The old bug used params.camera_eye which was in world space,
        // not camera space, causing incorrect flips when the camera rotated.)
        let tri_center = (p0 + p1 + p2) / 3.0;
        let to_camera = -tri_center;
        if (dot(normal, to_camera) < 0.0) {
            let tmp = tv1;
            tv1 = tv2;
            tv2 = tmp;
            normal = -normal;
        }

        tv0 = vertex_with_normal(tv0, normal);
        tv1 = vertex_with_normal(tv1, normal);
        tv2 = vertex_with_normal(tv2, normal);

        // Allocate output slot atomically
        // Increment by 3 because DrawIndirect needs vertex count, not triangle count
        let vertex_idx = atomicAdd(&triangle_count, 3u);
        let output_idx = vertex_idx / 3u;

        triangles[output_idx].v0 = tv0;
        triangles[output_idx].v1 = tv1;
        triangles[output_idx].v2 = tv2;
    }
}
