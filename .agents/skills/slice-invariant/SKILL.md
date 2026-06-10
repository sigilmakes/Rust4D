---
name: slice-invariant
description: Verify the slice-plane invariant when changing Rust4D camera, movement, physics, or rendering code. Use before and after touching Camera4D, CharacterController4D, SimulationSystem, the slice shader, or movement input handling. Explains the invariant, the regression tests, and the headless visual verification workflow.
---

# The Slice Invariant

For any world point `p`, its camera-space W coordinate `dot(camera.ana(), p - camera.position)`
must NOT change during WASD movement. Only deliberate ana movement (Q/E) and 4D
rotations may change it. The slice compute shader cuts geometry at exactly this
coordinate, so violating the invariant makes every cross-section on screen morph
while the player walks (the historical "4D movement bug").

## Rules that protect it

1. **Never scale world axes anisotropically for movement.** Speeds scale
   *semantic inputs* — slice movement (WASD) by `move_speed`, ana movement (Q/E)
   by `w_move_speed` — each uniformly, BEFORE composing in world space. See
   `CharacterController4D::apply_movement(physics, slice_dir, ana_dir)`.
2. WASD directions are projected to the horizontal XZW hyperplane (Y zeroed).
   They stay orthogonal to `ana` because ana never has a Y component (SkipY).
3. The camera matrix must stay orthogonal (rotors are normalized after compose);
   the shader relies on `transpose == inverse`.

## Numeric verification (always run this)

```bash
nix develop --command cargo test --test slice_invariant
```

Drives the full app stack (scene file, physics, controller, simulation) with a
fixed timestep and asserts zero drift across rotations, pitch, and key
combinations. Failure output prints `[MOVE]`/`[CAM]` lines with positions, ana
vectors, and per-frame drift.

## Visual verification (for rendering-path changes)

```bash
nix develop --command cargo run --example headless_protocol .scratchpad/captures
```

Renders a scripted protocol through the real slice + render pipelines into
offscreen textures — no window needed. Outputs:

- PPM frames per phase (baseline, strafe control, 45° rotation, strafe-after-
  rotation, forward, near approach). Convert: `magick f.ppm f.png`.
- `[STATE] slice_w(tesseract)=…` per capture — must stay constant within every
  WASD phase (sign of `-0.000000` is fine).
- `[CAPTURE] … triangles=N` — triangle count may change at rotations only.

What to compare across frames within a movement phase: cross-section shape and
color gradient must be constant; translation/parallax is fine. If the shape
shrinks/grows or the W color gradient shifts (e.g., purple → green), the slice
plane is drifting.

To compare against a known-good build, run the example on both revisions into
different directories and `magick a.png b.png +append cmp.png`.
