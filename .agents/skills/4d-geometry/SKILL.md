---
name: 4d-geometry
description: Build or modify Rust4D primitives and tetrahedral meshes. Use when touching rust4d_math::Mesh4D, primitives, Tesseract4D, ShapeTemplate geometry variants, or any code that creates 4D tetrahedra for slicing.
---

# 4D Geometry Workflow

Rust4D renders **3D slices of 4D boundary meshes**. The renderer wants a closed
3-manifold embedded in 4D, decomposed into tetrahedra. If a primitive is not
structurally watertight, the GPU will show cracks, T-junctions, missing faces,
or interior membranes when sliced.

## Mandatory checks for every primitive

1. Construct as `Mesh4D` where possible.
2. Run `mesh.validate()` — catches out-of-bounds and repeated cell indices.
3. Run `mesh.is_watertight()` — every triangular face must be shared by exactly
   two tetrahedra.
4. Pin expected cell counts in tests.
5. Pin total boundary 3-volume using `mesh.surface_volume()`:
   - exact for regular polytopes,
   - convergent-from-below for curved approximations.
6. Render with `cargo run --example shape_showcase .scratchpad/captures-gallery`
   and inspect captures.

## Seam rule

For composite curved shapes, shared vertices must share **global indices before
splitting prisms**. Use the `VertexPool` pattern from `primitives/curved.rs`.
Do not rely on post-hoc welding to fix seams: welding after splitting does not
fix mismatched quad diagonals.

Use `primitives::extrude::split_prism`, which applies the Dompierre
lowest-global-index rule so neighboring prisms choose the same diagonals on
shared quad faces.

## Tesseract warning

The tesseract must emit only its 48 boundary tetrahedra. The old 84-tet Kuhn
surface included 36 internal membranes, wasting slice work and rendering
spurious interior walls. Any future tesseract edit must keep:

```rust
assert_eq!(Tesseract4D::new(2.0).tetrahedra().len(), 48);
assert!(Mesh4D::from(&tess as &dyn ConvexShape4D).is_watertight());
```

## Related docs

- `docs/4d-math.md` — slicing and matrix conventions
- `docs/shapes.md` — shape catalog (when updated)
- `.agents/skills/headless-visual-verification` — GPU capture workflow
