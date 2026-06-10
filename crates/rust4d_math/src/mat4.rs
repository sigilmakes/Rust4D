//! 4x4 Matrix utilities for 4D transformations
//!
//! This module provides matrix operations needed for Engine4D-style camera control,
//! including the critical SkipY transformation that remaps 3D rotations to 4D
//! while keeping the Y axis unchanged.

use crate::Vec4;

/// Epsilon for geometric comparisons (near-zero length, parallelism checks).
/// Used in `ortho_iterate`, `from_to_rotation`, and similar functions.
const GEOMETRIC_EPSILON: f32 = 1e-6;

/// 4x4 matrix type (column-major)
pub type Mat4 = [[f32; 4]; 4];

/// Identity matrix
pub const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Create a rotation matrix in a specific 2D plane within 4D space.
///
/// This is equivalent to Engine4D's `Transform4D.PlaneRotation`.
///
/// # Arguments
/// * `angle` - Rotation angle in radians
/// * `p1`, `p2` - Indices of the axes forming the rotation plane (0=X, 1=Y, 2=Z, 3=W)
///
/// # Example
/// ```
/// use rust4d_math::mat4::plane_rotation;
/// // Create a YZ plane rotation (pitch)
/// let pitch_matrix = plane_rotation(0.5, 1, 2);
/// ```
pub fn plane_rotation(angle: f32, p1: usize, p2: usize) -> Mat4 {
    let cs = angle.cos();
    let sn = angle.sin();

    let mut m = IDENTITY;

    // Rotation in plane p1-p2
    m[p1][p1] = cs;
    m[p2][p2] = cs;
    m[p1][p2] = sn;
    m[p2][p1] = -sn;

    m
}

/// Remap a 4D rotation matrix so it operates in the XZW hyperplane,
/// leaving the Y axis unchanged.
///
/// This is the critical transformation from Engine4D (`Transform4D.SkipY`).
/// It maps:
/// - X axis → X axis (unchanged)
/// - Y axis → Z axis (in 4D)
/// - Z axis → W axis (in 4D)
///
/// The Y axis of the *output* remains identity, preserving gravity alignment.
///
/// # Why this matters
/// When you apply 4D rotations with SkipY, the Y axis (gravity direction) is
/// never affected. This means walking forward always stays horizontal relative
/// to world up, regardless of what 4D rotation state you're in.
///
/// # Implementation
/// This is equivalent to Engine4D's `XYZTo(matrix, 0, 2, 3)`:
/// - Takes a 3x3 rotation embedded in 4x4 (top-left 3x3)
/// - Remaps columns: 0→0, 1→2, 2→3
/// - Remaps rows: 0→0, 1→2, 2→3
/// - Column/row 1 (Y) is left as identity
pub fn skip_y(m: Mat4) -> Mat4 {
    // The input matrix is a 3D rotation embedded in 4x4 (top-left 3x3 is rotation).
    // We need to remap it so that the rotation affects XZW instead of XYZ.
    //
    // Engine4D's XYZTo does:
    // 1. Create a column-remapped matrix: columns 0,1,2 → columns sendX,sendY,sendZ
    // 2. Create a row-remapped matrix from that
    //
    // For SkipY: sendX=0, sendY=2, sendZ=3 (skip position 1)

    let mut result = IDENTITY;

    // The rotation in the input affects indices 0,1,2 (XYZ in 3D)
    // We want it to affect indices 0,2,3 (XZW in 4D)

    // Remap: input col 0 (X) → output col 0 (X)
    //        input col 1 (Y) → output col 2 (Z)
    //        input col 2 (Z) → output col 3 (W)
    // Output col 1 (Y) stays identity

    // Copy the 3x3 rotation with remapping
    // Input indices [0,1,2] map to output indices [0,2,3]
    let src_idx = [0usize, 1, 2];
    let dst_idx = [0usize, 2, 3];

    for i in 0..3 {
        for j in 0..3 {
            result[dst_idx[j]][dst_idx[i]] = m[src_idx[j]][src_idx[i]];
        }
    }

    // Y column/row stays identity (already set)
    result[1][1] = 1.0;

    result
}

