//! egui renderer integration with wgpu
//!
//! Provides the [`EguiRenderer`] type that wraps egui-wgpu for rendering
//! 2D UI on top of the 4D scene.

use egui::{Context, FullOutput, RawInput};
use egui_wgpu::{Renderer, ScreenDescriptor};

/// Manages egui state and rendering
///
/// This struct wraps the egui [`Context`] and the wgpu [`Renderer`] to provide
/// a simple interface for rendering egui UI on top of the 4D scene.
///
/// # Example
///
/// ```ignore
/// let mut egui_renderer = EguiRenderer::new(&device, surface_format);
///
/// // In game loop:
/// let ctx = egui_renderer.begin_frame(RawInput::default());
/// egui::Window::new("Debug").show(ctx, |ui| {
///     ui.label("Hello from egui!");
/// });
/// let output = egui_renderer.end_frame();
/// egui_renderer.render(&mut encoder, &view, &screen_desc, &device, &queue, output);
/// ```
pub struct EguiRenderer {
    ctx: Context,
    renderer: Renderer,
}

impl EguiRenderer {
    /// Create a new egui renderer
    ///
    /// # Arguments
    ///
    /// * `device` - wgpu Device for creating GPU resources
    /// * `output_format` - Surface texture format for rendering
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let ctx = Context::default();
        let renderer = Renderer::new(device, output_format, None, 1, false);

        Self { ctx, renderer }
    }

    /// Begin a new frame
    ///
    /// Returns a reference to the egui [`Context`] that can be used for drawing UI.
    /// After drawing, call [`end_frame`](Self::end_frame) to finalize the frame.
    ///
    /// # Arguments
    ///
    /// * `raw_input` - Input events for this frame (from winit or custom source)
    pub fn begin_frame(&mut self, raw_input: RawInput) -> &Context {
        self.ctx.begin_pass(raw_input);
        &self.ctx
    }

    /// End the frame and get the output
    ///
    /// Returns the [`FullOutput`] containing shapes to render and textures to update.
    pub fn end_frame(&mut self) -> FullOutput {
        self.ctx.end_pass()
    }

    /// Render the egui output to a render pass
    ///
    /// This method handles texture updates and renders the UI primitives.
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder for recording render commands
    /// * `render_target` - Texture view to render to
    /// * `screen_descriptor` - Screen dimensions and scale factor
    /// * `device` - wgpu Device for creating temporary resources
    /// * `queue` - wgpu Queue for submitting texture updates
    /// * `full_output` - Output from [`end_frame`](Self::end_frame)
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        screen_descriptor: &ScreenDescriptor,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        full_output: FullOutput,
    ) {
        // Handle texture updates
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // Tessellate shapes into primitives
        let clipped_primitives = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        // Update buffers
        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &clipped_primitives,
            screen_descriptor,
        );

        // Render
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load existing content (scene)
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // egui-wgpu requires RenderPass<'static> - use forget_lifetime to convert
            // SAFETY: The render pass internally keeps all referenced resources alive
            let mut render_pass = render_pass.forget_lifetime();

            self.renderer
                .render(&mut render_pass, &clipped_primitives, screen_descriptor);
        }

        // Free textures that are no longer needed
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }

    /// Get the egui context for external use
    ///
    /// This can be used to configure egui settings, fonts, or styles.
    pub fn context(&self) -> &Context {
        &self.ctx
    }
}
