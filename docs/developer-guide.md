# Rust4D Developer Guide

## Introduction

This guide is for contributors who want to understand, modify, or extend the Rust4D engine. It covers the architecture, algorithms, development workflow, and common tasks you will encounter when working with the codebase.

### Who This Guide Is For

- **Contributors**: Developers fixing bugs, adding features, or improving documentation
- **Engine Extenders**: Those building games or simulations on top of Rust4D
- **4D Enthusiasts**: Anyone curious about implementing 4D graphics and physics

### What You Will Learn

- How the crate architecture supports parallel development
- The mathematics behind 4D rotations (rotors) and slicing (marching tetrahedra)
- How to navigate and modify the rendering pipeline
- Physics simulation internals and collision detection
- Testing strategies and contribution workflow

### Prerequisites

- Proficiency in Rust (ownership, traits, generics, lifetimes)
- Basic understanding of computer graphics (vertices, shaders, render passes)
- Familiarity with linear algebra (vectors, matrices, transformations)
- Optional: Knowledge of geometric algebra helps with understanding rotors

## Project Structure

### Repository Layout

```
Rust4D/
├── crates/                     # Workspace crates
│   ├── rust4d_math/            # 4D math: Vec4, Rotor4, shapes
│   ├── rust4d_core/            # Entity, World, Transform4D, scenes
│   ├── rust4d_physics/         # PhysicsWorld, collision detection
│   ├── rust4d_render/          # WGPU rendering pipeline
│   └── rust4d_input/           # Input handling, CameraController
├── src/                        # Main application binary
│   └── main.rs                 # Entry point
├── examples/                   # Example programs
│   ├── 01_hello_tesseract.rs
│   ├── 02_multiple_shapes.rs
│   ├── 03_physics_demo.rs
│   └── 04_camera_exploration.rs
├── scenes/                     # RON scene files
│   └── default.ron
├── config/                     # Configuration files
│   ├── default.toml
│   └── user.toml               # (gitignored)
├── docs/                       # Documentation
├── tests/                      # Integration tests
├── scratchpad/                 # Shared scratchpad (orphan branch worktree, gitignored)
│   ├── reports/                # Development session reports
│   ├── plans/                  # Architecture and planning docs
│   └── ideas/                  # Feature proposals
├── ARCHITECTURE.md             # System design overview
├── CLAUDE.md                   # AI assistant instructions
└── Cargo.toml                  # Workspace manifest
```

### Build System

#### Cargo Workspace Structure

Rust4D uses a Cargo workspace to organize crates. The root `Cargo.toml` defines shared dependencies and workspace members:

```toml
[workspace]
resolver = "2"
members = [
    "crates/rust4d_math",
    "crates/rust4d_core",
    "crates/rust4d_render",
    "crates/rust4d_input",
    "crates/rust4d_physics",
]

[workspace.dependencies]
wgpu = "24"
winit = "0.30"
bytemuck = { version = "1.14", features = ["derive"] }
# ... other shared dependencies
```

#### Crate Dependency Graph

```
rust4d (main binary)
├── rust4d_core
│   ├── rust4d_math
│   └── rust4d_physics
│       └── rust4d_math
├── rust4d_render
│   ├── rust4d_math
│   ├── rust4d_core
│   └── rust4d_input
└── rust4d_input
    └── rust4d_math
```

Key design principle: `rust4d_math` has no internal dependencies and can be used independently.

#### Key Dependencies by Crate

| Crate | Key Dependencies | Purpose |
|-------|------------------|---------|
| rust4d_math | bytemuck, serde | GPU-compatible types, serialization |
| rust4d_core | slotmap, ron, bitflags | Entity storage, scene files, flags |
| rust4d_physics | slotmap, bitflags | Body storage, collision layers |
| rust4d_render | wgpu, winit, bytemuck | GPU rendering, windowing |
| rust4d_input | winit | Keyboard/mouse handling |

## Development Environment

### Setup with Nix (recommended)

The repository ships a `flake.nix` providing the full development environment:
Rust toolchain, clippy/rustfmt/rust-analyzer, Vulkan loader + validation
layers + lavapipe (software rasterizer for headless GPU work), and the imaging
tools used by the visual verification workflow (grim, imagemagick).

```bash
git clone https://github.com/sigilmakes/Rust4D.git
cd Rust4D
nix develop                       # or: nix develop --command <cmd>
cargo test --workspace
cargo run
```

