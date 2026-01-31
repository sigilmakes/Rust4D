//! Rust4D - 4D Rendering Engine
//!
//! A 4D rendering engine that displays 3D cross-sections of 4D geometry.

mod config;
mod input;
mod systems;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::WindowId,
};

use input::{InputMapper, InputAction};
use systems::{RenderError, RenderSystem, SimulationSystem, WindowSystem};

use rust4d_core::{World, SceneManager, Transform4D, ShapeRef, Material, Tags};
use rust4d_render::{
    camera4d::Camera4D,
    RenderableGeometry, CheckerboardGeometry, position_gradient_color,
};
use rust4d_input::CameraController;
use rust4d_math::Vec4;

use config::AppConfig;

/// Main application state
struct App {
    /// Application configuration
    config: AppConfig,
    /// Window system (created on resume)
    window_system: Option<WindowSystem>,
    /// Render system (created on resume)
    render_system: Option<RenderSystem>,
    /// Scene manager handling scene stack and physics
    scene_manager: SceneManager,
    /// Cached GPU geometry (rebuilt when world changes)
    geometry: RenderableGeometry,
    camera: Camera4D,
    controller: CameraController,
    /// Simulation system for game loop
    simulation: SimulationSystem,
}

impl App {
    fn new() -> Self {
        // Load configuration
        let config = AppConfig::load().unwrap_or_else(|e| {
            log::warn!("Failed to load config: {}. Using defaults.", e);
            AppConfig::default()
        });

        // Create scene manager and load scene from file
        // Pass physics config from TOML to the physics engine
        let mut scene_manager = SceneManager::new()
            .with_player_radius(config.scene.player_radius)
            .with_physics(config.physics.to_physics_config());

        // Load scene from configured path
        let scene_name = scene_manager.load_scene(&config.scene.path)
            .unwrap_or_else(|e| {
                panic!("Failed to load scene '{}': {}", config.scene.path, e);
            });

        // Instantiate and activate the scene
        scene_manager.instantiate(&scene_name)
            .unwrap_or_else(|e| panic!("Failed to instantiate scene: {}", e));
        scene_manager.push_scene(&scene_name)
            .unwrap_or_else(|e| panic!("Failed to push scene: {}", e));

        // Get player start from scene's player_spawn
        let player_start = scene_manager.active_scene()
            .and_then(|s| s.player_spawn)
            .map(|spawn| Vec4::new(spawn[0], spawn[1], spawn[2], spawn[3]))
            .unwrap_or_else(|| Vec4::new(
                config.camera.start_position[0],
                config.camera.start_position[1],
                config.camera.start_position[2],
                config.camera.start_position[3],
            ));

        // Build GPU geometry from the world
        let geometry = Self::build_geometry(scene_manager.active_world().unwrap());

        log::info!("Loaded scene '{}' with {} entities",
            scene_name,
            scene_manager.active_world().map(|w| w.entity_count()).unwrap_or(0));
        log::info!("Total geometry: {} vertices, {} tetrahedra",
            geometry.vertex_count(), geometry.tetrahedron_count());

        // Set camera with configured pitch limit and player start position
        let mut camera = Camera4D::with_pitch_limit(config.camera.pitch_limit.to_radians());
        camera.position = player_start;

        // Configure controller from config
        let controller = CameraController::new()
            .with_move_speed(config.input.move_speed)
            .with_w_move_speed(config.input.w_move_speed)
            .with_mouse_sensitivity(config.input.mouse_sensitivity)
            .with_w_rotation_sensitivity(config.input.w_rotation_sensitivity)
            .with_smoothing_half_life(config.input.smoothing_half_life)
            .with_smoothing(config.input.smoothing_enabled);

        Self {
            config,
            window_system: None,
            render_system: None,
            scene_manager,
            geometry,
            camera,
            controller,
            simulation: SimulationSystem::new(),
        }
    }

