---
name: slice-invariant
description: Verify the slice-plane invariant when changing Rust4D camera, movement, physics, or rendering code. Use before and after touching Camera4D, CharacterController4D, SimulationSystem, the slice shader, projection, or movement input handling. Explains the invariant, why it breaks, the regression tests, and the headless visual verification workflow.
---

# The Slice Invariant

## What you're protecting

Rust4D shows the player a 3D cross-section of the 4D world: a hyperplane glued
to the camera, perpendicular to the camera's **ana** axis (its 4th-dimension
"forward"). Everything you see is geometry intersected with that hyperplane.

The invariant:

> For any world point `p`, the value `dot(camera.ana(), p - camera.position)`
> — its camera-space W coordinate, the thing the slice shader cuts at —
> must NOT change while the player moves with WASD.
> Only deliberate ana movement (Q/E) and 4D rotations may change it.

If the invariant breaks, the slice hyperplane sweeps through every object
while the player merely walks, and all cross-sections visibly morph — shapes
shrink, change color, and vanish. This was the engine's most notorious bug,
and it survived multiple review rounds because it only appears when movement,
physics, and rendering interact. **No single-module unit test can catch it.**

Background reading: `docs/4d-math.md`, sections "The Slice Invariant" and
"Movement Math" — including the worked example of how the original bug
(per-world-axis speed scaling) produced 0.5 units/sec of drift at 45° rotation.

## The rules that keep it true

1. **Never scale world-space axes anisotropically for movement.** The two
   speeds (`move_speed`, `w_move_speed`) scale *semantic inputs* — the WASD
   slice-direction and the Q/E ana-direction — each uniformly, BEFORE they're
   combined. That's why `CharacterController4D::apply_movement(physics,
   slice_dir, ana_dir)` takes two vectors. If you ever see
   `velocity.w * something_different_from_xyz`, that's the bug reborn.
2. WASD directions come from `forward`/`right` projected to the horizontal
   XZW hyperplane (Y zeroed, renormalized). These stay orthogonal to `ana`
   because ana never has a Y component (SkipY construction — see the
   `4d-conventions` skill) and rotations preserve orthogonality.
3. The camera matrix must remain orthogonal — always `.normalize()` rotors
   after `.compose()`. The shader computes world→camera as
   `transpose(camera_mat)`, which is only the inverse while orthogonal.

## How to verify — numeric (always)

```bash
nix develop --command cargo test --test slice_invariant
```

This drives the **real app stack** (scene file, physics world, camera
controller, character controller, simulation system) with fixed timesteps via
`SimulationSystem::update_with_dt`, and asserts zero drift across: no
rotation, 45° ZW rotation, combined ZW+XW+yaw, and pitched movement. It also
sanity-checks that Q/E *does* change the slice and WASD actually moves the
player — so you can't "fix" drift by deleting movement.

On failure, the output prints `[MOVE]`/`[CAM]` lines with camera positions,
ana vectors, and per-frame drift values. Drift grows linearly with time; the
rate tells you the mechanism (e.g. `(move_speed − w_move_speed)·sinθcosθ` for
anisotropic scaling).

## How to verify — visual (for rendering-path changes)

```bash
nix develop --command cargo run --example headless_protocol .scratchpad/captures
```

Renders a scripted protocol through the real slice + render pipelines into an
offscreen texture — no window, no compositor, works in CI. Phases: baseline →
strafe control → 45° 4D rotation → strafe-after-rotation → forward → near
approach. Each capture saves a PPM frame and prints:

```
[CAPTURE] 11-strafe-after-rotation.ppm cam=(-1.061,-1.500,5.000,0.707) triangles=5856
[STATE]   11-strafe-after-rotation: slice_w(tesseract)=0.000000 tris=5856
```

Reading the results:

- `slice_w(tesseract)` must stay constant within every WASD phase
  (`-0.000000` is fine). If it counts up or down, that's drift.
- In the frames, the cross-section's **shape and color gradient** must stay
  constant within a phase. Translation and parallax are fine. A shrinking
  shape or a color shift (e.g. purple → green, which is the W position
  gradient) means the slice plane is sweeping through the object.
- Convert for viewing: `magick frame.ppm frame.png`. Compare two revisions:
  run the example on each into different directories, then
  `magick a/12.png b/12.png +append cmp.png`.

## Lessons from history

The bug this guards against was analyzed *theoretically* by many sessions and
review passes, which produced two wrong fixes (one zeroed `accel.w` in world
space — that makes drift worse, because the slice plane isn't aligned with
world W after rotation). It was only diagnosed when someone ran the code with
instrumentation and read the numbers. When in doubt: instrument, run, read,
look. Don't argue from the math alone — verify against it.