On NixOS the shell hook wires the system Vulkan ICDs automatically
(`/run/opengl-driver/share/vulkan/icd.d` → `VK_DRIVER_FILES`).

### Manual setup

1. **Install Rust**: Use rustup to install the latest stable Rust toolchain.

2. **Clone and build**:
   ```bash
   git clone https://github.com/sigilmakes/Rust4D.git
   cd Rust4D
   cargo build
   cargo run
   ```

   You'll need a working Vulkan/Metal/DX12 driver and, on Linux, ALSA headers
   (`alsa-lib`/`libasound2-dev`) for the audio crate.

### Recommended Tools

- **rust-analyzer**: Essential for IDE integration (VS Code, Neovim, etc.)
- **cargo-watch**: Auto-rebuild on file changes: `cargo watch -x check`
- **cargo-flamegraph**: CPU profiling for optimization work
- **RenderDoc**: GPU debugging and frame analysis (works with WGPU)

### IDE Configuration Tips

For VS Code with rust-analyzer:
```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.procMacro.enable": true
}
```

### GPU Debugging Tools

- **WGPU Validation**: Enabled by default in debug builds
- **RenderDoc**: Frame capture and GPU state inspection
- **PIX** (Windows): Alternative GPU debugger

Enable WGPU trace output:
```bash
RUST_LOG=wgpu=debug cargo run
```

### Running Tests

#### Full Test Suite

```bash
cargo test --all
```

#### Specific Crate Tests

```bash
cargo test -p rust4d_math
cargo test -p rust4d_physics
```

#### Single Test

```bash
cargo test -p rust4d_physics test_floor_collision
```

#### Test with Output

```bash
cargo test -p rust4d_physics -- --nocapture
```

#### Test Organization Conventions

- **Unit tests**: Inline `#[cfg(test)]` modules at the bottom of source files
- **Integration tests**: `tests/` directory at crate root
- **Doc tests**: Embedded in documentation comments

### Documentation

#### Generating Docs

```bash
cargo doc --all --open
```

#### Documentation Standards

All public items must have documentation. Follow these conventions:

```rust
/// Brief one-line description.
///
/// Longer explanation if needed. Include details about:
/// - What the function does
/// - Important behavior notes
/// - Performance characteristics
///
/// # Arguments
///
/// * `param` - Description of the parameter
///
/// # Returns
///
/// Description of return value.
///
/// # Panics
///
/// Document panic conditions if any.
///
/// # Examples
///
/// ```
/// use rust4d_math::Vec4;
/// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
/// assert_eq!(v.length_squared(), 30.0);
/// ```
pub fn example_function(param: Type) -> ReturnType {
    // ...
}
```

## Architecture Deep Dive

### Crate Responsibilities

Each crate has a focused responsibility, enabling parallel development and clean separation of concerns.

### rust4d_math

**Location**: `crates/rust4d_math/`

The foundational math library with no internal dependencies.

#### Vec4 Implementation

The 4D vector type is the workhorse of the engine:

```rust
// crates/rust4d_math/src/vec4.rs

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,  // The 4th spatial dimension (ana/kata)
}
```

Key features:
- `#[repr(C)]` ensures consistent memory layout for GPU buffers
- `Pod` and `Zeroable` from bytemuck enable safe casting to bytes
- Full operator overloading (`+`, `-`, `*`, `/`)
- Component-wise operations: `min_components`, `max_components`, `component_mul`

#### Rotor4 (Geometric Algebra Rotations)

4D rotations use rotors instead of matrices. See the Key Algorithms section for details.

**File**: `crates/rust4d_math/src/rotor4.rs`

#### Shape Trait System

Shapes implement the `ConvexShape4D` trait:

```rust
// crates/rust4d_math/src/shape.rs

pub trait ConvexShape4D: Send + Sync {
    /// Get the vertices of this shape
    fn vertices(&self) -> &[Vec4];

    /// Get the tetrahedra decomposition of this shape
    fn tetrahedra(&self) -> &[Tetrahedron];

    fn vertex_count(&self) -> usize {
        self.vertices().len()
    }

    fn tetrahedron_count(&self) -> usize {
        self.tetrahedra().len()
    }
}
```

The trait requires:
- `vertices()`: 4D vertex positions
- `tetrahedra()`: Decomposition into 3-simplices for slicing

