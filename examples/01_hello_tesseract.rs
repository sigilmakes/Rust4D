//! 01 - Hello Tesseract
//!
//! The simplest Rust4D example: display a single 4D tesseract (hypercube).
//!
//! This example demonstrates:
//! - Creating a window with winit
//! - Setting up the 4D rendering pipeline
//! - Creating a World with a single tesseract entity
//! - Using Camera4D for viewing 4D space
//! - Running a basic render loop
//!
//! Run with: `cargo run --example 01_hello_tesseract`

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
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

/// Application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    world: World,
    geometry: RenderableGeometry,
    camera: Camera4D,
}

impl App {
    fn new() -> Self {
        // Create a world with a single tesseract at the origin
        let mut world = World::new();

        // Create a tesseract (4D hypercube) with size 2.0
        let tesseract = Tesseract4D::new(2.0);
        world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::from_rgb(0.8, 0.4, 0.2), // Orange color
            DirtyFlags::ALL,
        ));

        // Build GPU geometry from the world
        let geometry = RenderableGeometry::from_world(&world);

        // Set up camera looking at the origin from a distance
        let mut camera = Camera4D::new();
        camera.position = Vec4::new(0.0, 2.0, 6.0, 0.0);

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            world,
            geometry,
            camera,
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
                            .with_title("Rust4D - Hello Tesseract")
                            .with_inner_size(winit::dpi::LogicalSize::new(800, 600)),
                    )
                    .expect("Failed to create window"),
            );

            // Initialize rendering
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
            WindowEvent::RedrawRequested => {
                // Render frame
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
                        view_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                        projection_matrix: perspective_matrix(std::f32::consts::FRAC_PI_4, ctx.aspect_ratio(), 0.1, 100.0),
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
                    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    sp.reset_counter(&ctx.queue);
                    sp.run_slice_pass(&mut encoder);
                    rp.prepare_indirect_draw(&mut encoder, sp.counter_buffer());
                    rp.render(&mut encoder, &view, sp.output_buffer(), wgpu::Color { r: 0.02, g: 0.02, b: 0.08, a: 1.0 });

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
