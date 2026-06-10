//! Headless visual verification of the slice-plane invariant.
//!
//! Runs the full app stack (scene, physics, camera, simulation) without a
//! window, renders frames through the REAL slice + render pipelines into an
//! offscreen texture, and saves them as PPM images.
//!
//! Protocol (mirrors scratchpad/plans/2026-02-06-camera-debug.md):
//!   1. settle, capture `00-baseline`
//!   2. strafe right 1s (no rotation), capture every 20 frames
//!   3. rotate 45° in the QE/ana plane, capture `10-rotated`
//!   4. strafe left 1s, capture every 20 frames   <- morphs when buggy
//!   5. walk forward 1s, capture every 20 frames  <- morphs when buggy
//!
//! If the slice invariant holds, the tesseract cross-section may translate
//! and parallax across frames within a phase, but its shape/extent in the
//! W direction must not change. Triangle counts are printed as a coarse
//! numeric morph indicator.
//!
//! Usage: cargo run --example headless_protocol [output_dir]

use rust4d::systems::{build_geometry, SimulationSystem};
use rust4d_core::SceneManager;
use rust4d_game::{scene_helpers, CharacterConfig, CharacterController4D};
use rust4d_input::CameraController;
use rust4d_math::Vec4;
use rust4d_physics::PhysicsConfig;
use rust4d_render::camera4d::Camera4D;
use rust4d_render::pipeline::{
    perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline,
};
use winit::event::ElementState;
use winit::keyboard::KeyCode;

use std::f32::consts::FRAC_PI_4;
use std::path::{Path, PathBuf};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const DT: f32 = 1.0 / 60.0;
const MOVE_SPEED: f32 = 3.0;
const W_MOVE_SPEED: f32 = 2.0;
const MAX_TRIANGLES: usize = 900_000;

// ============================================================================
// Headless GPU context
// ============================================================================

struct HeadlessGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    slice_pipeline: SlicePipeline,
    render_pipeline: RenderPipeline,
    color_texture: wgpu::Texture,
    readback_buffer: wgpu::Buffer,
    padded_bytes_per_row: u32,
}

impl HeadlessGpu {
    fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("no GPU adapter available");

        let info = adapter.get_info();
        println!("[GPU] adapter: {} ({:?}, {:?})", info.name, info.device_type, info.backend);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Headless Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("failed to create device");