Current implementations:
- `Tesseract4D`: 16 vertices, 48 boundary tetrahedra (8 cubic facets × 6)
- `Hyperplane4D`: Bounded 4D floor plane
- `Mesh4D` primitives (`rust4d_math::primitives`): hypersphere, 5-cell,
  16-cell, 24-cell, 600-cell, spherinder, cubinder, duocylinder

See the [Shape Catalog](./shapes.md) for construction details, scene syntax,
cell counts, and the visual verification workflow.

### rust4d_core

**Location**: `crates/rust4d_core/`

Provides entity management, world container, and scene serialization.

#### Entity Storage (SlotMap)

Entities use generational keys for safe references:

```rust
// crates/rust4d_core/src/world.rs

new_key_type! {
    pub struct EntityKey;
}

pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    name_index: HashMap<String, EntityKey>,
    physics_world: Option<PhysicsWorld>,
}
```

Benefits of SlotMap:
- O(1) insertion, removal, and lookup
- Generational indices prevent the ABA problem
- Safe iteration during modification

#### World Management and Dirty Tracking

The World tracks which entities need geometry rebuild:

```rust
impl World {
    pub fn update(&mut self, dt: f32) {
        // Step physics
        if let Some(ref mut physics) = self.physics_world {
            physics.step(dt);
        }

        // Sync transforms and mark dirty
        if let Some(ref physics) = self.physics_world {
            for (_key, entity) in &mut self.entities {
                if let Some(body_key) = entity.physics_body {
                    if let Some(body) = physics.get_body(body_key) {
                        if entity.transform.position != body.position {
                            entity.transform.position = body.position;
                            entity.mark_dirty(DirtyFlags::TRANSFORM);
                        }
                    }
                }
            }
        }
    }
}
```

#### Scene Serialization

Scenes use RON format for human-readable serialization:

```ron
// scenes/default.ron
Scene(
    name: "Default Scene",
    gravity: -20.0,
    entities: [
        (
            shape: Tesseract(size: 2.0),
            transform: (position: (x: 0.0, y: 0.0, z: 0.0, w: 0.0)),
            material: (base_color: [1.0, 1.0, 1.0, 1.0]),
            name: Some("tesseract"),
            tags: ["dynamic"],
        ),
    ],
)
```

### rust4d_physics

**Location**: `crates/rust4d_physics/`

4D physics simulation with collision detection and response.

#### PhysicsWorld Integration

```rust
// crates/rust4d_physics/src/world.rs

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    static_colliders: Vec<StaticCollider>,
    pub config: PhysicsConfig,
}
```

The physics step:
1. Apply gravity to non-static bodies
2. Integrate velocities into positions
3. Resolve static collider collisions
4. Resolve body-body collisions

#### Collision Algorithms

All collision functions are in `crates/rust4d_physics/src/collision.rs`.

Supported collision pairs:
- AABB vs AABB: 4D axis-aligned bounding box intersection
- AABB vs Plane: Hyperplane intersection
- Sphere vs AABB: Closest point + distance check
- Sphere vs Plane: Signed distance calculation
- Sphere vs Sphere: Center distance comparison

#### Contact Resolution and Response

```rust
pub struct Contact {
    pub point: Vec4,       // Contact point on first shape surface
    pub normal: Vec4,      // Normal from second shape toward first
    pub penetration: f32,  // Positive = overlapping
}
```

Response algorithm:
1. Push objects apart by `normal * penetration`
2. Apply restitution (bounce) to velocity along normal
3. Apply friction to tangent velocity

### rust4d_render

**Location**: `crates/rust4d_render/`

GPU rendering using WGPU.

#### Pipeline Structure

```
crates/rust4d_render/src/
├── context.rs           # RenderContext: device, queue, surface
├── camera4d.rs          # Camera4D: position, rotation, matrices
├── pipeline/
│   ├── mod.rs           # Pipeline exports
│   ├── slice_pipeline.rs    # Compute shader for 4D->3D slicing
│   ├── render_pipeline.rs   # Vertex/fragment shaders for 3D rendering
│   ├── lookup_tables.rs     # Edge and triangle tables
│   └── types.rs         # GPU-compatible data structures
├── renderable.rs        # RenderableGeometry: World to GPU buffers
└── shaders/
    ├── slice_tetra.wgsl # Tetrahedra slicing (compute)
    └── render.wgsl      # 3D rendering (vertex + fragment)
```

#### WGPU Pipeline Setup