    /// Build GPU geometry from the world using custom coloring
    fn build_geometry(world: &World) -> RenderableGeometry {
        let mut geometry = RenderableGeometry::new();

        // Checkerboard pattern for the floor
        let checkerboard = CheckerboardGeometry::new(
            [0.3, 0.3, 0.35, 1.0], // Dark gray
            [0.7, 0.7, 0.75, 1.0], // Light gray
            2.0, // Cell size
        );

        // Query all renderable entities (Transform4D + ShapeRef + Material)
        // Optionally check Tags for coloring strategy
        for (_entity, (transform, shape, material, tags)) in
            world.ecs().query::<(&Transform4D, &ShapeRef, &Material, Option<&Tags>)>().iter()
        {
            let is_dynamic = tags.map(|t| t.has("dynamic")).unwrap_or(false);
            if is_dynamic {
                // Dynamic entities (tesseract): use position gradient
                geometry.add_components_with_color(transform, shape.as_shape(), material, &position_gradient_color);
            } else {
                // Static entities (floor): use checkerboard pattern
                geometry.add_components_with_color(transform, shape.as_shape(), material, &|v, _m| {
                    checkerboard.color_for_position(v.x, v.z)
                });
            }
        }

        geometry
    }

}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_system.is_none() {
            // Create window system
            let window_system = WindowSystem::create(event_loop, &self.config.window)
                .expect("Failed to create window");

            // Create render system
            let mut render_system = RenderSystem::new(
                window_system.window().clone(),
                self.config.rendering.clone(),
                self.config.camera.clone(),
                self.config.window.vsync,
            );

            // Upload initial geometry
            render_system.upload_geometry(&self.geometry);

            self.window_system = Some(window_system);
            self.render_system = Some(render_system);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(rs) = &mut self.render_system {
                    rs.resize(physical_size.width, physical_size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    // Map to action via InputMapper
                    let cursor_captured = self.window_system.as_ref()
                        .map(|ws| ws.is_cursor_captured())
                        .unwrap_or(false);

                    if let Some(action) = InputMapper::map_keyboard(key, event.state, cursor_captured) {
                        match action {
                            InputAction::ToggleCursor => {
                                if let Some(ws) = &mut self.window_system {
                                    ws.release_cursor();
                                }
                            }
                            InputAction::Exit => {
                                event_loop.exit();
                            }
                            InputAction::ResetCamera => {
                                self.camera.reset();
                                log::info!("Camera reset to starting position");
                            }
                            InputAction::ToggleFullscreen => {
                                if let Some(ws) = &self.window_system {
                                    ws.toggle_fullscreen();
                                }
                            }
                            InputAction::ToggleSmoothing => {
                                let enabled = self.controller.toggle_smoothing();
                                log::info!("Input smoothing: {}", if enabled { "ON" } else { "OFF" });
                            }
                        }
                        return;
                    }

                    // Pass to controller for movement keys
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Map to action via InputMapper
                let cursor_captured = self.window_system.as_ref()
                    .map(|ws| ws.is_cursor_captured())
                    .unwrap_or(false);

                if let Some(action) = InputMapper::map_mouse_button(button, state, cursor_captured) {
                    if action == InputAction::ToggleCursor {
                        if let Some(ws) = &mut self.window_system {
                            ws.capture_cursor();
                        }
                    }
                }
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll wheel adjusts slice offset
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                // Run simulation
                let cursor_captured = self.window_system.as_ref()
                    .map(|ws| ws.is_cursor_captured())
                    .unwrap_or(false);

                let result = self.simulation.update(
                    &mut self.scene_manager,
                    &mut self.camera,
                    &mut self.controller,
                    cursor_captured,
                );

                // Rebuild geometry if entities changed
                if result.geometry_dirty {
                    self.geometry = Self::build_geometry(self.scene_manager.active_world().unwrap());
                    if let Some(rs) = &mut self.render_system {
                        rs.upload_geometry(&self.geometry);
                    }
                    if let Some(w) = self.scene_manager.active_world_mut() {
                        w.clear_all_dirty();
                    }
                }

                // Update window title with debug info
                if let Some(ws) = &self.window_system {
                    let pos = self.camera.position;
                    ws.update_title([pos.x, pos.y, pos.z, pos.w], self.camera.get_slice_w());
                }

                // Render frame
                if let Some(rs) = &mut self.render_system {
                    match rs.render_frame(&self.camera, &self.geometry) {
                        Ok(()) => {}
                        Err(RenderError::SurfaceLost) => {
                            let (w, h) = rs.size();
                            rs.resize(w, h);
                        }
                        Err(RenderError::OutOfMemory) => {
                            event_loop.exit();
                            return;
                        }
                        Err(e) => {
                            log::warn!("Render error: {}", e);
                        }
                    }
                }

                // Request next frame
                if let Some(ws) = &self.window_system {
                    ws.request_redraw();
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
    // Initialize logging
    env_logger::init();
    log::info!("Starting Rust4D");

    // Create event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create and run application
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
