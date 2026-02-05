//! RON Asset Preview Tool
//!
//! A standalone tool for previewing 4D assets defined in RON format.
//!
//! Usage: cargo run --example ron_preview -- path/to/asset.ron
//!
//! Features:
//! - Hot-reload: Watches the file for changes and reloads automatically
//! - Camera controls: WASD + mouse for XYZ movement, Q/E for W-axis
//! - W-slice navigation via scroll wheel
//!
//! Controls:
//! - Click to capture cursor, Escape to release
//! - WASD: Move in XZ plane (forward/backward/strafe)
//! - Space/Shift: Move up/down (Y-axis)
//! - Q/E: Move along W-axis (4th dimension - ana/kata)
//! - Mouse: Look around (when cursor captured)
//! - Scroll: Adjust W-slice offset
//! - R: Reset camera
//! - ESC: Exit (or release cursor if captured)
//!
//! Run with: `cargo run --example ron_preview`
//! Or with a custom file: `cargo run --example ron_preview -- path/to/asset.ron`

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use rust4d_core::{Material, ShapeRef, Tesseract4D, Transform4D, DirtyFlags, World};
use rust4d_render::{
    camera4d::Camera4D,
    context::RenderContext,
    pipeline::{
        perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline,
        MAX_OUTPUT_TRIANGLES,
    },
    RenderableGeometry, position_gradient_color,
};
use rust4d_math::Vec4;
use rust4d_input::CameraController;

// ============================================================================
// RON Asset Types
// ============================================================================

/// A previewable 4D asset loaded from RON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewAsset {
    /// Asset name
    pub name: String,
    /// Mesh type to render
    pub mesh: PreviewMesh,
    /// Initial position [x, y, z, w]
    #[serde(default)]
    pub position: [f32; 4],
    /// Uniform scale factor
    #[serde(default = "default_scale")]
    pub scale: f32,
}

fn default_scale() -> f32 {
    1.0
}

/// Mesh types supported by the preview tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreviewMesh {
    /// Built-in tesseract (hypercube)
    Tesseract,
    /// Built-in 4D sphere approximation (hypersphere)
    /// Note: Currently renders as tesseract since Hypersphere isn't in rust4d_math
    Hypersphere {
        /// Subdivision segments (higher = smoother)
        segments: u32,
    },
}

impl PreviewAsset {
    /// Load asset from a RON file
    pub fn load(path: &std::path::Path) -> Result<Self, PreviewLoadError> {
        let contents = std::fs::read_to_string(path)?;
        let asset: PreviewAsset = ron::from_str(&contents)?;
        Ok(asset)
    }
}

/// Error loading a preview asset
#[derive(Debug)]
pub enum PreviewLoadError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
}

impl From<std::io::Error> for PreviewLoadError {
    fn from(e: std::io::Error) -> Self {
        PreviewLoadError::Io(e)
    }
}

impl From<ron::error::SpannedError> for PreviewLoadError {
    fn from(e: ron::error::SpannedError) -> Self {
        PreviewLoadError::Parse(e)
    }
}

impl std::fmt::Display for PreviewLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewLoadError::Io(e) => write!(f, "IO error: {}", e),
            PreviewLoadError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for PreviewLoadError {}

// ============================================================================
// Preview Application
// ============================================================================

/// Main preview application state
struct PreviewApp {
    // Asset state
    asset_path: PathBuf,
    asset: Option<PreviewAsset>,
    last_modified: Option<SystemTime>,
    last_check: Instant,

    // Window and rendering
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,

    // World and geometry
    world: World,
    geometry: RenderableGeometry,
    geometry_dirty: bool,

    // Camera and input
    camera: Camera4D,
    controller: CameraController,
    last_frame: Instant,
    cursor_captured: bool,
}

impl PreviewApp {
    /// Check interval for hot-reload (500ms)
    const HOT_RELOAD_INTERVAL_MS: u64 = 500;