        let slice_pipeline = SlicePipeline::new(&device, MAX_TRIANGLES);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut render_pipeline = RenderPipeline::new(&device, format);
        render_pipeline.ensure_depth_texture(&device, WIDTH, HEIGHT);

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Color"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // wgpu requires bytes_per_row aligned to 256
        let unpadded = WIDTH * 4;
        let padded_bytes_per_row = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback"),
            size: (padded_bytes_per_row * HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            slice_pipeline,
            render_pipeline,
            color_texture,
            readback_buffer,
            padded_bytes_per_row,
        }
    }

    fn upload_geometry(&mut self, geometry: &rust4d_render::RenderableGeometry) {
        self.slice_pipeline.upload_tetrahedra(
            &self.device,
            &geometry.vertices,
            &geometry.tetrahedra,
        );
        println!(
            "[GPU] uploaded {} vertices, {} tetrahedra",
            geometry.vertex_count(),
            geometry.tetrahedron_count()
        );
    }

    /// Render one frame exactly like `RenderSystem::render_frame` and save it.
    fn capture(
        &mut self,
        camera: &Camera4D,
        tetrahedron_count: u32,
        path: &Path,
    ) -> u32 {
        let pos = camera.position;
        let camera_matrix = camera.rotation_matrix();
        let slice_params = SliceParams {
            slice_w: camera.get_slice_w(),
            tetrahedron_count,
            _padding: [0.0; 2],
            camera_matrix,
            camera_eye: [pos.x, pos.y, pos.z],
            _padding2: 0.0,
            camera_position: [pos.x, pos.y, pos.z, pos.w],
        };
        self.slice_pipeline.update_params(&self.queue, &slice_params);

        let proj = perspective_matrix(
            45.0_f32.to_radians(),
            WIDTH as f32 / HEIGHT as f32,
            0.1,
            100.0,
        );
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.render_pipeline.update_uniforms(
            &self.queue,
            &RenderUniforms {
                view_matrix: identity,
                projection_matrix: proj,
                light_dir: [0.5, 1.0, 0.3],
                _padding: 0.0,
                ambient_strength: 0.3,
                diffuse_strength: 0.7,
                w_color_strength: 0.3,
                w_range: 2.0,
            },
        );

        let view = self
            .color_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Headless Encoder"),
            });

        self.slice_pipeline.reset_counter(&self.queue);
        self.slice_pipeline.run_slice_pass(&mut encoder);
        self.render_pipeline
            .prepare_indirect_draw(&mut encoder, self.slice_pipeline.counter_buffer());
        self.render_pipeline.render(
            &mut encoder,
            &view,
            self.slice_pipeline.output_buffer(),
            wgpu::Color {
                r: 0.05,
                g: 0.05,
                b: 0.08,
                a: 1.0,
            },
        );

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and save
        let slice = self.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().expect("readback map failed");

        {
            let data = slice.get_mapped_range();
            write_ppm(path, &data, WIDTH, HEIGHT, self.padded_bytes_per_row);
        }
        self.readback_buffer.unmap();

        // Read the vertex counter for a coarse numeric morph indicator
        let vertex_count = self.read_counter();
        println!(
            "[CAPTURE] {} cam=({:.3},{:.3},{:.3},{:.3}) triangles={}",
            path.file_name().unwrap().to_string_lossy(),
            pos.x,
            pos.y,
            pos.z,
            pos.w,
            vertex_count / 3
        );
        vertex_count / 3
    }

    fn read_counter(&self) -> u32 {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Counter Staging"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(self.slice_pipeline.counter_buffer(), 0, &staging, 0, 4);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().expect("counter map failed");
        let count = {
            let data = slice.get_mapped_range();
            u32::from_le_bytes(data[..4].try_into().unwrap())
        };
        staging.unmap();
        count
    }
}

fn write_ppm(path: &Path, data: &[u8], width: u32, height: u32, padded_bytes_per_row: u32) {
    let mut out = Vec::with_capacity((width * height * 3) as usize + 32);
    out.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    for y in 0..height {
        let row_start = (y * padded_bytes_per_row) as usize;
        for x in 0..width {
            let px = row_start + (x * 4) as usize;
            out.extend_from_slice(&data[px..px + 3]); // RGB, drop A
        }
    }
    std::fs::write(path, out).expect("failed to write ppm");
}

// ============================================================================
// Simulation rig (mirrors App::new in src/main.rs)
// ============================================================================

struct Rig {
    scene_manager: SceneManager,
    camera: Camera4D,
    controller: CameraController,
    character: Option<CharacterController4D>,
    simulation: SimulationSystem,
}

impl Rig {
    fn new() -> Self {
        let mut scene_manager = SceneManager::new().with_physics(PhysicsConfig::new(-20.0));
        let scene_name = scene_manager
            .load_scene("scenes/default.ron")
            .expect("load default scene");
        scene_manager.instantiate(&scene_name).expect("instantiate");
        scene_manager.push_scene(&scene_name).expect("push");

        let mut player_start = Vec4::new(0.0, 0.0, 5.0, 0.0);
        if let Some(scene) = scene_manager.active_scene_mut() {
            if let Some(spawn) = scene.player_spawn {
                let spawn_pos = Vec4::new(spawn[0], spawn[1], spawn[2], spawn[3]);
                player_start = spawn_pos;
                if let Some(physics) = scene.world.physics_mut() {
                    let key = scene_helpers::create_player_body(physics, spawn_pos, 0.5);
                    scene.player_body_key = Some(key);
                }
            }
        }

        let mut camera = Camera4D::new();
        camera.position = player_start;

        let controller = CameraController::new()
            .with_move_speed(MOVE_SPEED)
            .with_w_move_speed(W_MOVE_SPEED);

        let character = scene_manager
            .active_scene()
            .and_then(|s| s.player_body_key)
            .map(|key| {
                CharacterController4D::new(
                    key,
                    CharacterConfig {
                        move_speed: MOVE_SPEED,
                        w_move_speed: W_MOVE_SPEED,
                        jump_velocity: 8.0,
                    },
                )
            });

        Self {
            scene_manager,
            camera,
            controller,
            character,
            simulation: SimulationSystem::new(),
        }
    }

