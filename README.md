# Rust4D

A 4D rendering engine in Rust. See real-time 3D cross-sections of 4D geometry.
Heavily inspired by [Engine4D](https://github.com/HackerPoet/Engine4D) by Code Parade

## Project Status

**Alpha Stage** - Core rendering and physics are functional, but the API may change.

What works:
- 4D to 3D slicing via compute shaders
- Real-time navigation in 4D space
- Basic 4D physics with gravity and collisions
- Multiple 4D primitives (tesseract, hyperplane)
- FPS-style camera controls with 4D extensions
- Scene serialization and loading (RON format)
- Configuration system (TOML with env var overrides)
- 4D spatial audio with bus routing (Master/Sfx/Music/Ambient)
- Lua 5.4 scripting with lifecycle callbacks and hot-reload
- HUD system with sprites and particles
- Game utilities: EventBus, StateMachine, TweenManager

What's in progress:
- More 4D shapes (hypersphere, 4D prisms)
- Advanced collision detection

## What is 4D Rendering?

Imagine you're a 2D being living on a flat plane. If a 3D sphere passes through your plane, you'd see it as a circle that grows, reaches maximum size, then shrinks and vanishes. You're seeing 2D cross-sections of a 3D object.

**Rust4D does the same thing, but one dimension higher.**

We take 4D objects (like a tesseract, the 4D analog of a cube) and slice them with a 3D hyperplane. What you see on screen is a 3D cross-section of 4D geometry. Move along the W-axis (the 4th spatial dimension) and watch shapes morph in ways impossible in 3D:

- A tesseract appears as a cube, morphs into complex polyhedra, then back to a cube
- Objects can pass "through" each other by taking different paths in W
- The floor extends infinitely in 4D, but you only see the slice at your W position

This is not just a visualization trick - the engine actually computes 4D geometry and performs true 4D physics.

## Features

- **True 4D Geometry**: All primitives are mathematically defined in 4D space
- **GPU-Accelerated Slicing**: Compute shaders slice tetrahedra in parallel
- **4D Physics**: Gravity, collision detection, and rigid body dynamics in 4D
- **FPS-Style Controls**: Navigate 4D space with intuitive WASD + Q/E controls
- **Modular Architecture**: 8 specialized crates for math, core, render, physics, input, audio, scripting, and game logic
- **Configuration System**: TOML-based configuration with environment variable overrides
- **Cross-Platform**: Runs on Windows, macOS, and Linux via wgpu

## Architecture Overview

Rust4D is organized as a Cargo workspace with 8 specialized crates:

```
rust4d/                 # Main application
crates/
  rust4d_math/          # 4D vector and matrix math
  rust4d_core/          # World, entities, transforms, shapes
  rust4d_render/        # GPU rendering pipeline, Camera4D, HUD, sprites, particles
  rust4d_physics/       # 4D physics simulation
  rust4d_input/         # Input handling and camera controller
  rust4d_audio/         # 4D spatial audio with kira, bus routing
  rust4d_scripting/     # Lua 5.4 scripting with hot-reload
  rust4d_game/          # CharacterController4D, EventBus, StateMachine, TweenManager
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed diagrams and data flow.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- A GPU with Vulkan, Metal, or DX12 support

### Building and Running

```bash
# Clone the repository
git clone https://github.com/yourusername/rust4d
cd rust4d

# Run the main application
cargo run --release

# Or try an example
cargo run --example 01_hello_tesseract
```

### Quick Start with Examples

The examples are the best way to learn Rust4D. They progress from simple to complex:

```bash
# 1. Minimal setup - just a tesseract
cargo run --example 01_hello_tesseract

# 2. Multiple shapes with keyboard movement
cargo run --example 02_multiple_shapes

# 3. Physics simulation with falling objects
cargo run --example 03_physics_demo

# 4. Full camera controls - explore 4D space
cargo run --example 04_camera_exploration
```

See [examples/README.md](examples/README.md) for detailed descriptions.

## Configuration

Configuration is loaded from TOML files with environment variable overrides:

1. `config/default.toml` - Default settings (checked into git)
2. `config/user.toml` - Your personal overrides (gitignored)
3. Environment variables with `R4D_` prefix

### Example: config/default.toml

```toml
[window]
title = "Rust4D - 4D Rendering Engine"
width = 1280
height = 720

[camera]
start_position = [0.0, 0.0, 5.0, 0.0]
fov = 45.0

[input]
move_speed = 3.0
w_move_speed = 2.0
mouse_sensitivity = 0.002

[physics]
gravity = -20.0
jump_velocity = 8.0
```

### Environment Variable Override

```bash
# Override window title
R4D_WINDOW__TITLE="My 4D Game" cargo run

# Override physics gravity
R4D_PHYSICS__GRAVITY="-10.0" cargo run
```

## Controls

| Input | Action |
|-------|--------|
| WASD | Move in XZ plane |
| Q/E | Move along W-axis (4th dimension) |
| Space/Shift | Move up/down |
| Mouse | Look around |
| Right-click drag | Rotate through W |
| Scroll | Adjust slice offset |
| R | Reset camera |
| F | Fullscreen |
| G | Toggle input smoothing |
| ESC | Release cursor / Quit |

## Examples

See [examples/README.md](examples/README.md) for the full example index and learning path.

| Example | Description |
|---------|-------------|
| 01_hello_tesseract | Minimal 4D rendering setup |
| 02_multiple_shapes | Multiple objects with transforms |
| 03_physics_demo | Physics with gravity and collision |
| 04_camera_exploration | Full camera controls |

## Inspiration

- [4D Golf](https://store.steampowered.com/app/2147950/4D_Golf/) - Camera controls based on this
- [4D Toys](http://4dtoys.com/) - 4D physics visualization
- [Miegakure](https://miegakure.com/) - 4D puzzle platformer
- [Engine4D](https://github.com/HackerPoet/Engine4D) - Original inspiration

## License

MIT