    fn new(asset_path: PathBuf) -> Self {
        let mut app = Self {
            asset_path,
            asset: None,
            last_modified: None,
            last_check: Instant::now(),

            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,

            world: World::new(),
            geometry: RenderableGeometry::new(),
            geometry_dirty: true,

            camera: Camera4D::new(),
            controller: CameraController::new()
                .with_move_speed(5.0)
                .with_w_move_speed(3.0)
                .with_mouse_sensitivity(0.002)
                .with_smoothing(false),
            last_frame: Instant::now(),
            cursor_captured: false,
        };

        // Set initial camera position
        app.camera.position = Vec4::new(0.0, 2.0, 8.0, 0.0);

        // Initial load
        if let Err(e) = app.load_asset() {
            eprintln!("Error loading asset: {}", e);
        }

        app
    }

    /// Load or reload asset from file
    fn load_asset(&mut self) -> Result<(), String> {
        match PreviewAsset::load(&self.asset_path) {
            Ok(asset) => {
                println!("Loaded: {} ({})", asset.name, self.asset_path.display());
                self.asset = Some(asset);
                self.rebuild_world();
                Ok(())
            }
            Err(e) => Err(format!("{}", e)),
        }
    }

    /// Rebuild the world from the current asset
    fn rebuild_world(&mut self) {
        self.world = World::new();

        if let Some(asset) = &self.asset {
            let position = Vec4::new(
                asset.position[0],
                asset.position[1],
                asset.position[2],
                asset.position[3],
            );

            // Create transform with position and scale
            let mut transform = Transform4D::from_position(position);
            transform.scale = asset.scale;

            // Create mesh based on type
            let shape: Box<dyn rust4d_math::ConvexShape4D> = match &asset.mesh {
                PreviewMesh::Tesseract => Box::new(Tesseract4D::new(1.0)),
                PreviewMesh::Hypersphere { segments: _ } => {
                    // TODO: Implement proper hypersphere when available in rust4d_math
                    // For now, use a tesseract as placeholder
                    println!("Note: Hypersphere not yet implemented, using tesseract as placeholder");
                    Box::new(Tesseract4D::new(1.0))
                }
            };

            // Spawn entity - use ShapeRef::Owned directly since shape is already boxed
            self.world.spawn((
                ShapeRef::Owned(shape),
                transform,
                Material::from_rgb(0.8, 0.5, 0.2), // Orange color
                DirtyFlags::ALL,
            ));
        }

        self.geometry_dirty = true;
    }

    /// Rebuild GPU geometry from world
    fn rebuild_geometry(&mut self) {
        self.geometry = RenderableGeometry::new();

        for (_entity, (transform, shape, material)) in self
            .world
            .ecs()
            .query::<(&Transform4D, &ShapeRef, &Material)>()
            .iter()
        {
            self.geometry.add_components_with_color(
                transform,
                shape.as_shape(),
                material,
                &position_gradient_color,
            );
        }

        self.geometry_dirty = false;
    }

    /// Check for hot-reload (file modification)
    fn check_hot_reload(&mut self) -> bool {
        // Only check every HOT_RELOAD_INTERVAL_MS
        if self.last_check.elapsed().as_millis() < Self::HOT_RELOAD_INTERVAL_MS as u128 {
            return false;
        }
        self.last_check = Instant::now();

        let metadata = match std::fs::metadata(&self.asset_path) {
            Ok(m) => m,
            Err(_) => return false,
        };

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => return false,
        };

        if Some(modified) != self.last_modified {
            self.last_modified = Some(modified);

            // Skip the initial check (first time we see the file)
            if self.asset.is_some() {
                println!("File changed, reloading...");
                if let Err(e) = self.load_asset() {
                    eprintln!("Reload error: {}", e);
                }
                return true;
            }
        }

