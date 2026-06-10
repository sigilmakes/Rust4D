---
name: 4d-conventions
description: Rust4D math, matrix, rotation, and GPU conventions. Use when editing rust4d_math, Camera4D, WGSL shaders, projection matrices, or anything passing matrices between Rust and the GPU. Prevents convention-mismatch bugs (row/column major, depth range, rotation planes).
---

# Rust4D Conventions

## Matrices

- `Mat4 = [[f32; 4]; 4]` is **column-major**: `m[col][row]`. `mat4::transform`
  computes `M * v`; `mat4::mul(a, b)` applies `b` first.
- Raw memory order matches WGSL `mat4x4<f32>` (first inner array = first
  column), so matrices upload without conversion.
- `camera.camera_matrix()` is **camera→world**. The slice shader computes
  `transpose(camera_mat) * (world_pos - camera_pos)` for world→camera; this is
  only valid while the matrix stays orthogonal.

## Depth

- Projection uses wgpu's **[0, 1]** depth range (near→0, far→1), NOT OpenGL's
  [-1, 1]. Pinned by `test_perspective_depth_range_is_wgpu_zero_to_one`.

## 4D rotations (SkipY)

- Camera orientation = `skip_y(rotation_4d) * pitch`. Pitch (YZ) is stored
  separately and clamped; `rotation_4d` is a Rotor4 acting in XZW via SkipY,
  so the Y (gravity) axis is never affected.
- SkipY maps the 3D rotation axes (X, Y, Z) onto 4D axes (X, Z, W). Therefore:
  - pre-SkipY YZ plane → 4D **ZW** (`rotate_w`: forward tilts toward +W)
  - pre-SkipY XZ plane → 4D **XW** (`rotate_xw`: right tilts toward +W)
  - pre-SkipY XY plane → 4D **XZ** (yaw)
- Normalize rotors after composing (`.compose(&r).normalize()`).

## Slicing pipeline

- `SliceParams` is `#[repr(C)]` + bytemuck; field order and 16-byte alignment
  must match the WGSL struct exactly. A struct-size unit test pins the layout —
  update both sides together.
- The compute shader slices at camera-space W = `slice_w` (`camera.slice_offset`,
  a camera-space offset, NOT a world W coordinate).
- Triangle counter counts vertices (×3) for DrawIndirect.

## Scenes / config

- Scenes are RON files in `scenes/`; config is TOML in `config/`
  (`default.toml` < `user.toml` < env vars, via figment).
- Default speeds: `move_speed = 3.0`, `w_move_speed = 2.0` — anisotropic by
  design; see the `slice-invariant` skill for why these must scale inputs,
  never world axes.
