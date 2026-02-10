# About This Project

Rust4D is a 4D game engine written in Rust. Cargo workspace with 8 internal crates.

# Repository Map

```
Rust4D/
├── src/                        # Top-level binary + library (config, input, systems)
├── crates/
│   ├── rust4d_math/            # 4D math (Vec4, Mat4, Rotor4, Ray, Hyperplane, Tesseract)
│   ├── rust4d_core/            # ECS, scenes, assets, transforms
│   ├── rust4d_render/          # wgpu rendering, shaders, particles, sprites, egui overlay
│   ├── rust4d_input/           # Camera controller
│   ├── rust4d_physics/         # Collision, raycasting, spatial indexing, rigid bodies
│   ├── rust4d_game/            # FSM, tweens, events, character controller
│   ├── rust4d_scripting/       # Lua scripting, hot reload, engine bindings
│   └── rust4d_audio/           # Sound, spatial audio, bus mixing
├── examples/                   # Runnable demos + RON scene assets
├── tests/                      # Integration tests
├── docs/                       # Developer/user guides
└── scratchpad/                 # → Obsidian vault (Hades memory)
```

# Key Dependencies

- **wgpu 24** + **winit 0.30** for rendering/windowing
- **hecs** for ECS
- **figment** + **serde** + **RON** for config/scene serialization
- **bytemuck** for GPU data marshalling

# Rust

This project is written in Rust. Use `cargo` for building, testing, and running.
