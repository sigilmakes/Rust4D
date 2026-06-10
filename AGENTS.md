# Rust4D

A 4D game engine in Rust: renders 3D cross-sections of 4D geometry via a GPU
compute pipeline, with 4D physics, Lua scripting, spatial audio, and an egui HUD.
Cargo workspace: app crate in `src/`, engine crates in `crates/rust4d_*`.

## Requirements

- Build and test inside the nix dev shell: `nix develop --command cargo test --workspace`
- Keep `cargo clippy --workspace --all-targets` warning-free
- Before changing camera, movement, physics, or slicing code, load the
  `slice-invariant` skill; for math/shader conventions, load `4d-conventions`
- `tests/slice_invariant.rs` must pass — it guards the engine's core invariant