The SlicePipeline creates GPU resources:

```rust
// crates/rust4d_render/src/pipeline/slice_pipeline.rs

pub struct SlicePipeline {
    // Tetrahedra pipeline (preferred)
    tetra_pipeline: wgpu::ComputePipeline,
    tetra_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: Option<wgpu::Buffer>,
    tetra_buffer: Option<wgpu::Buffer>,

    // Shared resources
    output_buffer: wgpu::Buffer,      // 3D triangles output
    counter_buffer: wgpu::Buffer,     // Atomic triangle count
    params_buffer: wgpu::Buffer,      // Slice parameters uniform
}
```

#### Slicing Compute Shader

The shader at `crates/rust4d_render/src/shaders/slice_tetra.wgsl` performs:

1. Transform vertices to camera space
2. Compute which vertices are above/below the slice plane
3. Use lookup tables to find crossed edges
4. Interpolate intersection points
5. Output triangles with normals

#### Render Pass Structure

Each frame:
1. Reset triangle counter
2. Update slice parameters (camera position, rotation)
3. Run compute pass (4D slicing)
4. Run render pass (3D drawing)

### rust4d_input

**Location**: `crates/rust4d_input/`

Input handling for 4D camera control.

#### Camera Controller Design

```rust
// crates/rust4d_input/src/camera_controller.rs

pub struct CameraController {
    // Movement state (booleans for each direction)
    forward: bool,
    backward: bool,
    // ... other directions

    // Configuration
    pub move_speed: f32,
    pub w_move_speed: f32,
    pub mouse_sensitivity: f32,
}

pub trait CameraControl {
    fn move_local_xz(&mut self, forward: f32, right: f32);
    fn move_y(&mut self, delta: f32);
    fn move_w(&mut self, delta: f32);
    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32);
    fn rotate_w(&mut self, delta: f32);
    fn rotate_xw(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}
```

Controls:
- WASD: 3D movement (XZ plane)
- Q/E: 4D movement (W axis, ana/kata)
- Space/Shift: Vertical movement (Y axis)
- Mouse: 3D camera rotation
- Right-click + drag: 4D rotation (W-axis)

## Key Algorithms

### Marching Tetrahedra

The core rendering algorithm slices 4D tetrahedra with a 3D hyperplane.

#### Algorithm Overview

1. **Input**: 4D tetrahedron (4 vertices), slice plane at W coordinate
2. **Classification**: Determine which vertices are above/below the plane (4 bits = 16 cases)
3. **Edge crossing**: Find edges that cross the plane (lookup table)
4. **Interpolation**: Calculate intersection points along crossed edges
5. **Triangulation**: Form 0, 1, or 2 triangles from intersection points

#### Lookup Table Structure

```rust
// crates/rust4d_render/src/pipeline/lookup_tables.rs

// Edge definitions: 6 edges for 4 vertices
pub const TETRA_EDGES: [[usize; 2]; 6] = [
    [0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3],
];

// Edge table: bit mask of crossed edges for each case
pub const TETRA_EDGE_TABLE: [u8; 16] = [...];

// Triangle table: vertex indices for output triangles
// -1 indicates end of triangles
pub const TETRA_TRI_TABLE: [[i8; 6]; 16] = [...];
```

Case analysis:
- **0 or 4 vertices above**: No intersection (plane misses tetrahedron)
- **1 or 3 vertices above**: Triangle (3 crossed edges)
- **2 vertices above**: Quadrilateral (4 crossed edges, split into 2 triangles)

#### Edge Interpolation

For an edge from vertex A to B:
```
t = (slice_w - A.w) / (B.w - A.w)
intersection = A + t * (B - A)
```

The shader implementation:

```wgsl
// crates/rust4d_render/src/shaders/slice_tetra.wgsl

fn edge_intersection(
    p0: vec4<f32>, p1: vec4<f32>,
    c0: vec4<f32>, c1: vec4<f32>,
    slice_w: f32
) -> Vertex3D {
    let w0 = p0.w;
    let w1 = p1.w;
    let dw = w1 - w0;
    let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);

    let pos = mix(p0, p1, t);
    let color = mix(c0, c1, t);
    // ... build vertex
}
```

### 4D Rotation (Rotor4)

Rotors represent 4D rotations more elegantly than matrices.

#### Geometric Algebra Basics

