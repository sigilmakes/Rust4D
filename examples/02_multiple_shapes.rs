//! 02 - Multiple Shapes
//!
//! Demonstrates multiple tesseracts with different materials and basic WASD movement.
//!
//! This example shows:
//! - Creating multiple entities with different positions and colors
//! - Using Transform4D for positioning
//! - Basic keyboard movement (WASD + QE for W-axis)
//! - Camera position in world space
//!
//! Run with: `cargo run --example 02_multiple_shapes`

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use rust4d_core::{Material, ShapeRef, Tesseract4D, Transform4D, DirtyFlags, World};
use rust4d_render::{
    camera4d::Camera4D,
    context::RenderContext,
    pipeline::{perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline, MAX_OUTPUT_TRIANGLES},
    RenderableGeometry,
};
use rust4d_math::Vec4;

/// Movement state
struct Movement {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    ana: bool,   // Q - move toward +W
    kata: bool,  // E - move toward -W
}

impl Movement {
    fn new() -> Self {
        Self {
            forward: false, backward: false, left: false, right: false,
            up: false, down: false, ana: false, kata: false,
        }
    }
}

/// Application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    world: World,
    geometry: RenderableGeometry,
    camera: Camera4D,
    movement: Movement,
    last_frame: std::time::Instant,
}

impl App {
    fn new() -> Self {
        let mut world = World::new();

        // Create multiple tesseracts with different colors and positions
        let positions_and_colors = [
            (Vec4::new(0.0, 0.0, 0.0, 0.0), Material::from_rgb(0.9, 0.3, 0.2)),   // Red at origin
            (Vec4::new(4.0, 0.0, 0.0, 0.0), Material::from_rgb(0.2, 0.9, 0.3)),   // Green to the right
            (Vec4::new(-4.0, 0.0, 0.0, 0.0), Material::from_rgb(0.2, 0.3, 0.9)), // Blue to the left
            (Vec4::new(0.0, 4.0, 0.0, 0.0), Material::from_rgb(0.9, 0.9, 0.2)),  // Yellow above
            (Vec4::new(0.0, 0.0, 0.0, 4.0), Material::from_rgb(0.9, 0.2, 0.9)),  // Magenta in +W
        ];

        for (position, material) in positions_and_colors {
            let tesseract = Tesseract4D::new(1.5);
            let transform = Transform4D::from_position(position);
            world.spawn((
                ShapeRef::shared(tesseract),
                transform,
                material,
                DirtyFlags::ALL,
            ));
        }

        let geometry = RenderableGeometry::from_world(&world);

        let mut camera = Camera4D::new();
        camera.position = Vec4::new(0.0, 2.0, 10.0, 0.0);

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            world,
            geometry,
            camera,
            movement: Movement::new(),
            last_frame: std::time::Instant::now(),
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
                            .with_title("Rust4D - Multiple Shapes (WASD to move, QE for W-axis)")
                            .with_inner_size(winit::dpi::LogicalSize::new(1024, 768)),
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
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = event.physical_key {
                    match key {
                        KeyCode::KeyW => self.movement.forward = pressed,
                        KeyCode::KeyS => self.movement.backward = pressed,
                        KeyCode::KeyA => self.movement.left = pressed,
                        KeyCode::KeyD => self.movement.right = pressed,
                        KeyCode::Space => self.movement.up = pressed,
                        KeyCode::ShiftLeft => self.movement.down = pressed,
                        KeyCode::KeyQ => self.movement.ana = pressed,
                        KeyCode::KeyE => self.movement.kata = pressed,
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // Calculate delta time
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Apply movement
                let speed = 5.0;
                let forward = (self.movement.forward as i32 - self.movement.backward as i32) as f32;
                let right = (self.movement.right as i32 - self.movement.left as i32) as f32;
                let up = (self.movement.up as i32 - self.movement.down as i32) as f32;
                let w = (self.movement.ana as i32 - self.movement.kata as i32) as f32;

                self.camera.move_local_xz(forward * speed * dt, right * speed * dt);
                self.camera.move_y(up * speed * dt);
                self.camera.move_w(w * speed * dt);

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
                        view_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0],
                                      [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                        projection_matrix: perspective_matrix(
                            std::f32::consts::FRAC_PI_4, ctx.aspect_ratio(), 0.1, 100.0,
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
                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = ctx.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor { label: None },
                    );

                    sp.reset_counter(&ctx.queue);
                    sp.run_slice_pass(&mut encoder);
                    rp.prepare_indirect_draw(&mut encoder, sp.counter_buffer());
                    rp.render(&mut encoder, &view, sp.output_buffer(),
                        wgpu::Color { r: 0.02, g: 0.02, b: 0.08, a: 1.0 });

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
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