/// Multiply two 4x4 matrices: result = a * b
///
/// In column-major convention, this applies b first, then a.
#[allow(clippy::needless_range_loop)]
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut result = [[0.0f32; 4]; 4];

    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[k][j] * b[i][k];
            }
        }
    }

    result
}

/// Transform a Vec4 by a 4x4 matrix (column-major)
///
/// result = M * v
pub fn transform(m: Mat4, v: Vec4) -> Vec4 {
    Vec4::new(
        m[0][0] * v.x + m[1][0] * v.y + m[2][0] * v.z + m[3][0] * v.w,
        m[0][1] * v.x + m[1][1] * v.y + m[2][1] * v.z + m[3][1] * v.w,
        m[0][2] * v.x + m[1][2] * v.y + m[2][2] * v.z + m[3][2] * v.w,
        m[0][3] * v.x + m[1][3] * v.y + m[2][3] * v.z + m[3][3] * v.w,
    )
}

/// Get a column vector from a matrix
pub fn get_column(m: Mat4, col: usize) -> Vec4 {
    Vec4::new(m[col][0], m[col][1], m[col][2], m[col][3])
}

/// Set a column of a matrix
pub fn set_column(m: &mut Mat4, col: usize, v: Vec4) {
    m[col][0] = v.x;
    m[col][1] = v.y;
    m[col][2] = v.z;
    m[col][3] = v.w;
}

/// Get a row vector from a matrix
pub fn get_row(m: Mat4, row: usize) -> Vec4 {
    Vec4::new(m[0][row], m[1][row], m[2][row], m[3][row])
}

/// Set a row of a matrix
pub fn set_row(m: &mut Mat4, row: usize, v: Vec4) {
    m[0][row] = v.x;
    m[1][row] = v.y;
    m[2][row] = v.z;
    m[3][row] = v.w;
}

/// Negate a row of a matrix (used for view matrix transformation)
pub fn negate_row(m: &mut Mat4, row: usize) {
    m[0][row] = -m[0][row];
    m[1][row] = -m[1][row];
    m[2][row] = -m[2][row];
    m[3][row] = -m[3][row];
}

/// Transpose a matrix
pub fn transpose(m: Mat4) -> Mat4 {
    [
        [m[0][0], m[1][0], m[2][0], m[3][0]],
        [m[0][1], m[1][1], m[2][1], m[3][1]],
        [m[0][2], m[1][2], m[2][2], m[3][2]],
        [m[0][3], m[1][3], m[2][3], m[3][3]],
    ]
}

/// Outer product of two Vec4: `result[i][j] = a[i] * b[j]`
///
/// Creates a matrix where each column is `a` scaled by the corresponding
/// component of `b`.
pub fn outer_product(a: Vec4, b: Vec4) -> Mat4 {
    [
        [a.x * b.x, a.y * b.x, a.z * b.x, a.w * b.x],
        [a.x * b.y, a.y * b.y, a.z * b.y, a.w * b.y],
        [a.x * b.z, a.y * b.z, a.z * b.z, a.w * b.z],
        [a.x * b.w, a.y * b.w, a.z * b.w, a.w * b.w],
    ]
}

/// Matrix addition: result = a + b
#[allow(clippy::needless_range_loop)]
pub fn add(a: Mat4, b: Mat4) -> Mat4 {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = a[i][j] + b[i][j];
        }
    }
    result
}

/// Scalar multiplication: result = m * s
#[allow(clippy::needless_range_loop)]
pub fn scale_by(m: Mat4, s: f32) -> Mat4 {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = m[i][j] * s;
        }
    }
    result
}

/// Create a diagonal scale matrix from a Vec4
pub fn scale(s: Vec4) -> Mat4 {
    [
        [s.x, 0.0, 0.0, 0.0],
        [0.0, s.y, 0.0, 0.0],
        [0.0, 0.0, s.z, 0.0],
        [0.0, 0.0, 0.0, s.w],
    ]
}

