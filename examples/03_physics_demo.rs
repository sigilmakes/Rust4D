//! 03 - Physics Demo
//!
//! Demonstrates 4D physics with gravity and collision detection.
//!
//! This example shows:
//! - Creating a World with physics enabled
//! - Adding a floor with collision
//! - Dynamic physics bodies (tesseracts that fall and bounce)
//! - RigidBody4D and PhysicsConfig integration
//! - Real-time geometry updates as physics moves objects
//!
//! Run with: `cargo run --example 03_physics_demo`

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use rust4d_core::{
    Material, ShapeRef, Tesseract4D, Transform4D, DirtyFlags, World, Tags, Name,
    PhysicsConfig, PhysicsBody, RigidBody4D, StaticCollider, Hyperplane4D,
};
use rust4d_render::{
    camera4d::Camera4D,
    context::RenderContext,
    pipeline::{perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline, MAX_OUTPUT_TRIANGLES},
    RenderableGeometry, CheckerboardGeometry, position_gradient_color,
};
use rust4d_math::Vec4;
use rust4d_physics::{BodyType, PhysicsMaterial};
use rust4d_core::hecs;

/// Application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    world: World,
    geometry: RenderableGeometry,
    camera: Camera4D,
    last_frame: std::time::Instant,
}

impl App {
    fn new() -> Self {
        // Create world with physics (gravity = -15 for visible falling)
        let config = PhysicsConfig::new(-15.0);
        let mut world = World::new().with_physics(config);

        // Add floor at Y = -3
        let floor_y = -3.0;
        if let Some(physics) = world.physics_mut() {
            physics.add_static_collider(StaticCollider::floor(floor_y, PhysicsMaterial::RUBBER));
        }

        // Add visual floor entity (shape at y=0 local, positioned by transform)
        let floor_shape = Hyperplane4D::new(15.0, 10, 2.0, 0.001);
        let floor_transform = Transform4D::from_position(Vec4::new(0.0, floor_y, 0.0, 0.0));
        world.spawn((
            ShapeRef::shared(floor_shape),
            floor_transform,
            Material::GRAY,
            DirtyFlags::ALL,
            Name::new("floor"),
            Tags::new().with_tag("static"),
        ));

        // Add falling tesseracts at different heights
        let spawn_positions = [
            (Vec4::new(0.0, 5.0, 0.0, 0.0), Material::from_rgb(0.9, 0.3, 0.2)),
            (Vec4::new(3.0, 8.0, 0.0, 0.0), Material::from_rgb(0.2, 0.9, 0.3)),
            (Vec4::new(-3.0, 11.0, 0.0, 0.0), Material::from_rgb(0.2, 0.3, 0.9)),
            (Vec4::new(0.0, 14.0, 2.0, 0.0), Material::from_rgb(0.9, 0.9, 0.2)),
            (Vec4::new(1.5, 17.0, -1.5, 0.0), Material::from_rgb(0.9, 0.2, 0.9)),
        ];

        for (i, (position, material)) in spawn_positions.iter().enumerate() {
            let size = 1.2;
            let half_extent = size / 2.0;

            // Add physics body
            let body_key = if let Some(physics) = world.physics_mut() {
                let body = RigidBody4D::new_aabb(
                    *position,
                    Vec4::new(half_extent, half_extent, half_extent, half_extent),
                )
                .with_body_type(BodyType::Dynamic)
                .with_mass(5.0)
                .with_material(PhysicsMaterial::RUBBER);
                Some(physics.add_body(body))
            } else {
                None
            };

            // Add visual entity with ECS components
            let tesseract = Tesseract4D::new(size);
            let transform = Transform4D::from_position(*position);
            let mut builder = hecs::EntityBuilder::new();
            builder.add(ShapeRef::shared(tesseract));
            builder.add(transform);
            builder.add(*material);
            builder.add(DirtyFlags::ALL);
            builder.add(Name::new(format!("tesseract_{}", i)));
            builder.add(Tags::new().with_tag("dynamic"));
            if let Some(key) = body_key {
                builder.add(PhysicsBody(key));
            }
            world.spawn(builder.build());
        }

        let geometry = Self::build_geometry(&world);

        let mut camera = Camera4D::new();
        camera.position = Vec4::new(0.0, 5.0, 18.0, 0.0);

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            world,
            geometry,
            camera,
            last_frame: std::time::Instant::now(),
        }
    }

    /// Build geometry with custom coloring (checkerboard floor, gradient tesseracts)
    fn build_geometry(world: &World) -> RenderableGeometry {
        let mut geometry = RenderableGeometry::new();

        let checkerboard = CheckerboardGeometry::new(
            [0.3, 0.3, 0.35, 1.0],
            [0.6, 0.6, 0.65, 1.0],
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Rust4D - Physics Demo (Watch tesseracts fall!)")
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

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                        event_loop.exit();
                    }
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(ctx) = &mut self.render_context {
                    ctx.resize(size);
                }
                if let (Some(ctx), Some(rp)) = (&self.render_context, &mut self.render_pipeline) {
                    rp.ensure_depth_texture(&ctx.device, size.width, size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Step physics simulation
                self.world.update(dt);

                // Rebuild geometry if entities moved
                if self.world.has_dirty_entities() {
                    self.geometry = Self::build_geometry(&self.world);

                    if let (Some(sp), Some(ctx)) = (&mut self.slice_pipeline, &self.render_context) {
                        sp.upload_tetrahedra(
                            &ctx.device,
                            &self.geometry.vertices,
                            &self.geometry.tetrahedra,
                        );
                    }

                    self.world.clear_all_dirty();
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