        false
    }

    /// Upload geometry to GPU
    fn upload_geometry(&mut self) {
        if let (Some(ctx), Some(sp)) = (&self.render_context, &mut self.slice_pipeline) {
            sp.upload_tetrahedra(&ctx.device, &self.geometry.vertices, &self.geometry.tetrahedra);
        }
    }

    /// Capture cursor for FPS-style controls
    fn capture_cursor(&mut self) {
        if let Some(window) = &self.window {
            let grab_result = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));

            if grab_result.is_ok() {
                window.set_cursor_visible(false);
                self.cursor_captured = true;
            }
        }
    }

    /// Release cursor
    fn release_cursor(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.cursor_captured = false;
        }
    }
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let title = match &self.asset {
                Some(a) => format!("RON Preview - {}", a.name),
                None => "RON Preview".to_string(),
            };

            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title(&title)
                            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                    )
                    .expect("Failed to create window"),
            );

            let render_context = pollster::block_on(RenderContext::new(window.clone()));
            let mut slice_pipeline =
                SlicePipeline::new(&render_context.device, MAX_OUTPUT_TRIANGLES);
            let mut render_pipeline =
                RenderPipeline::new(&render_context.device, render_context.config.format);

            render_pipeline.ensure_depth_texture(
                &render_context.device,
                render_context.size.width,
                render_context.size.height,
            );

            // Build initial geometry if needed
            if self.geometry_dirty {
                self.rebuild_geometry();
            }

            slice_pipeline.upload_tetrahedra(
                &render_context.device,
                &self.geometry.vertices,
                &self.geometry.tetrahedra,
            );

            self.window = Some(window);
            self.render_context = Some(render_context);
            self.slice_pipeline = Some(slice_pipeline);
            self.render_pipeline = Some(render_pipeline);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(ctx) = &mut self.render_context {
                    ctx.resize(size);
                }
                if let (Some(ctx), Some(rp)) = (&self.render_context, &mut self.render_pipeline) {
                    rp.ensure_depth_texture(&ctx.device, size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        match key {
                            KeyCode::Escape => {
                                if self.cursor_captured {
                                    self.release_cursor();
                                } else {
                                    event_loop.exit();
                                }
                                return;
                            }
                            KeyCode::KeyR => {
                                // Reset camera
                                self.camera.reset();
                                self.camera.position = Vec4::new(0.0, 2.0, 8.0, 0.0);
                            }
                            _ => {}
                        }
                    }
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed
                    && button == MouseButton::Left
                    && !self.cursor_captured
                {
                    self.capture_cursor();
                }
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                // Calculate delta time
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Check for hot-reload
                if self.check_hot_reload() && self.geometry_dirty {
                    self.rebuild_geometry();
                    self.upload_geometry();
                }

                // Update camera
                self.controller
                    .update(&mut self.camera, dt, self.cursor_captured);

                // Update window title with position info
                if let Some(window) = &self.window {
                    let pos = self.camera.position;
                    let slice_w = self.camera.get_slice_w();
                    let name = self
                        .asset
                        .as_ref()
                        .map(|a| a.name.as_str())
                        .unwrap_or("No asset");
                    let capture_hint = if self.cursor_captured {
                        "[ESC to release]"
                    } else {
                        "[Click to capture]"
                    };
                    let title = format!(
                        "RON Preview - {} | Pos: ({:.1}, {:.1}, {:.1}, W:{:.1}) Slice:{:.2} {}",
                        name, pos.x, pos.y, pos.z, pos.w, slice_w, capture_hint
                    );
                    window.set_title(&title);
                }

                // Render
                if let (Some(ctx), Some(sp), Some(rp)) = (
                    &self.render_context,
                    &self.slice_pipeline,
                    &self.render_pipeline,
                ) {
                    let pos = self.camera.position;
                    let slice_params = SliceParams {
                        slice_w: self.camera.get_slice_w(),
                        tetrahedron_count: self.geometry.tetrahedron_count() as u32,
                        _padding: [0.0; 2],
                        camera_matrix: self.camera.view_matrix(),
                        camera_position: [pos.x, pos.y, pos.z, pos.w],
                    };
                    sp.update_params(&ctx.queue, &slice_params);

                    let render_uniforms = RenderUniforms {
                        view_matrix: [
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 0.0, 0.0, 1.0],
                        ],
                        projection_matrix: perspective_matrix(
                            std::f32::consts::FRAC_PI_4,
                            ctx.aspect_ratio(),
                            0.1,
                            100.0,
                        ),
                        light_dir: [0.5, 1.0, 0.3],
                        _padding: 0.0,
                        ambient_strength: 0.3,
                        diffuse_strength: 0.7,
                        w_color_strength: 0.5,
                        w_range: 2.0,
                    };
                    rp.update_uniforms(&ctx.queue, &render_uniforms);

                    let output = match ctx.surface.get_current_texture() {
                        Ok(o) => o,
                        Err(_) => return,
                    };
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = ctx
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    sp.reset_counter(&ctx.queue);
                    sp.run_slice_pass(&mut encoder);
                    rp.prepare_indirect_draw(&mut encoder, sp.counter_buffer());
                    rp.render(
                        &mut encoder,
                        &view,
                        sp.output_buffer(),
                        wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.1,
                            a: 1.0,
                        },
                    );

                    ctx.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.controller.process_mouse_motion(delta.0, delta.1);
        }
    }
}