/// Re-orthogonalize a matrix to prevent numerical drift.
///
/// Uses one iteration of a Gram-Schmidt-like process, similar to Engine4D's
/// `OrthoIterate`. This should be called periodically on rotation matrices
/// that accumulate over many frames.
///
/// The algorithm:
/// 1. Normalize each column
/// 2. Remove mutual projections between columns
#[allow(clippy::needless_range_loop)] // index pairs (i, j) into mt are clearest here
pub fn ortho_iterate(mut m: Mat4) -> Mat4 {
    // Normalize columns
    for i in 0..4 {
        let col = get_column(m, i);
        let mag = col.length();
        if mag < GEOMETRIC_EPSILON {
            return IDENTITY; // Degenerate matrix, return identity to avoid inconsistent state
        }
        set_column(&mut m, i, col * (1.0 / mag));
    }

    // Compute M^T * M (gives dot products between columns)
    let mt = mul(transpose(m), m);

    // Remove mutual projections and re-normalize
    let mut result = IDENTITY;
    for i in 0..4 {
        let mut sum = get_column(m, i);
        for j in 0..4 {
            if i != j {
                let col_j = get_column(m, j);
                sum += col_j * (-0.5 * mt[i][j]);
            }
        }
        // Re-normalize after projection removal to ensure unit-length columns
        set_column(&mut result, i, sum.normalized());
    }

    result
}

/// Create a rotation matrix that rotates `from` direction to `to` direction.
///
/// Vectors are normalized internally for robustness. Uses the Householder
/// reflection method: two reflections compose to a rotation.
///
/// For anti-parallel vectors (180° rotation), uses a two-step rotation through
/// a perpendicular intermediate axis to avoid numerical instability.
pub fn from_to_rotation(from: Vec4, to: Vec4) -> Mat4 {
    let from = from.normalized();
    let to = to.normalized();
    let c = from + to;
    let mag_sq = c.length_squared();

    if mag_sq < GEOMETRIC_EPSILON {
        // Vectors are nearly anti-parallel. The Householder method becomes unstable
        // when from + to ≈ 0. Use two-step rotation through a perpendicular axis.
        //
        // SAFETY: find_perpendicular returns a vector orthogonal to `from`, so
        // (from + perp).length_squared() ≈ 2.0, always >> GEOMETRIC_EPSILON.
        // Recursion depth is therefore at most 1.
        let perp = find_perpendicular(from);
        let r1 = from_to_rotation(from, perp);
        let r2 = from_to_rotation(perp, to);
        return mul(r2, r1);
    }

    // First reflection: reflect across the hyperplane perpendicular to (from + to)
    // This is a Householder reflection: I - 2 * (c ⊗ c) / |c|²
    let s = add(IDENTITY, scale_by(outer_product(c, c), -2.0 / mag_sq));

    // Transform `to` by this reflection
    let s_to = transform(s, to);

    // Second reflection: across the hyperplane perpendicular to (to - S*to)
    // The composition of these two reflections gives the rotation
    add(s, outer_product(to * -2.0, s_to))
}

