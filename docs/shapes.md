# Rust4D Shape Catalog

Rust4D renders 4D objects by slicing a **tetrahedralized 3D boundary mesh**
with the camera's 3D hyperplane. This catalog documents every built-in shape,
how it is constructed, what it looks like under slicing, and how to use it in
RON scene files.

For the math behind slicing, camera conventions, and W-depth, read
[The Mathematics of Rust4D](./4d-math.md). For the implementation details,
see `rust4d_math::Mesh4D` and `rust4d_math::primitives`.

---

## Invariants for every shape

Every production primitive is tested for:

1. **Index validity** — all tetrahedra reference valid, distinct vertices.
2. **Watertightness** — every triangular face is shared by exactly two
   tetrahedra (`Mesh4D::is_watertight`). This prevents slice cracks and
   T-junctions.
3. **Boundary 3-volume** — exact for regular polytopes, convergent from below
   for curved approximations.
4. **Visual rendering** — `cargo run --example shape_showcase` captures every
   primitive at multiple slice offsets and 4D orientations.

The renderer wants only boundary tetrahedra, not interior volume cells. This is
why the tesseract now emits 48 boundary tetrahedra rather than the older 84-tet
Kuhn complex that included 36 internal membranes.

---

## RON scene syntax

Shapes are serialized through `ShapeTemplate(type: "...")`:

```ron
shape: ShapeTemplate(type: "Hypersphere", radius: 1.25, subdivisions: 2)
```

Resolution fields have defaults, so you may omit them:

```ron
shape: ShapeTemplate(type: "Duocylinder", radius_xy: 1.0, radius_zw: 1.0)
```

All shapes are created in local space and positioned by `Transform4D`.

---

## Tesseract

**Type**: `Tesseract`  
**Parameters**: `size`  
**Cells**: 48 tetrahedra (8 cubic facets × 6 tetrahedra)

The tesseract is the 4D hypercube. At a W-aligned slice through its center it
appears as a cube. Moving the slice through W shrinks/translates the visible
cross-section depending on orientation, just as a cube passing through a 2D
plane would show changing square cross-sections.

```ron
shape: ShapeTemplate(type: "Tesseract", size: 2.0)
```

Implementation note: the boundary is induced by the 4D Kuhn triangulation and
filtered to only tetrahedra whose four vertices share a fixed coordinate bit.

---

## Hypersphere

**Type**: `Hypersphere`  
**Parameters**: `radius`, `subdivisions = 2`  
**Cells**: `16 · 8^subdivisions`

A solid 4-ball bounded by a 3-sphere (`S³`, also called a glome). Its 3D slice
is a sphere that grows to full size and shrinks away as the camera moves along
ana/kata.

```ron
shape: ShapeTemplate(type: "Hypersphere", radius: 1.25, subdivisions: 2)
```

Construction: start from the 16-cell, recursively split every tetrahedron into
8, and reproject midpoints to the sphere. The boundary 3-volume converges from
below to `2π²r³`.

Recommended subdivisions:

| Level | Cells | Use |
|-------|-------|-----|
| 0 | 16 | debugging / low-poly style |
| 1 | 128 | cheap rounded object |
| 2 | 1024 | default gameplay quality |
| 3 | 8192 | hero object |

---

## Pentachoron / 5-cell

**Type**: `Pentachoron`  
**Parameters**: `circumradius`  
**Cells**: 5 tetrahedra

The 4-simplex: the simplest regular 4-polytope and the 4D analogue of the
tetrahedron. Its boundary is five regular tetrahedra. Slices are angular and
simple, useful as a diagnostic for degenerate cases because it has the fewest
possible cells.

```ron
shape: ShapeTemplate(type: "Pentachoron", circumradius: 1.3)
```

---

## Hexadecachoron / 16-cell

**Type**: `Hexadecachoron`  
**Parameters**: `circumradius`  
**Cells**: 16 tetrahedra

The 4D orthoplex: vertices lie at `±r` on the four coordinate axes. It is the
4D analogue of the octahedron and the dual of the tesseract. Its clean
axis-aligned construction makes it a useful base mesh for hypersphere
subdivision.

