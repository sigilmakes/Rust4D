# Changelog

All notable changes to Rust4D are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
Conventional Commits.

## Unreleased — Engine Expansion

### Added

- General `Mesh4D` tetrahedral mesh type with merge, transform, weld,
  validation, Gram-determinant cell volumes, and watertightness checks.
- Full primitive catalog:
  - tesseract (fixed boundary-only tetrahedralization),
  - hypersphere,
  - regular 5-cell, 16-cell, 24-cell, 600-cell,
  - spherinder,
  - cubinder,
  - duocylinder.
- RON `ShapeTemplate` variants for all primitives, including defaulted
  resolution fields.
- Shape-aware physics collider hints for scene instantiation.
- `scenes/gallery.ron` with all primitive exhibits.
- `examples/shape_showcase.rs`, an offscreen visual verification harness for
  the full primitive catalog.
- Two-sided Blinn-Phong lighting with specular highlights, point lights, and
  distance fog.
- `rust4d_input::ActionMap` and `CameraAction` for semantic camera bindings.
- Lua ECS entity handle bit round-tripping via `world.entity_from_bits(bits)`
  and `entity:equals(other)`.
- GitHub Actions CI: formatting, clippy with `-D warnings`, rustdoc with
  `-D warnings`, and workspace tests.
- Project skills for 4D geometry, headless visual verification, and production
  readiness.
- Shape catalog documentation (`docs/shapes.md`).

### Changed

- `CameraController` now processes semantic actions from an `ActionMap` while
  preserving the legacy keyboard defaults.
- Workspace is now `rustfmt` clean.
- Rendering disables back-face culling because slice-generated triangle winding
  is not stable across all marching-tetrahedra cases.

### Fixed

- Tesseract geometry now emits only the 48 boundary tetrahedra. The previous
  84-tetrahedron Kuhn-derived surface included 36 internal membranes, wasting
  GPU slice work and producing spurious interior walls when viewed from inside.

## PR #15 — 4D Rendering Debug Fix

### Added

- `tests/slice_invariant.rs`, an end-to-end invariant suite for camera,
  physics, controller, and simulation movement.
- `examples/headless_protocol.rs`, an offscreen GPU visual verification harness
  for slice-plane drift and projection issues.
- `flake.nix` dev shell with Rust, Vulkan wiring, lavapipe, and image tools.
- `docs/4d-math.md`, documenting rotors, SkipY, slicing, projection, movement
  invariants, and matrix conventions.
- Minimal `AGENTS.md` plus progressive-disclosure skills.

### Fixed

- Long-standing 4D movement bug: WASD movement after 4D rotation drifted across
  the slice plane because world axes were scaled anisotropically. Speeds now
  scale semantic movement inputs instead.
- Perspective matrix depth range now matches wgpu `[0, 1]` rather than OpenGL
  `[-1, 1]`.
- `rotate_w` and `rotate_xw` now operate in their documented 4D planes after
  SkipY remapping.
- Removed dead `camera_eye` from `SliceParams`.

### Quality

- Workspace clippy-clean and rustdoc-clean at merge time.
- Windowed and headless visual verification performed.
