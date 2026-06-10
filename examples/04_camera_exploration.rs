//! 04 - Camera Exploration
//!
//! A full-featured example with complete camera controls for exploring 4D space.
//!
//! This example demonstrates:
//! - Using CameraController from rust4d_input for FPS-style controls
//! - Mouse look with cursor capture/release
//! - Full 4D navigation (WASD + Q/E for W-axis)
//! - Multiple tesseracts at different 4D positions
//! - A floor for spatial reference
//! - Dynamic window title showing camera position
//!
//! Controls:
//! - Click to capture cursor, Escape to release
//! - WASD: Move in XZ plane (forward/backward/strafe)
//! - Space/Shift: Move up/down (Y-axis)
//! - Q/E: Move along W-axis (4th dimension - ana/kata)
//! - Mouse: Look around (when cursor captured)
//! - Right-click + drag: W-axis rotation (4D rotation)
//! - Scroll: Adjust slice offset
//! - R: Reset camera
//! - F: Toggle fullscreen
//! - G: Toggle input smoothing
//!
//! Run with: `cargo run --example 04_camera_exploration`

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

use rust4d_core::{
    Material, ShapeRef, Tesseract4D, Transform4D, DirtyFlags, World, Tags, Name,
    Hyperplane4D,
};
use rust4d_render::{
    camera4d::Camera4D,
    context::RenderContext,
    pipeline::{perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline, MAX_OUTPUT_TRIANGLES},
    RenderableGeometry, CheckerboardGeometry, position_gradient_color,
};
use rust4d_math::Vec4;
use rust4d_input::CameraController;

/// Application state with full camera controller integration
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    #[allow(dead_code)] // keeps ECS world alive alongside its derived GPU geometry
    world: World,
    geometry: RenderableGeometry,
    camera: Camera4D,
    controller: CameraController,
    last_frame: std::time::Instant,
    cursor_captured: bool,
}