fn main() {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let asset_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/assets/preview_tesseract.ron"));

    println!("===========================================");
    println!("       RON Asset Preview Tool");
    println!("===========================================");
    println!();
    println!("Loading: {}", asset_path.display());
    println!();
    println!("Controls:");
    println!("  Click       - Capture cursor");
    println!("  ESC         - Release cursor / Exit");
    println!("  WASD        - Move camera (XZ plane)");
    println!("  Space/Shift - Move up/down (Y)");
    println!("  Q/E         - Move ana/kata (W axis)");
    println!("  Mouse       - Look around");
    println!("  Scroll      - Adjust W-slice");
    println!("  R           - Reset camera");
    println!();
    println!("Hot-reload: File changes are detected automatically");
    println!("===========================================");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = PreviewApp::new(asset_path);
    event_loop.run_app(&mut app).expect("Event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_asset_defaults() {
        let ron_str = r#"(
            name: "Test",
            mesh: Tesseract,
        )"#;

        let asset: PreviewAsset = ron::from_str(ron_str).unwrap();
        assert_eq!(asset.name, "Test");
        assert_eq!(asset.position, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(asset.scale, 1.0);
    }

    #[test]
    fn test_preview_asset_with_position() {
        let ron_str = r#"(
            name: "Offset Test",
            mesh: Tesseract,
            position: [1.0, 2.0, 3.0, 4.0],
            scale: 2.5,
        )"#;

        let asset: PreviewAsset = ron::from_str(ron_str).unwrap();
        assert_eq!(asset.name, "Offset Test");
        assert_eq!(asset.position, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(asset.scale, 2.5);
    }

    #[test]
    fn test_preview_mesh_hypersphere() {
        let ron_str = r#"(
            name: "Sphere",
            mesh: Hypersphere(segments: 16),
        )"#;

        let asset: PreviewAsset = ron::from_str(ron_str).unwrap();
        match asset.mesh {
            PreviewMesh::Hypersphere { segments } => {
                assert_eq!(segments, 16);
            }
            _ => panic!("Expected Hypersphere mesh"),
        }
    }

    #[test]
    fn test_preview_asset_serialization_roundtrip() {
        let asset = PreviewAsset {
            name: "Roundtrip Test".to_string(),
            mesh: PreviewMesh::Tesseract,
            position: [1.0, 2.0, 3.0, 0.5],
            scale: 1.5,
        };

        let serialized = ron::to_string(&asset).unwrap();
        let deserialized: PreviewAsset = ron::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, asset.name);
        assert_eq!(deserialized.position, asset.position);
        assert_eq!(deserialized.scale, asset.scale);
    }
}
