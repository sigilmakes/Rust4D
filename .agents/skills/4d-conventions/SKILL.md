---
name: 4d-conventions
description: Rust4D math, matrix, rotation, and GPU layout conventions. Use when editing rust4d_math, Camera4D, WGSL shaders, projection matrices, GPU uniform structs, or anything passing matrices between Rust and the GPU. Prevents convention-mismatch bugs (row/column major, depth range, rotation planes, struct layout).
---

# Rust4D Conventions

Most of this engine's historical bugs were not logic errors — they were
**convention mismatches** between layers: a matrix interpreted with the wrong
majority, a depth range copied from OpenGL, a rotation plane that didn't match
its name. This skill is the working checklist; the full derivations live in
`docs/4d-math.md`.

## Matrices: column-major, everywhere

```
Mat4 = [[f32; 4]; 4]      m[col][row]
```

- `m[i]` is the **i-th column** — the image of basis vector i. So
  `camera_matrix()[3]` is the camera's ana axis expressed in world space.
- `mat4::transform(m, v)` computes `M·v` (column-vector convention).
- `mat4::mul(a, b)` = `A·B`, meaning **b is applied first**. Camera
  composition `mul(skip_y(rot), pitch)` applies pitch, then the 4D rotation.
- Raw memory order matches WGSL `mat4x4<f32>` exactly (first inner array =
  first column), so matrices upload via bytemuck **without transposition**.
  If you see `transpose()` in a shader it's a deliberate
  inverse-of-orthogonal-matrix, not a layout conversion. Don't "fix" it.

## Depth: wgpu [0, 1], not OpenGL [−1, 1]

`perspective_matrix` maps near→0, far→1 (D3D/Metal/WebGPU convention). The
OpenGL convention puts the near half of the frustum at negative depth, which
wgpu clips away. Pinned by `test_perspective_depth_range_is_wgpu_zero_to_one`
in `render_pipeline.rs`. Be suspicious of any projection code copied from
OpenGL-era references — check the third column.

## 4D rotation: SkipY and the plane mapping

The camera is `skip_y(rotation_4d) * pitch` — pitch is a separate clamped
float (YZ plane), and the rotor's 3D rotation is remapped by SkipY onto the
XZW subspace so the Y (gravity) axis is **never** affected:

```
3D rotor axes:  X → 4D X        Y → 4D Z        Z → 4D W
```

This means the plane you pass to `Rotor4::from_plane_angle` is NOT the 4D
plane the camera rotates in. The authoritative mapping (these were swapped
once and shipped wrong for months — tests now pin them):

| Camera method | Rotor plane (pre-SkipY) | Actual 4D plane | Effect |
|---------------|------------------------|-----------------|--------|
| yaw           | XY | XZ | turn left/right |
| `rotate_w`    | YZ | **ZW** | forward (−Z) tilts toward +W |
| `rotate_xw`   | XZ | **XW** | right (+X) tilts toward +W |

Rules:
- **Always normalize after composing**: `r.compose(&r2).normalize()`.
  Drifted rotors → non-orthogonal matrices → the shader's
  `transpose(camera_mat)` is no longer the inverse → subtle rendering skew.
- Never construct rotations involving the 4D Y axis (YW, XY-in-4D, YZ-in-4D)
  for the camera — that's what SkipY exists to prevent.

## GPU uniform structs: three places must agree

`SliceParams` and `RenderUniforms` exist in three forms that must match
field-for-field: the Rust struct (`#[repr(C)]`, bytemuck, explicit padding to
16-byte alignment), the WGSL struct in the shader, and the struct-size unit
test in `types.rs`. When you change one:

1. Update the Rust struct (mind 16-byte alignment of vec4/mat4 fields).
2. Update the WGSL struct in `slice_tetra.wgsl` / `render.wgsl`.
3. Update the size assertion test.

If they disagree the GPU reads garbage from the wrong offsets — typically
manifesting as wild geometry or nothing rendering, with no error message.

Other slicing-pipeline facts worth knowing:
- The compute shader slices at camera-space W = `slice_w`, which is
  `camera.slice_offset` — a **camera-relative** offset (normally 0, "slice
  through my own W position"), NOT a world W coordinate.
- The atomic counter counts **vertices** (increments of 3) because
  DrawIndirect consumes a vertex count.

## Scenes and config

- Scenes: RON files in `scenes/` (entities, shapes, materials, gravity,
  player spawn). Config: TOML in `config/` — `default.toml` < `user.toml` <
  `RUST4D_*` env vars, merged by figment.
- Default speeds are anisotropic **by design** (`move_speed = 3.0`,
  `w_move_speed = 2.0`): walking is faster than deliberate 4D travel. That is
  safe only because speeds scale semantic inputs, never world axes — see the
  `slice-invariant` skill before touching anything in that path.

## When you fix a convention bug

Add a test that pins the convention (so it can't silently regress), then
update `docs/4d-math.md` and, if the working rule changed, this skill.