In 4D, rotations happen in planes, not around axes. There are 6 rotation planes:
- **XY, XZ, YZ**: Standard 3D rotations
- **XW, YW, ZW**: 4D rotations involving the W axis

A rotor has 8 components:
- 1 scalar (s)
- 6 bivectors (b_xy, b_xz, b_xw, b_yz, b_yw, b_zw)
- 1 pseudoscalar (p)

#### Why Rotors Not Matrices

1. **No gimbal lock**: Rotors smoothly represent all 4D rotations
2. **Composition**: Multiply rotors to combine rotations
3. **Interpolation**: SLERP-like interpolation between rotations
4. **Normalization**: Single magnitude check vs orthogonality check for matrices

#### Creating Rotors

```rust
// From angle in a specific plane
let r = Rotor4::from_plane_angle(RotationPlane::XY, angle);

// From Euler angles (3D subset)
let r = Rotor4::from_euler_xyz(pitch, yaw, roll);

// Compose rotations: other applied first, then self
let combined = self.compose(&other);
```

#### Rotating Vectors

The sandwich product rotates a vector:

```rust
// v' = R * v * R^reverse
pub fn rotate(&self, v: Vec4) -> Vec4 {
    // Full geometric product computation
    // See rotor4.rs for the detailed implementation
}
```

#### SkipY Transformation for Camera

When the camera rotates in 4D, we want the Y axis (up) to remain stable. The camera uses rotor composition:

```rust
// Build camera rotation from yaw, pitch, and 4D rotations
let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, yaw);
let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, pitch);
let r_zw = Rotor4::from_plane_angle(RotationPlane::ZW, roll_w);
let r_xw = Rotor4::from_plane_angle(RotationPlane::XW, roll_xw);

// Compose: pitch * yaw * zw * xw
let rotation = r_pitch.compose(&r_yaw.compose(&r_zw.compose(&r_xw)));
```

### Collision Detection

#### AABB vs AABB

4D axis-aligned bounding box intersection uses the separating axis theorem:

```rust
// crates/rust4d_physics/src/collision.rs

pub fn aabb_vs_aabb(a: &AABB4D, b: &AABB4D) -> Option<Contact> {
    // Check for separation on each axis
    if a.max.x < b.min.x || a.min.x > b.max.x { return None; }
    if a.max.y < b.min.y || a.min.y > b.max.y { return None; }
    if a.max.z < b.min.z || a.min.z > b.max.z { return None; }
    if a.max.w < b.min.w || a.min.w > b.max.w { return None; }

    // Find overlap on each axis
    let overlap_x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
    // ... similarly for y, z, w

    // Use minimum overlap as penetration, axis as normal
    // ...
}
```

#### Sphere Collisions

Sphere vs plane uses signed distance:

```rust
pub fn sphere_vs_plane(sphere: &Sphere4D, plane: &Plane4D) -> Option<Contact> {
    let signed_dist = plane.signed_distance(sphere.center);
    let penetration = sphere.radius - signed_dist;

    if penetration > 0.0 {
        Some(Contact::new(
            sphere.center - plane.normal * sphere.radius,
            plane.normal,
            penetration,
        ))
    } else {
        None
    }
}
```

#### Bounded Floor Special Case

The bounded floor handles 4D edges specially. When the player is outside the floor's XZW bounds, collision is skipped to allow clean falling:

```rust
// In resolve_static_collisions
if is_player {
    if let Collider::AABB(_) = &static_col.collider {
        if !static_col.is_position_over(body.position) {
            continue; // Skip collision - player fell off edge
        }
    }
}
```

## Code Conventions

### Naming

