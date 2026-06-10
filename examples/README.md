# Rust4D Examples

Examples demonstrating the Rust4D 4D rendering engine.

## Running Examples

```bash
cargo run --example 01_hello_tesseract
cargo run --example 02_multiple_shapes
cargo run --example 03_physics_demo
cargo run --example 04_camera_exploration
```

## Example Index

| Example | Description | Key Concepts |
|---------|-------------|--------------|
| 01_hello_tesseract | Minimal 4D rendering setup | World, Entity, Camera4D, basic render loop |
| 02_multiple_shapes | Multiple objects with colors | Transform4D, Material, keyboard input |
| 03_physics_demo | Physics with gravity | PhysicsConfig, RigidBody4D, collision |
| 04_camera_exploration | Full camera controls | CameraController, mouse look, 4D navigation |
| 05_audio_demo | 4D spatial audio | AudioEngine, bus routing, W-distance attenuation |
| ron_preview | Scene file viewer | RON scene loading |
| headless_protocol | **Automated visual verification** — renders a scripted movement protocol offscreen, saves PPM frames + numeric slice-plane logs. No window needed. | Offscreen wgpu, slice invariant, screenshot diffing |
| shape_showcase | **Primitive catalog verification** — renders all built-in primitives at 3 slice offsets × 3 orientations, saves PPM frames and triangle-count logs. | Mesh4D, polytopes, curved shapes, visual regression |

## Learning Path

1. **Start with 01_hello_tesseract** - Understand the basic render pipeline
2. **Move to 02_multiple_shapes** - Learn about transforms and materials
3. **Try 03_physics_demo** - See how physics integration works
4. **Explore with 04_camera_exploration** - Master the full control scheme

## Controls Reference

| Input | Action |
|-------|--------|
| WASD | Move in XZ plane |
| Space | Jump (physics examples) |
| Shift | Move down |
| Q/E | Move along W-axis (4th dimension) |
| Mouse | Look around |
| Scroll | Adjust slice offset |
| R | Reset camera |
| F | Toggle fullscreen |
| Escape | Release cursor / Quit |