impl App {
    fn new() -> Self {
        let mut world = World::new();

        // Add floor at Y = -2 for spatial reference (shape at y=0 local, positioned by transform)
        let floor_shape = Hyperplane4D::new(20.0, 12, 2.0, 0.001);
        let floor_transform = Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0));
        world.spawn((
            ShapeRef::shared(floor_shape),
            floor_transform,
            Material::GRAY,
            DirtyFlags::ALL,
            Name::new("floor"),
            Tags::new().with_tag("static"),
        ));

        // Create tesseracts at various 4D positions to explore
        // Each one at a different location in 4D space
        let tesseracts = [
            // XYZ positions (W=0) - visible immediately
            (Vec4::new(0.0, 0.0, 0.0, 0.0), Material::from_rgb(0.9, 0.4, 0.2), "origin"),
            (Vec4::new(5.0, 0.0, 0.0, 0.0), Material::from_rgb(0.2, 0.9, 0.3), "right"),
            (Vec4::new(-5.0, 0.0, 0.0, 0.0), Material::from_rgb(0.2, 0.3, 0.9), "left"),
            (Vec4::new(0.0, 3.0, 0.0, 0.0), Material::from_rgb(0.9, 0.9, 0.2), "above"),
            (Vec4::new(0.0, 0.0, -5.0, 0.0), Material::from_rgb(0.2, 0.9, 0.9), "forward"),
            // Tesseracts at different W positions (use Q/E to find them!)
            (Vec4::new(3.0, 0.0, 3.0, 2.0), Material::from_rgb(0.9, 0.2, 0.9), "w+2"),
            (Vec4::new(-3.0, 0.0, 3.0, -2.0), Material::from_rgb(0.6, 0.3, 0.9), "w-2"),
            (Vec4::new(0.0, 1.0, -3.0, 4.0), Material::from_rgb(0.3, 0.6, 0.9), "w+4"),
        ];

        for (position, material, name) in tesseracts {
            let tesseract = Tesseract4D::new(1.5);
            let transform = Transform4D::from_position(position);
            world.spawn((
                ShapeRef::shared(tesseract),
                transform,
                material,
                DirtyFlags::ALL,
                Name::new(name),
                Tags::new().with_tag("dynamic"),
            ));
        }

        let geometry = Self::build_geometry(&world);

        // Start camera at a good observation position
        let mut camera = Camera4D::new();
        camera.position = Vec4::new(0.0, 2.0, 10.0, 0.0);

        // Configure controller with reasonable defaults
        let controller = CameraController::new()
            .with_move_speed(5.0)
            .with_w_move_speed(3.0)
            .with_mouse_sensitivity(0.002)
            .with_smoothing(false);

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            world,
            geometry,
            camera,
            controller,
            last_frame: std::time::Instant::now(),
            cursor_captured: false,
        }
    }

    /// Build geometry with checkerboard floor and gradient tesseracts
    fn build_geometry(world: &World) -> RenderableGeometry {
        let mut geometry = RenderableGeometry::new();

        let checkerboard = CheckerboardGeometry::new(
            [0.25, 0.25, 0.30, 1.0],
            [0.55, 0.55, 0.60, 1.0],
            2.0,
        );

        for (_entity, (transform, shape, material, tags)) in
            world.ecs().query::<(&Transform4D, &ShapeRef, &Material, Option<&Tags>)>().iter()
        {
            let is_dynamic = tags.map(|t| t.has("dynamic")).unwrap_or(false);
            if is_dynamic {
                geometry.add_components_with_color(transform, shape.as_shape(), material, &position_gradient_color);
            } else {
                geometry.add_components_with_color(transform, shape.as_shape(), material, &|v, _m| {
                    checkerboard.color_for_position(v.x, v.z)
                });
            }
        }

        geometry
    }

    /// Capture cursor for FPS-style controls
    fn capture_cursor(&mut self) {
        if let Some(window) = &self.window {
            let grab_result = window.set_cursor_grab(CursorGrabMode::Locked)
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Rust4D - Camera Exploration [Click to capture cursor]")
                            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                    )
                    .expect("Failed to create window"),
            );

            let render_context = pollster::block_on(RenderContext::new(window.clone()));
            let mut slice_pipeline = SlicePipeline::new(&render_context.device, MAX_OUTPUT_TRIANGLES);
            let mut render_pipeline =
                RenderPipeline::new(&render_context.device, render_context.config.format);

            render_pipeline.ensure_depth_texture(
                &render_context.device,
                render_context.size.width,
                render_context.size.height,
            );

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
                    // Handle special keys on press
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
                                // Reset camera to starting position
                                self.camera.reset();
                                self.camera.position = Vec4::new(0.0, 2.0, 10.0, 0.0);
                            }
                            KeyCode::KeyF => {
                                // Toggle fullscreen
                                if let Some(window) = &self.window {
                                    let new_fullscreen = if window.fullscreen().is_some() {
                                        None
                                    } else {
                                        Some(Fullscreen::Borderless(None))
                                    };
                                    window.set_fullscreen(new_fullscreen);
                                }
                            }
                            KeyCode::KeyG => {
                                // Toggle input smoothing
                                let enabled = self.controller.toggle_smoothing();
                                println!("Input smoothing: {}", if enabled { "ON" } else { "OFF" });
                            }
                            _ => {}
                        }
                    }
                    // Pass all keyboard input to controller for movement
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Click to capture cursor
                if state == ElementState::Pressed && button == MouseButton::Left && !self.cursor_captured {
                    self.capture_cursor();
                }
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll wheel adjusts slice offset (W position for slicing)
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                // Calculate delta time
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Update camera via controller
                self.controller.update(&mut self.camera, dt, self.cursor_captured);

                // Update window title with position info
                if let Some(window) = &self.window {
                    let pos = self.camera.position;
                    let slice_w = self.camera.get_slice_w();
                    let title = if self.cursor_captured {
                        format!(
                            "Rust4D - Pos: ({:.1}, {:.1}, {:.1}, W:{:.1}) Slice:{:.2} [ESC to release]",
                            pos.x, pos.y, pos.z, pos.w, slice_w
                        )
                    } else {
                        format!(
                            "Rust4D - Pos: ({:.1}, {:.1}, {:.1}, W:{:.1}) Slice:{:.2} [Click to capture]",
                            pos.x, pos.y, pos.z, pos.w, slice_w
                        )
                    };
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
                        camera_matrix: self.camera.rotation_matrix(),
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
                            r: 0.02,
                            g: 0.02,
                            b: 0.08,
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
        // Process raw mouse motion for smoother camera control
        if let DeviceEvent::MouseMotion { delta } = event {
            self.controller.process_mouse_motion(delta.0, delta.1);
        }
    }
}

fn main() {
    env_logger::init();
    println!("Rust4D Camera Exploration");
    println!("=========================");
    println!("Click to capture cursor, Escape to release");
    println!("WASD: Move | Q/E: Move in W-axis (4th dimension)");
    println!("Mouse: Look | Right-click+drag: 4D rotation");
    println!("Space/Shift: Up/Down | R: Reset | F: Fullscreen");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