- **Types**: `PascalCase` (e.g., `PhysicsWorld`, `RigidBody4D`)
- **Functions**: `snake_case` (e.g., `add_entity`, `rotate_3d`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_JUMP_VELOCITY`)
- **Modules**: `snake_case` (e.g., `collision`, `camera_controller`)

### Documentation

All public items must be documented:

```rust
/// Brief description in one line.
///
/// Extended description with implementation details,
/// usage notes, and any caveats.
///
/// # Examples
///
/// ```
/// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
/// ```
pub fn documented_function() { }
```

Module-level documentation in `lib.rs`:

```rust
//! Crate-level documentation.
//!
//! ## Overview
//!
//! Describe the crate's purpose and main types.
//!
//! ## Examples
//!
//! Show typical usage patterns.
```

### Error Handling

- Use `Result` for recoverable errors
- Use `Option` for optional values
- Avoid `panic!` in library code
- Document panic conditions when they exist

Pattern for error types:

```rust
#[derive(Debug)]
pub enum SceneError {
    LoadError(String),
    ParseError(ron::de::SpannedError),
    IoError(std::io::Error),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::LoadError(msg) => write!(f, "Scene load error: {}", msg),
            SceneError::ParseError(e) => write!(f, "Parse error: {}", e),
            SceneError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SceneError {}
```

## Testing Strategy

### Unit Tests

Unit tests live in inline `#[cfg(test)]` modules:

```rust
// At the bottom of the source file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

Conventions:
- Test name describes what is being tested: `test_sphere_vs_plane_colliding`
- Use descriptive assertion messages: `assert!(result, "Expected collision")`
- Test both success and failure cases

### Integration Tests

Integration tests go in the `tests/` directory:

**File**: `crates/rust4d_core/tests/physics_integration.rs`

Structure:
1. **Scene Loading Tests**: Verify scene instantiation creates correct physics
2. **Physics Simulation Tests**: Verify gravity, collision, grounding
3. **Entity-Physics Sync Tests**: Verify transform updates from physics
4. **Full Pipeline Tests**: End-to-end scene loading to physics settling

Example:

```rust
#[test]
fn test_scene_dynamic_entity_falls_to_floor() {
    // Create scene with floor and tesseract
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);
    scene.add_entity(/* floor */);
    scene.add_entity(/* tesseract */);

    // Instantiate
    let mut active = ActiveScene::from_template(&scene, None);

    // Simulate
    for _ in 0..120 {
        active.update(1.0 / 60.0);
    }

    // Verify tesseract fell and landed
    let (_, entity) = active.world.get_by_name("tesseract").unwrap();
    assert!(entity.transform.position.y < 0.0);
}
```

### Test Coverage by Crate

| Crate | Coverage Focus | Key Test Files |
|-------|---------------|----------------|
| rust4d_math | Vector operations, rotor composition | Inline tests in vec4.rs, rotor4.rs |
| rust4d_core | Entity CRUD, dirty tracking, scene loading | Inline + physics_integration.rs |
| rust4d_physics | Collision detection, physics step, body dynamics | Inline in collision.rs, world.rs |
| rust4d_game | CharacterController4D, scene_helpers, events, FSM | Inline + tests/game_integration.rs |
| rust4d_render | GPU buffer sizes, lookup tables | Inline in lookup_tables.rs |
| rust4d_input | Input state handling | Minimal (mostly integration) |

### The Slice Invariant Suite

`tests/slice_invariant.rs` guards the engine's core correctness property (see
[The Mathematics of Rust4D](./4d-math.md#the-slice-invariant)): WASD movement
must never drift the camera across its own slice hyperplane, or every
cross-section on screen morphs while walking.

The suite drives the **real app stack** — `scenes/default.ron`, the physics
world, `CameraController`, `CharacterController4D`, and `SimulationSystem` —
using `SimulationSystem::update_with_dt` for deterministic fixed timesteps.
It asserts `dot(ana, p − cam_pos)` stays constant for a reference world point
across: no rotation, 45° ZW rotation, combined ZW+XW+yaw rotations, and
pitched movement; plus sanity checks that Q/E *does* change the slice and
WASD actually moves the player.

Run it after any change to camera, movement, physics, or input code:

```bash
cargo test --test slice_invariant
```

On failure it prints `[MOVE]`/`[CAM]` lines with camera positions, ana
vectors, and per-frame drift so you can see exactly when and how fast the
slice plane moved.

### Headless Visual Verification

For rendering-path changes (shaders, pipelines, projection, camera matrices),
numeric tests aren't enough — you want to see frames. The headless harness
renders a scripted protocol through the real slice + render pipelines into
offscreen textures, with no window or compositor needed:

```bash
cargo run --example headless_protocol .scratchpad/captures
```

It walks the protocol (baseline → strafe control → 45° 4D rotation →
strafe/forward after rotation → near approach), saving a PPM frame and
printing numeric state at each step:

```
[CAPTURE] 11-strafe-after-rotation.ppm cam=(-1.061,-1.500,5.000,0.707) triangles=5856
[STATE]   11-strafe-after-rotation: slice_w(tesseract)=0.000000 tris=5856
```

Interpretation:
- `slice_w(tesseract)` must stay constant within every WASD phase — that's
  the invariant, measured against the actual render parameters.
- Cross-section **shape and colors** must stay constant within a phase
  (translation and parallax are fine; shrinking or color-gradient shifts mean
  the slice plane is drifting).
- Convert frames for viewing with `magick frame.ppm frame.png`; build
  side-by-side comparisons of two revisions with `magick a.png b.png +append cmp.png`.

This is how the 4D movement bug was finally pinned down after multiple rounds
of purely theoretical analysis failed: instrument the pipeline, run it, read
the numbers, look at the frames.

### Adding New Tests

1. **Unit test**: Add to the `#[cfg(test)]` module in the relevant file
2. **Integration test**: Add to `tests/` directory with `#[test]` attribute
3. **Run specific test**: `cargo test -p crate_name test_name`
4. **Verify all tests pass**: `cargo test --workspace`
5. **Convention-pinning tests**: if you fix a bug rooted in a math or layout
   convention (matrix order, depth range, rotation planes, struct layout),
   add a test that pins the convention and update [4d-math.md](./4d-math.md)

## Performance

### Hot Paths

1. **Rendering loop**: SlicePipeline compute dispatch, RenderPipeline draw
2. **Physics step**: Collision detection, velocity integration
3. **Geometry generation**: World to RenderableGeometry conversion

### Optimization Tips

#### Dirty Tracking Usage

Only rebuild GPU geometry when entities change:

```rust
if world.has_dirty_entities() {
    renderable.rebuild(&world, device);
    world.clear_all_dirty();
}
```

#### GPU Buffer Management

- Reuse buffers when possible (upload_tetrahedra updates existing buffers)
- Use indirect rendering to avoid CPU readback of triangle count
- Batch draw calls by material when multiple objects exist

#### Allocation Avoidance

- Use `Vec::with_capacity()` when size is known
- Reuse containers across frames
- Avoid string allocations in hot paths

### Profiling

#### CPU Profiling with Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bin rust4d
```

#### GPU Profiling

1. Build with validation: `RUST_LOG=wgpu=debug cargo run`
2. Use RenderDoc for frame capture
3. Check for:
   - Buffer upload frequency
   - Compute dispatch efficiency
   - Overdraw in fragment shaders

#### Identifying Bottlenecks

Common issues:
- **Excessive geometry rebuild**: Check dirty flag logic
- **GPU synchronization**: Ensure async compute/render
- **Collision O(n^2)**: Consider spatial partitioning for many bodies

## Contributing

### Git Workflow

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make small, focused commits**:
   ```bash
   git add specific_file.rs
   git commit -m "Add friction to physics materials"
   ```

3. **Push and create PR**:
   ```bash
   git push -u origin feature/your-feature-name
   ```

### Commit Message Format

```
Add friction to physics materials

Implement friction coefficient in PhysicsMaterial struct.
Apply friction during collision response to reduce tangent velocity.

Part of Wave 2: Physics improvements
```

Guidelines:
- First line: imperative mood, ~50 characters
- Blank line, then details if needed
- Reference related issues or plans

### Code Review Checklist

Reviewers check for:
- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] No unnecessary public API changes
- [ ] Performance implications considered
- [ ] Error handling appropriate

### Merge Requirements

- All tests pass (`cargo test --all`)
- No clippy warnings (`cargo clippy --all`)
- Documentation builds (`cargo doc --all`)
- At least one approval (if team project)

### Adding Features

1. **Determine which crate**: Use the dependency graph to find the right location
2. **Write tests first**: Define expected behavior
3. **Implement feature**: Make tests pass
4. **Update documentation**: Doc comments and relevant docs/
5. **Create PR**: Include description and test plan

## Common Tasks

### Adding a New Shape

1. **Define in rust4d_math** (`crates/rust4d_math/src/`):

```rust
// new_shape.rs
pub struct NewShape4D {
    vertices: Vec<Vec4>,
    tetrahedra: Vec<Tetrahedron>,
}

impl NewShape4D {
    pub fn new(/* params */) -> Self {
        // Generate vertices
        // Compute tetrahedra decomposition
    }
}

impl ConvexShape4D for NewShape4D {
    fn vertices(&self) -> &[Vec4] { &self.vertices }
    fn tetrahedra(&self) -> &[Tetrahedron] { &self.tetrahedra }
}
```

2. **Add ShapeTemplate variant** (`crates/rust4d_core/src/shapes.rs`):

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShapeTemplate {
    Tesseract { size: f32 },
    Hyperplane { /* fields */ },
    NewShape { /* fields */ },  // Add this
}
```

3. **Implement instantiation**:

```rust
impl ShapeTemplate {
    pub fn instantiate(&self) -> Box<dyn ConvexShape4D> {
        match self {
            // ...
            ShapeTemplate::NewShape { /* fields */ } => {
                Box::new(NewShape4D::new(/* fields */))
            }
        }
    }
}
```

4. **Add serialization support**: RON derives handle this automatically

5. **Write tests**:

```rust
#[test]
fn test_new_shape_vertex_count() {
    let shape = NewShape4D::new(/* params */);
    assert_eq!(shape.vertex_count(), EXPECTED_COUNT);
}

#[test]
fn test_new_shape_tetrahedra_valid() {
    let shape = NewShape4D::new(/* params */);
    for tet in shape.tetrahedra() {
        for &idx in &tet.indices {
            assert!(idx < shape.vertex_count());
        }
    }
}
```

### Adding a Physics Feature

1. **Modify rust4d_physics**:

```rust
// crates/rust4d_physics/src/body.rs or new file

pub struct NewFeature {
    // ...
}

impl RigidBody4D {
    pub fn with_new_feature(mut self, feature: NewFeature) -> Self {
        self.new_feature = Some(feature);
        self
    }
}
```

2. **Update PhysicsWorld**:

```rust
// crates/rust4d_physics/src/world.rs

impl PhysicsWorld {
    pub fn step(&mut self, dt: f32) {
        // ... existing code ...

        // Add new feature processing
        self.process_new_feature();
    }

    fn process_new_feature(&mut self) {
        // Implementation
    }
}
```

3. **Add tests**:

```rust
#[test]
fn test_new_feature_behavior() {
    let mut world = PhysicsWorld::new();
    let body = RigidBody4D::new_sphere(/* ... */)
        .with_new_feature(NewFeature { /* ... */ });
    let key = world.add_body(body);

    world.step(0.1);

    let body = world.get_body(key).unwrap();
    // Assert expected behavior
}
```

4. **Update documentation**: Add to this guide and inline docs

### Modifying Shaders

#### Shader File Locations

- **Compute shaders**: `crates/rust4d_render/src/shaders/slice_tetra.wgsl`
- **Render shaders**: `crates/rust4d_render/src/shaders/render.wgsl`

#### WGSL Syntax Notes

```wgsl
// Structs must match Rust layout exactly
struct Vertex4D {
    position: vec4<f32>,
    color: vec4<f32>,
}

// Storage buffers for large data
@group(0) @binding(0) var<storage, read> vertices: array<Vertex4D>;
@group(0) @binding(1) var<storage, read_write> output: array<Triangle3D>;

// Uniform buffers for small, frequently-updated data
@group(0) @binding(2) var<uniform> params: SliceParams;

// Compute shader entry point
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    // ...
}
```

#### Pipeline Updates Required

When modifying shader structs:

1. Update Rust struct in `crates/rust4d_render/src/pipeline/types.rs`
2. Ensure `#[repr(C)]` and `bytemuck` derives match
3. Update bind group layout if bindings change
4. Rebuild pipeline if entry point changes

#### Testing Shader Changes

1. Run with validation: `RUST_LOG=wgpu=debug cargo run`
2. Check for shader compilation errors
3. Verify visual output is correct
4. Test edge cases (empty geometry, degenerate triangles)

## Future Architecture

### ECS Migration Path

The current entity system is simplified. For larger games, consider ECS:

**Current Design**:
- Entity has embedded components (shape, transform, material)
- World stores entities directly

**ECS Design**:
- Components stored in separate arrays
- Systems iterate over component queries
- Better cache locality for large entity counts

**Migration Strategy**:
1. Keep current API as facade
2. Add component storage behind the scenes
3. Migrate hot paths first (rendering, physics)
4. Maintain backward compatibility

### Plugin System

Potential design for extensibility:

**Extension Points**:
- Custom shapes (implement ConvexShape4D)
- Custom collision handlers
- Post-processing effects
- Input handlers

**API Stability**:
- Core traits (ConvexShape4D, CameraControl) are stable
- Internal types may change
- Version plugins with engine version

**Future Considerations**:
- Dynamic loading of WASM plugins
- Hot reloading for development
- Scripting language integration (Lua, Python)