    fn step(&mut self) {
        self.simulation.update_with_dt(
            DT,
            &mut self.scene_manager,
            &mut self.camera,
            &mut self.controller,
            self.character.as_ref(),
            false,
        );
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step();
        }
    }

    fn slice_w_of(&self, p: Vec4) -> f32 {
        self.camera.ana().dot(p - self.camera.position)
    }
}

// ============================================================================
// Protocol
// ============================================================================

fn main() {
    env_logger::init();

    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".scratchpad/captures"));
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let mut rig = Rig::new();
    let geometry = build_geometry(rig.scene_manager.active_world().unwrap());
    let tetra_count = geometry.tetrahedron_count() as u32;

    let mut gpu = HeadlessGpu::new();
    gpu.upload_geometry(&geometry);

    let tesseract = Vec4::new(0.0, 0.0, 0.0, 0.0);
    let cap = |rig: &Rig, gpu: &mut HeadlessGpu, name: &str| {
        let path = out_dir.join(format!("{name}.ppm"));
        let tris = gpu.capture(&rig.camera, tetra_count, &path);
        println!(
            "[STATE]   {name}: slice_w(tesseract)={:.6} tris={tris}",
            rig.slice_w_of(tesseract)
        );
    };

    // Phase 0: settle
    rig.run(120);
    cap(&rig, &mut gpu, "00-baseline");

    // Phase 1: strafe right then back, no rotation (control: shape must only
    // translate/parallax, never morph)
    rig.controller
        .process_keyboard(KeyCode::KeyD, ElementState::Pressed);
    for i in 1..=2 {
        rig.run(30);
        cap(&rig, &mut gpu, &format!("0{i}-strafe-no-rotation"));
    }
    rig.controller
        .process_keyboard(KeyCode::KeyD, ElementState::Released);
    rig.controller
        .process_keyboard(KeyCode::KeyA, ElementState::Pressed);
    rig.run(60); // return to x ≈ 0 so the rotated slice passes through the tesseract
    rig.controller
        .process_keyboard(KeyCode::KeyA, ElementState::Released);
    cap(&rig, &mut gpu, "03-returned-to-start");

    // Phase 2: rotate 45° into the 4th dimension
    rig.camera.rotate_w(FRAC_PI_4);
    cap(&rig, &mut gpu, "10-rotated-45");

    // Phase 3: strafe left after rotation (bug trigger: with the anisotropic
    // scaling bug, the slice plane drifts 0.5 units/s and the cross-section
    // visibly morphs/shrinks; when fixed it only translates)
    rig.controller
        .process_keyboard(KeyCode::KeyA, ElementState::Pressed);
    for i in 1..=4 {
        rig.run(30);
        cap(&rig, &mut gpu, &format!("1{i}-strafe-after-rotation"));
    }
    rig.controller
        .process_keyboard(KeyCode::KeyA, ElementState::Released);

    // Phase 4: walk forward after rotation. At this particular orientation
    // forward stays -Z, so even the buggy build must not morph here —
    // an in-protocol control.
    rig.controller
        .process_keyboard(KeyCode::KeyW, ElementState::Pressed);
    for i in 1..=2 {
        rig.run(30);
        cap(&rig, &mut gpu, &format!("2{i}-forward-after-rotation"));
    }
    rig.controller
        .process_keyboard(KeyCode::KeyW, ElementState::Released);

    println!("\nCaptures written to {}", out_dir.display());
}
