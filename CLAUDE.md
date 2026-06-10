# About This Project

Rust4D is a 4D game engine written in Rust. It renders 3D cross-sections of
4D geometry via a GPU compute pipeline (tetrahedra slicing), with 4D physics,
Lua scripting, spatial audio, and an egui HUD.

# Repository Map

```
Rust4D/
├── src/                        # Application crate (lib + bin)
│   ├── main.rs                 # Windowed app entry point (winit)
│   ├── lib.rs                  # Exports config, input, systems for tests/harnesses
│   ├── config.rs               # TOML config (figment: defaults < files < env)
│   ├── input/                  # Input mapping (raw events → semantic actions)
│   └── systems/                # Render, simulation, window, geometry systems
├── crates/
│   ├── rust4d_math/            # Vec4, Rotor4, Mat4, shapes, rays
│   ├── rust4d_core/            # ECS world, scenes (RON), transforms, assets
│   ├── rust4d_render/          # wgpu pipelines, Camera4D, particles, sprites, HUD
│   ├── rust4d_physics/         # 4D rigid bodies, colliders, raycasts, triggers
│   ├── rust4d_input/           # CameraController (WASD + Q/E + mouse)
│   ├── rust4d_game/            # CharacterController4D, events, FSM, tweens
│   ├── rust4d_scripting/       # Lua 5.4 (mlua), sandboxed, hot-reload
│   └── rust4d_audio/           # kira-based 4D spatial audio
├── examples/                   # Runnable demos + headless_protocol (visual verification)
├── tests/                      # Integration tests (slice_invariant.rs is the key one)
├── scenes/                     # RON scene definitions
├── config/                     # default.toml + user.toml
├── flake.nix                   # Nix dev shell (toolchain, Vulkan, screenshot tools)
├── scratchpad/                 # Symlink to shared Obsidian vault (see ~/.pi/agent/AGENTS.md)
└── .scratchpad/                # Local throwaway temp files (gitignored)
```

# Development

Use the nix dev shell for builds — it provides the toolchain, Vulkan ICD
wiring, and imaging tools:

```bash
nix develop --command cargo test --workspace
```

# The Slice Invariant (read before touching camera/movement/rendering)

For any world point `p`, its camera-space W coordinate `dot(ana, p - cam_pos)`
must NOT change during WASD movement — only deliberate Q/E movement and 4D
rotations may change it. If this breaks, every cross-section on screen morphs
while walking (the historical "4D movement bug").

- Guarded by `tests/slice_invariant.rs` (drives the full app stack).
- Never scale world axes anisotropically for movement; scale semantic inputs
  (slice movement vs ana movement) — see `CharacterController4D::apply_movement`.
- Visual verification: `cargo run --example headless_protocol <outdir>` renders
  the protocol through the real GPU pipelines and saves PPM frames + numeric
  `[STATE] slice_w` logs. Convert with `magick x.ppm x.png`.

# Conventions

- Matrices are column-major `[[f32; 4]; 4]` (`m[col][row]`), matching WGSL.
  `camera_matrix()` is camera→world; the slice shader transposes for view.
- Depth convention is wgpu [0,1] (not OpenGL [-1,1]) — see `perspective_matrix`.
- 4D rotations go through SkipY: the 3D rotation axes (X, Y, Z) map onto the
  4D axes (X, Z, W), so Y (gravity) is never affected. `rotate_w` = ZW plane,
  `rotate_xw` = XW plane.
