# The Mathematics of Rust4D

This document explains the math the engine is built on, from first principles.
It's aimed at developers who know 3D graphics but have never worked in 4D.
Read it before touching `rust4d_math`, `Camera4D`, the movement code, or the
slicing shader — most historical bugs in this engine came from convention
mismatches between these layers.

Related reading:
- [Developer Guide](./developer-guide.md) — architecture and algorithms
- [Shape Catalog](./shapes.md) — built-in primitives, construction math, scene syntax
- [Getting Started](./getting-started.md) — intuition for 4D space
- `.agents/skills/` — condensed working rules for these conventions

---

## Table of Contents

1. [4D Space and the W Axis](#4d-space-and-the-w-axis)
2. [Rotations Happen in Planes, Not Around Axes](#rotations-happen-in-planes-not-around-axes)
3. [Rotors: 4D Rotations Without Gimbal Hell](#rotors-4d-rotations-without-gimbal-hell)
4. [The Camera Model: Pitch + SkipY](#the-camera-model-pitch--skipy)
5. [Slicing: How 4D Becomes 3D](#slicing-how-4d-becomes-3d)
6. [The Slice Invariant](#the-slice-invariant)
7. [Movement Math: Scaling Inputs, Not Axes](#movement-math-scaling-inputs-not-axes)
8. [Projection and Depth](#projection-and-depth)
9. [Matrix Conventions Cheat Sheet](#matrix-conventions-cheat-sheet)

---

## 4D Space and the W Axis

A point in 4D space is `Vec4 { x, y, z, w }`. The first three axes are the
familiar ones; **W is a fourth spatial axis**, perpendicular to all of them.
It is not time — Rust4D simulates Euclidean 4-space, like
[Miegakure](https://miegakure.com/) or 4D Golf, not spacetime.

Useful intuitions, by analogy with a 2D being living on a sheet of paper:

- A 2D being sees 1D cross-sections of 2D shapes. We are 3D beings looking at
  **3D cross-sections of 4D shapes**.
- If a 3D sphere passes through the paper, the 2D being sees a point grow into
  a circle and shrink away. When a 4D hypersphere passes through our 3D slice,
  we see a sphere grow and shrink.
- The 2D being can't point "out of the page", but the direction exists.
  Likewise **ana** (+W) and **kata** (−W) are real directions we can't point at.

The engine's standard names:

| Direction | Axis | Camera accessor |
|-----------|------|-----------------|
| right     | +X   | `camera.right()` |
| up        | +Y   | `camera.up()` |
| forward   | −Z   | `camera.forward()` |
| ana       | +W   | `camera.ana()` |

Y is special: it is the **gravity axis**, and the camera system is built so
that no 4D rotation ever tilts it (see [SkipY](#the-camera-model-pitch--skipy)).

---

## Rotations Happen in Planes, Not Around Axes

In 3D we say "rotate around the Z axis", but that's a coincidence of three
dimensions: a rotation actually happens **in a plane** (the XY plane, in that
example), and in 3D every plane happens to have exactly one perpendicular axis
left over. In 4D a plane has **two** perpendicular directions left over, so
"rotation axis" stops making sense. We always name rotations by their plane.

4D has six coordinate planes, hence six independent rotation directions:

| Plane | 3D meaning | 4D meaning in Rust4D |
|-------|-----------|----------------------|
| XY    | roll      | (used pre-SkipY for yaw) |
| XZ    | yaw       | (used pre-SkipY for XW rotation) |
| YZ    | pitch     | (used pre-SkipY for ZW rotation) |
| XW    | —         | right axis tilts into the 4th dimension |
| YW    | —         | never used (would tilt gravity) |
| ZW    | —         | forward axis tilts into the 4th dimension |

A rotation in the ZW plane by angle θ maps:

```
z' =  z·cosθ − w·sinθ
w' =  z·sinθ + w·cosθ        (x and y untouched)
```

This is exactly the familiar 2D rotation formula, applied to the (z, w) pair.
`mat4::plane_rotation(angle, p1, p2)` builds these matrices for any axis pair.

A consequence that surprises 3D programmers: in 4D, two rotations can be
**completely independent** (e.g. XY and ZW share no axes), and a general 4D
rotation is a composition of two such independent plane rotations. This is why
4D rotation needs more than a quaternion.

---

## Rotors: 4D Rotations Without Gimbal Hell

`Rotor4` is the 4D generalization of a quaternion, built from geometric
algebra. Where a quaternion has 4 components (1 scalar + 3 bivector), a 4D
rotor has **8 components**: 1 scalar, 6 bivector (one per rotation plane:
`b_xy, b_xz, b_xw, b_yz, b_yw, b_zw`), and 1 quadvector (`p`, the XYZW
4-volume term, needed for compositions of independent rotations).

What you need to know to use them:

- `Rotor4::from_plane_angle(plane, angle)` creates a rotation in one plane.
- `r1.compose(&r2)` chains rotations (apply `r2` first, then `r1` — same
  convention as matrix multiplication).
- **Always normalize after composing**: `r1.compose(&r2).normalize()`.
  Floating-point drift de-normalizes rotors, and a de-normalized rotor produces
  a non-orthogonal matrix — which silently breaks the renderer's
  `transpose == inverse` assumption (see [Slicing](#slicing-how-4d-becomes-3d)).
- `rotor.to_matrix()` converts to a `Mat4` for the camera/GPU path. The matrix
  of a normalized rotor is orthogonal: its columns are the rotated basis
  vectors, all unit length and mutually perpendicular.

The unit tests in `rotor4.rs` verify orthogonality is preserved through long
composition chains — if you add rotor operations, extend them.

---

## The Camera Model: Pitch + SkipY

A naive 4D camera (one rotor for everything) has a fatal usability flaw:
composing 4D rotations tilts the Y axis, so after looking around in 4D,
"up" is no longer up and walking forward drifts vertically. Engine4D solved
this with an architecture Rust4D copies:

**The camera orientation is two separate pieces:**

```rust
camera_matrix() = skip_y(rotation_4d.to_matrix()) * pitch_matrix
```

1. **`pitch`** — a plain YZ-plane angle, stored as a float, clamped to ±89°.
   Mouse-up/down only ever changes this. It is applied first.
2. **`rotation_4d`** — a `Rotor4` holding yaw and all 4D rotation, applied
   through the **SkipY** remapping.

### SkipY

`mat4::skip_y(m)` takes a 3D rotation matrix (acting on axes 0, 1, 2) and
re-indexes it to act on axes **0, 2, 3** — that is, X, Z, W — leaving axis 1
(Y) exactly alone:

```
3D rotation axes:   X → 4D X
                    Y → 4D Z
                    Z → 4D W
```

So the rotor is "a 3D rotation", but the three dimensions it rotates are the
horizontal-and-W subspace XZW. **Y is structurally untouchable.** Gravity
always points down, no matter what 4D acrobatics the player performs, and
`camera.ana()` never has a Y component (the proof is one line: SkipY's Y row
and column are identity).

### The plane mapping table

Because of SkipY, the plane you pass to the rotor is *not* the plane the
camera rotates in. This caused real bugs (the functions were swapped for
months), so here is the authoritative table:

| Camera method | Pre-SkipY rotor plane | Actual 4D plane | Effect |
|---------------|----------------------|-----------------|--------|
| `rotate_3d` (yaw) | XY | XZ | turn left/right |
| `rotate_3d` (pitch) | — (separate float) | YZ | look up/down |
| `rotate_w`    | YZ | **ZW** | forward tilts toward +W |
| `rotate_xw`   | XZ | **XW** | right tilts toward +W |

If you change anything here, `test_ana_changes_after_4d_rotation` and friends
in `camera4d.rs` pin the expected mappings.

---

## Slicing: How 4D Becomes 3D

The renderer never projects 4D to screen directly. Every frame, a compute
shader (`slice_tetra.wgsl`) intersects all 4D geometry with the camera's
**slice hyperplane** and emits ordinary 3D triangles, which a second pipeline
rasterizes normally.

### Step 1: tetrahedral meshes

3D renderers triangulate surfaces; a 4D renderer **tetrahedralizes volumes**.
Every shape (tesseract, hyperplane floor, …) is decomposed into 4D tetrahedra
(4 vertices each). A tesseract decomposes into 8 cubic cells × 6 tetrahedra
plus internal padding — see `rust4d_math::tesseract`.

### Step 2: transform to camera space

For each vertex, the shader computes its position relative to the camera:

```wgsl
let relative = world_pos - camera_pos;            // translate
let cam_space = transpose(camera_mat) * relative; // rotate world→camera
```

`camera_mat` is camera→world; because it's orthogonal, its transpose is its
inverse. (This is the assumption that rotor normalization protects.)

### Step 3: slice at constant W

In camera space, the camera's ana axis *is* the W axis. The shader keeps the
part of each tetrahedron at `cam_space.w == slice_w`, where `slice_w` is
`camera.slice_offset` — a **camera-relative** offset, normally 0, meaning
"slice through my own W position".

Slicing a tetrahedron with a hyperplane is marching-tetrahedra: classify the
4 vertices as above/below (16 cases), interpolate crossing edges (3 or 4 of
them), emit 1 or 2 triangles. The lookup tables are documented in the
[Developer Guide](./developer-guide.md#key-algorithms).

The mental model: **the visible world is the intersection of 4D space with a
3D hyperplane glued to the camera, perpendicular to the camera's ana axis.**
Rotating in ZW/XW tilts that hyperplane through 4D; moving along ana shifts
it; and *nothing else* may move it — which brings us to:

---

## The Slice Invariant

> For any world point `p`, its camera-space W coordinate
> `dot(camera.ana(), p − camera.position)`
> must not change during WASD movement. Only deliberate ana movement (Q/E)
> and 4D rotation may change it.

Why it matters: if WASD movement changes that quantity, the slice hyperplane
sweeps through every object in the scene while the player merely walks, and
all cross-sections visibly morph — the engine's most notorious historical bug.

Why walking is *allowed* to move through world-W at all: after a ZW rotation,
camera-forward legitimately has a W component. Walking "forward" then moves
through world X/Z **and** W simultaneously — that's the point of a 4D game.
The invariant doesn't say "don't move in W"; it says "move only **within**
the slice hyperplane", i.e. perpendicular to ana. Movement within the
hyperplane changes which part of 4D space you're in, but not which slice of
each *object* you see (their cross-sections translate on screen with parallax,
exactly like normal 3D walking).

Two structural facts make WASD movement safe:

1. WASD directions are built from `forward`/`right` projected to the
   horizontal XZW hyperplane (Y zeroed, renormalized). Both remain orthogonal
   to `ana`, because ana is Y-free and rotation preserves orthogonality.
2. Speeds scale those direction vectors **uniformly** (next section).

Guarded by `tests/slice_invariant.rs`, which drives the real app stack —
scene file, physics, character controller, camera, simulation system — with a
fixed timestep and asserts zero drift. Run it after touching anything in the
camera/movement/physics path. For rendering-path changes, also run the visual
harness: `cargo run --example headless_protocol` (see the
[Developer Guide](./developer-guide.md#testing-strategy)).

---

## Movement Math: Scaling Inputs, Not Axes

The subtle bug class this engine teaches: **anisotropic scaling in the wrong
coordinate frame breaks direction invariants.**

The engine has two speeds: `move_speed` (WASD) and `w_move_speed` (Q/E).
The original implementation scaled the world-space velocity per axis —
X, Y, Z by `move_speed` and W by `w_move_speed`. Looks harmless; isn't.
After a 45° ZW rotation:

```
forward = (0, 0, −cosθ, −sinθ)         ana = (0, 0, −sinθ, cosθ)
forward · ana = 0                       ✓ orthogonal — no morphing

scaled  = (0, 0, −3cosθ, −2sinθ)        (per-axis scaling)
scaled · ana = 3cosθsinθ − 2sinθcosθ = sinθcosθ ≈ 0.5   ✗ drift!
```

Scaling different axes by different factors **rotates** any vector not aligned
with an axis. The fix: scale each *semantic input* uniformly, then compose:

```rust
velocity = slice_dir * move_speed      // WASD direction, ⊥ ana, scaled uniformly
         + ana_dir   * w_move_speed;   // Q/E direction, ∥ ana, scaled uniformly
```

Uniform scaling preserves direction, so `slice_dir` stays orthogonal to ana at
any speed ratio. This is why `CharacterController4D::apply_movement` takes two
vectors, and why you should be suspicious of any future code that multiplies
world-space components by different constants.

---

## Projection and Depth

After slicing, the 3D triangles are already in camera space, so the render
pipeline's view matrix is identity and only a projection matrix is applied.

The projection maps the view frustum to wgpu's clip space, which uses the
**[0, 1] depth range** (Direct3D/Metal/WebGPU convention):

```
depth(z = −near) = 0        depth(z = −far) = 1
```

OpenGL's [−1, 1] convention is wrong here: it pushes the near half of the
frustum to negative depth, which the rasterizer clips away. The engine
shipped with that bug for a while; `test_perspective_depth_range_is_wgpu_zero_to_one`
pins the correct mapping. If you ever port projection code from an OpenGL
source, check the third column twice.

---

## Matrix Conventions Cheat Sheet

The single most bug-prone area. Memorize this or keep it open:

```
Mat4 = [[f32; 4]; 4]      m[col][row]   — COLUMN-major
```

- `m[i]` is the **i-th column** = the image of the i-th basis vector.
  `camera_matrix()[3]` is the ana axis in world space.
- `mat4::transform(m, v)` computes `M·v` (column-vector convention).
- `mat4::mul(a, b)` = `A·B`: **b applies first**.
- Memory layout matches WGSL `mat4x4<f32>` exactly (first inner array = first
  column), so structs upload via bytemuck without any transposition. If you
  see a `transpose()` in a shader, it's a deliberate inverse-of-orthogonal,
  not a layout fix.
- GPU uniform structs (`SliceParams`, `RenderUniforms`) are `#[repr(C)]` with
  explicit padding to 16-byte alignment, mirrored field-for-field in WGSL.
  Struct-size unit tests pin the layout; update Rust, WGSL, and the test
  together or the GPU reads garbage.

---

*If you fix a bug rooted in any convention on this page, add a test that pins
the convention, then update this page. The next developer is reading this for
the same reason you are.*