/// Find a unit vector perpendicular to the given vector.
/// Delegates to `Vec4::find_perpendicular`.
fn find_perpendicular(v: Vec4) -> Vec4 {
    v.find_perpendicular()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    fn mat_approx_eq(a: Mat4, b: Mat4) -> bool {
        for i in 0..4 {
            for j in 0..4 {
                if !approx_eq(a[i][j], b[i][j]) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_identity() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = transform(IDENTITY, v);
        assert!(vec_approx_eq(v, result));
    }

    #[test]
    fn test_plane_rotation_yz() {
        use std::f32::consts::FRAC_PI_2;

        // 90° rotation in YZ plane (pitch)
        let m = plane_rotation(FRAC_PI_2, 1, 2);

        // Y should go to Z
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(m, y);
        assert!(
            vec_approx_eq(result, Vec4::new(0.0, 0.0, 1.0, 0.0)),
            "Y should become Z, got {:?}",
            result
        );

        // Z should go to -Y
        let z = Vec4::new(0.0, 0.0, 1.0, 0.0);
        let result = transform(m, z);
        assert!(
            vec_approx_eq(result, Vec4::new(0.0, -1.0, 0.0, 0.0)),
            "Z should become -Y, got {:?}",
            result
        );

        // X should be unchanged
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result = transform(m, x);
        assert!(
            vec_approx_eq(result, x),
            "X should be unchanged, got {:?}",
            result
        );
    }

    #[test]
    fn test_skip_y_preserves_y_axis() {
        use crate::RotationPlane;
        use crate::Rotor4;
        use std::f32::consts::FRAC_PI_4;

        // Create a 3D rotation (using YZ plane which affects Y and Z)
        let r = Rotor4::from_plane_angle(RotationPlane::YZ, FRAC_PI_4);
        let m = r.to_matrix();

        // Apply SkipY
        let skip_m = skip_y(m);

        // Now transform Y axis - should be unchanged!
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(skip_m, y);

        assert!(
            vec_approx_eq(result, y),
            "Y axis should be preserved after skip_y, got {:?}",
            result
        );
    }

    #[test]
    fn test_skip_y_remaps_rotation() {
        use crate::RotationPlane;
        use crate::Rotor4;
        use std::f32::consts::FRAC_PI_2;

        // Create a 90° rotation in XY plane (affects X and Y)
        let r = Rotor4::from_plane_angle(RotationPlane::XY, FRAC_PI_2);
        let m = r.to_matrix();

        // Original: X→Y, Y→-X
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let original_result = transform(m, x);
        assert!(
            vec_approx_eq(original_result, Vec4::new(0.0, 1.0, 0.0, 0.0)),
            "Original: X should become Y, got {:?}",
            original_result
        );

        // After SkipY: rotation is now in XZ plane (indices 0,2)
        let skip_m = skip_y(m);

        // X should now go to Z (not Y)
        let result = transform(skip_m, x);
        assert!(
            vec_approx_eq(result, Vec4::new(0.0, 0.0, 1.0, 0.0)),
            "After skip_y: X should become Z, got {:?}",
            result
        );

        // Y should be unchanged
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(skip_m, y);
        assert!(
            vec_approx_eq(result, y),
            "After skip_y: Y should be unchanged, got {:?}",
            result
        );
    }

    #[test]
    fn test_skip_y_xz_becomes_xw() {
        use crate::RotationPlane;
        use crate::Rotor4;
        use std::f32::consts::FRAC_PI_2;

        // Create a 90° rotation in XZ plane
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, FRAC_PI_2);
        let m = r.to_matrix();

        // After SkipY: XZ rotation becomes XW rotation
        // (Z axis in input becomes W axis in output)
        let skip_m = skip_y(m);

        // X should go to W
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result = transform(skip_m, x);
        assert!(
            vec_approx_eq(result, Vec4::new(0.0, 0.0, 0.0, 1.0)),
            "After skip_y(XZ rotation): X should become W, got {:?}",
            result
        );
    }

    #[test]
    fn test_mul_identity() {
        let a = plane_rotation(0.5, 0, 1);
        let result = mul(IDENTITY, a);
        assert!(mat_approx_eq(a, result));

        let result = mul(a, IDENTITY);
        assert!(mat_approx_eq(a, result));
    }

    #[test]
    fn test_mul_composition() {
        use std::f32::consts::FRAC_PI_4;

        // Two 45° rotations should equal one 90° rotation
        let r45 = plane_rotation(FRAC_PI_4, 0, 1);
        let r90 = plane_rotation(FRAC_PI_4 * 2.0, 0, 1);

        let composed = mul(r45, r45);

        let v = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result1 = transform(composed, v);
        let result2 = transform(r90, v);

        assert!(
            vec_approx_eq(result1, result2),
            "Composed: {:?}, Direct: {:?}",
            result1,
            result2
        );
    }

    #[test]
    fn test_get_column() {
        let m = plane_rotation(0.5, 1, 2);

        let col0 = get_column(m, 0);
        assert!(
            vec_approx_eq(col0, Vec4::new(1.0, 0.0, 0.0, 0.0)),
            "Column 0 should be X axis for YZ rotation"
        );
    }

    #[test]
    fn test_set_column() {
        let mut m = IDENTITY;
        set_column(&mut m, 1, Vec4::new(0.0, 2.0, 0.0, 0.0));
        let col1 = get_column(m, 1);
        assert!(vec_approx_eq(col1, Vec4::new(0.0, 2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_set_row() {
        let mut m = IDENTITY;
        set_row(&mut m, 2, Vec4::new(1.0, 2.0, 3.0, 4.0));
        let row2 = get_row(m, 2);
        assert!(vec_approx_eq(row2, Vec4::new(1.0, 2.0, 3.0, 4.0)));
    }

    #[test]
    fn test_negate_row() {
        let mut m = IDENTITY;
        negate_row(&mut m, 2);
        let row2 = get_row(m, 2);
        assert!(vec_approx_eq(row2, Vec4::new(0.0, 0.0, -1.0, 0.0)));
    }

    #[test]
    fn test_outer_product() {
        let a = Vec4::new(1.0, 2.0, 0.0, 0.0);
        let b = Vec4::new(3.0, 0.0, 0.0, 0.0);
        let m = outer_product(a, b);

        // Column 0 should be a * b.x = (3, 6, 0, 0)
        let col0 = get_column(m, 0);
        assert!(vec_approx_eq(col0, Vec4::new(3.0, 6.0, 0.0, 0.0)));

        // Other columns should be zero (b.y = b.z = b.w = 0)
        let col1 = get_column(m, 1);
        assert!(vec_approx_eq(col1, Vec4::ZERO));
    }

    #[test]
    fn test_add() {
        let a = plane_rotation(0.5, 0, 1);
        let result = add(a, IDENTITY);

        // Diagonal elements should be original + 1
        assert!(approx_eq(result[0][0], a[0][0] + 1.0));
        assert!(approx_eq(result[1][1], a[1][1] + 1.0));
    }

    #[test]
    fn test_scale_matrix() {
        let s = scale(Vec4::new(2.0, 3.0, 4.0, 5.0));
        let v = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let result = transform(s, v);
        assert!(vec_approx_eq(result, Vec4::new(2.0, 3.0, 4.0, 5.0)));
    }

    #[test]
    fn test_ortho_iterate_preserves_orthogonal() {
        // A rotation matrix is already orthogonal - ortho_iterate should preserve it
        let m = plane_rotation(0.7, 0, 2);
        let result = ortho_iterate(m);

        // Should still be approximately equal
        assert!(
            mat_approx_eq(m, result),
            "ortho_iterate should preserve orthogonal matrices"
        );
    }

    #[test]
    fn test_ortho_iterate_fixes_drift() {
        // Create a slightly non-orthogonal matrix by adding small errors
        let mut m = plane_rotation(0.5, 1, 2);
        m[0][0] += 0.01;
        m[1][1] -= 0.01;

        let result = ortho_iterate(m);

        // Columns should now be orthonormal
        let col0 = get_column(result, 0);
        let col1 = get_column(result, 1);
        let dot = col0.dot(col1);
        assert!(
            dot.abs() < 0.01,
            "Columns should be orthogonal after ortho_iterate, dot = {}",
            dot
        );

        // Column lengths should be near 1
        assert!((col0.length() - 1.0).abs() < 0.01);
        assert!((col1.length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_from_to_rotation_x_to_y() {
        let from = Vec4::X;
        let to = Vec4::Y;
        let m = from_to_rotation(from, to);

        // Should rotate X to Y
        let result = transform(m, from);
        assert!(
            vec_approx_eq(result, to),
            "from_to_rotation(X, Y) * X should equal Y, got {:?}",
            result
        );
    }

    #[test]
    fn test_from_to_rotation_preserves_orthogonal() {
        let from = Vec4::new(1.0, 0.0, 1.0, 0.0).normalized();
        let to = Vec4::new(0.0, 1.0, 0.0, 1.0).normalized();
        let m = from_to_rotation(from, to);

        // Result should rotate `from` to `to`
        let result = transform(m, from);
        let dot = result.dot(to);
        assert!(
            dot > 0.99,
            "from_to_rotation should rotate from to to, dot = {}",
            dot
        );
    }

    #[test]
    fn test_from_to_rotation_same_direction() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0).normalized();
        let m = from_to_rotation(v, v);

        // Should be close to identity (no rotation needed)
        let result = transform(m, v);
        assert!(
            vec_approx_eq(result, v),
            "Identity rotation failed: {:?}",
            result
        );
    }

    #[test]
    fn test_from_to_rotation_opposite() {
        // C1/T1: 180° rotation should work for all basis axes
        let cases: Vec<(Vec4, Vec4)> = vec![
            (Vec4::X, -Vec4::X),
            (Vec4::Y, -Vec4::Y),
            (Vec4::Z, -Vec4::Z),
            (Vec4::W, -Vec4::W),
        ];
        for (from, to) in &cases {
            let m = from_to_rotation(*from, *to);
            let result = transform(m, *from);
            assert!(
                vec_approx_eq(result, *to),
                "180° rotation failed: from={:?} to={:?}, got {:?}",
                from,
                to,
                result
            );
        }
    }

    #[test]
    fn test_from_to_rotation_nearly_opposite() {
        // Test stability for nearly anti-parallel vectors
        let from = Vec4::X;
        let to = Vec4::new(-1.0, 0.001, 0.0, 0.0).normalized();
        let m = from_to_rotation(from, to);
        let result = transform(m, from);
        let dot = result.dot(to);
        assert!(dot > 0.99, "Nearly opposite rotation failed, dot = {}", dot);
    }

    #[test]
    fn test_from_to_rotation_opposite_diagonal() {
        let from = Vec4::new(1.0, 1.0, 1.0, 1.0).normalized();
        let to = -from;
        let m = from_to_rotation(from, to);
        let result = transform(m, from);
        assert!(
            vec_approx_eq(result, to),
            "180° diagonal rotation failed: got {:?}, expected {:?}",
            result,
            to
        );
    }

    /// Compute determinant of a 4x4 matrix using cofactor expansion
    fn determinant(m: Mat4) -> f32 {
        let a = m[0][0];
        let b = m[1][0];
        let c = m[2][0];
        let d = m[3][0];
        let e = m[0][1];
        let f = m[1][1];
        let g = m[2][1];
        let h = m[3][1];
        let i = m[0][2];
        let j = m[1][2];
        let k = m[2][2];
        let l = m[3][2];
        let mm = m[0][3];
        let n = m[1][3];
        let o = m[2][3];
        let p = m[3][3];

        a * (f * (k * p - l * o) - g * (j * p - l * n) + h * (j * o - k * n))
            - b * (e * (k * p - l * o) - g * (i * p - l * mm) + h * (i * o - k * mm))
            + c * (e * (j * p - l * n) - f * (i * p - l * mm) + h * (i * n - j * mm))
            - d * (e * (j * o - k * n) - f * (i * o - k * mm) + g * (i * n - j * mm))
    }

    #[test]
    fn test_from_to_rotation_is_proper_rotation() {
        // T1 from review: verify det = +1 (proper rotation, not reflection)
        let cases: Vec<(Vec4, Vec4)> = vec![
            (Vec4::X, Vec4::Y),
            (Vec4::X, -Vec4::X), // anti-parallel
            (Vec4::Y, -Vec4::Y),
            (
                Vec4::new(1.0, 1.0, 1.0, 1.0).normalized(),
                Vec4::new(-1.0, -1.0, -1.0, -1.0).normalized(),
            ),
        ];
        for (from, to) in &cases {
            let m = from_to_rotation(*from, *to);
            let det = determinant(m);
            assert!(
                (det - 1.0).abs() < 0.01,
                "from_to_rotation({:?}, {:?}) should have det=1, got {}",
                from,
                to,
                det
            );
        }
    }

    #[test]
    fn test_ortho_iterate_degenerate_column() {
        // T2: Matrix with a near-zero column should return identity
        let mut m = IDENTITY;
        set_column(&mut m, 2, Vec4::new(0.0, 0.0, 1e-12, 0.0)); // Near-zero column
        let result = ortho_iterate(m);
        // Should return identity rather than a partially normalized matrix
        assert!(
            mat_approx_eq(result, IDENTITY),
            "Degenerate ortho_iterate should return identity"
        );
    }
}