```ron
shape: ShapeTemplate(type: "Hexadecachoron", circumradius: 1.35)
```

---

## Icositetrachoron / 24-cell

**Type**: `Icositetrachoron`  
**Parameters**: `circumradius`  
**Cells**: 96 tetrahedra (24 octahedral cells × 4 tetrahedra)

The 24-cell is unique to four dimensions: it has no 3D or 5D analogue. It is
self-dual and has octahedral cells. Rust4D constructs it from permutations of
`(±1, ±1, 0, 0)`, then finds the 24 octahedral cells as support sets of the
dual 24-cell and splits each octahedron into four tetrahedra.

```ron
shape: ShapeTemplate(type: "Icositetrachoron", circumradius: 1.5)
```

Use it when you want something unmistakably 4D but still cheap to render.

---

## Hexacosichoron / 600-cell

**Type**: `Hexacosichoron`  
**Parameters**: `circumradius`  
**Cells**: 600 tetrahedra

The 600-cell is the 4D analogue of the icosahedron and the most intricate
regular primitive currently shipped. It has 120 vertices and 600 tetrahedral
cells.

```ron
shape: ShapeTemplate(type: "Hexacosichoron", circumradius: 1.2)
```

Construction: vertices are the 120 unit quaternions of the binary icosahedral
group:

- 8 coordinate-axis points, permutations of `(±1, 0, 0, 0)`
- 16 half-points `(±½, ±½, ±½, ±½)`
- 96 golden points from even permutations of `(±φ/2, ±1/2, ±1/(2φ), 0)`

The edge graph uses distance `r/φ`; the 600 cells are exactly the 4-cliques of
that graph. Tests pin 120 vertices, 600 cells, watertightness, and the expected
regular-tetrahedron boundary volume.

---

## Spherinder

**Type**: `Spherinder`  
**Parameters**: `radius`, `half_height`, `subdivisions = 2`  
**Cells**: `5 ×` the number of icosphere triangles

A 3-ball extruded along W: `B³ × segment`. It is the most literal 4D analogue
of a cylinder. A W-aligned slice shows a sphere that stays approximately
constant-sized through the tube, then disappears at the caps.

```ron
shape: ShapeTemplate(type: "Spherinder", radius: 1.05, half_height: 1.25)
```

Boundary pieces:

- two 3-ball caps,
- an `S² × segment` tube built from triangular prisms.

---

## Cubinder

**Type**: `Cubinder`  
**Parameters**: `radius`, `half_size`, `segments = 24`  
**Cells**: `18 × segments`

A disk in XY crossed with a square in ZW: `D² × square`. It blends curved and
flat direction pairs, making it useful for testing how slicing behaves when a
shape has both cylindrical and prismatic structure.

```ron
shape: ShapeTemplate(type: "Cubinder", radius: 1.05, half_size: 0.9, segments: 32)
```

Boundary pieces:

- `S¹ × square` curved shell,
- `D² ×` each square edge.

---

## Duocylinder

**Type**: `Duocylinder`  
**Parameters**: `radius_xy`, `radius_zw`, `segments = 24`  
**Cells**: `6 × segments²`

A product of two disks: `D² × D²`. The boundary consists of two solid-torus
pieces, `S¹ × D²` and `D² × S¹`, glued along a Clifford torus `S¹ × S¹`.
This is one of the most characteristically 4D objects in the engine.

```ron
shape: ShapeTemplate(type: "Duocylinder", radius_xy: 1.0, radius_zw: 1.0, segments: 32)
```

At different 4D rotations it produces dramatically different 3D slices,
including tube-like and ball-like sections. Use `shape_showcase` to inspect it
from identity, XW, and ZW orientations.

---

## Visual verification

Generate captures for the entire catalog:

```bash
nix develop --command cargo run --example shape_showcase .scratchpad/captures-gallery
```

Expected output:

- 81 PPM files (`9 shapes × 3 offsets × 3 orientations`)
- zero zero-triangle captures
- a nonzero triangle count logged for every frame

Representative contact sheets from this branch are in `.scratchpad/` during
development; they are intentionally gitignored.
