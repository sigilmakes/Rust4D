//! GPU rendering system
//!
//! Manages GPU rendering including:
//! - Render context and surface
//! - Slice and render pipelines
//! - Frame rendering

use std::sync::Arc;
use winit::window::Window;
use rust4d_render::{
    context::RenderContext,
    camera4d::Camera4D,
    pipeline::{perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline},
    RenderableGeometry,
};
use crate::config::{CameraConfig, RenderingConfig};

/// Render error types
#[derive(Debug)]
pub enum RenderError {
    /// Surface was lost (window resized, minimized, etc.)
    SurfaceLost,
    /// GPU out of memory
    OutOfMemory,
    /// Other surface error
    Other(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::SurfaceLost => write!(f, "Surface lost"),
            RenderError::OutOfMemory => write!(f, "Out of memory"),
            RenderError::Other(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for RenderError {}

/// Manages GPU rendering
pub struct RenderSystem {
    context: RenderContext,
    slice_pipeline: SlicePipeline,
    render_pipeline: RenderPipeline,
    render_config: RenderingConfig,
    camera_config: CameraConfig,
}

impl RenderSystem {
    /// Create render system from window and config
    pub fn new(
        window: Arc<Window>,
        render_config: RenderingConfig,
        camera_config: CameraConfig,
        vsync: bool,
    ) -> Self {
        let context = pollster::block_on(RenderContext::with_vsync(window, vsync));

        let slice_pipeline = SlicePipeline::new(
            &context.device,
            render_config.max_triangles as usize,
        );

        let mut render_pipeline = RenderPipeline::new(&context.device, context.config.format);

        // Ensure depth texture exists
        render_pipeline.ensure_depth_texture(
            &context.device,
            context.size.width,
            context.size.height,
        );

        Self {
            context,
            slice_pipeline,
            render_pipeline,
            render_config,
            camera_config,
        }
    }

    /// Handle window resize
    pub fn resize(&mut self, width: u32, height: u32) {
        self.context
            .resize(winit::dpi::PhysicalSize::new(width, height));
        self.render_pipeline.ensure_depth_texture(&self.context.device, width, height);
    }

    /// Upload geometry to GPU
    pub fn upload_geometry(&mut self, geometry: &RenderableGeometry) {
        self.slice_pipeline.upload_tetrahedra(
            &self.context.device,
            &geometry.vertices,
            &geometry.tetrahedra,
        );
        // trace, not info: dynamic scenes re-upload every frame while bodies move
        log::trace!(
            "Uploaded {} vertices and {} tetrahedra",
            geometry.vertex_count(),
            geometry.tetrahedron_count()
        );
    }

    /// Render a single frame
    pub fn render_frame(
        &mut self,
        camera: &Camera4D,
        geometry: &RenderableGeometry,
    ) -> Result<(), RenderError> {
        let pos = camera.position;
        let camera_pos_4d = [pos.x, pos.y, pos.z, pos.w];

        // Update slice parameters
        let camera_matrix = camera.rotation_matrix();
        let slice_params = SliceParams {
            slice_w: camera.get_slice_w(),
            tetrahedron_count: geometry.tetrahedron_count() as u32,
            _padding: [0.0; 2],
            camera_matrix,
            camera_position: camera_pos_4d,
        };
        self.slice_pipeline
            .update_params(&self.context.queue, &slice_params);

        // Create view and projection matrices
        let aspect = self.context.aspect_ratio();
        let proj_matrix = perspective_matrix(
            self.camera_config.fov.to_radians(),
            aspect,
            self.camera_config.near,
            self.camera_config.far,
        );

        // View matrix is identity (slice shader outputs camera-space coordinates)
        let view_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let render_uniforms = RenderUniforms {
            view_matrix,
            projection_matrix: proj_matrix,
            light_dir: self.render_config.light_dir,
            _padding: 0.0,
            ambient_strength: self.render_config.ambient_strength,
            diffuse_strength: self.render_config.diffuse_strength,
            w_color_strength: self.render_config.w_color_strength,
            w_range: self.render_config.w_range,
            ..RenderUniforms::default()
        };
        self.render_pipeline
            .update_uniforms(&self.context.queue, &render_uniforms);

        // Get surface texture
        let output = match self.context.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => return Err(RenderError::SurfaceLost),
            Err(wgpu::SurfaceError::OutOfMemory) => return Err(RenderError::OutOfMemory),
            Err(e) => return Err(RenderError::Other(format!("{:?}", e))),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Reset counter and run compute pass
        self.slice_pipeline.reset_counter(&self.context.queue);
        self.slice_pipeline.run_slice_pass(&mut encoder);

        // Copy triangle count to indirect buffer
        self.render_pipeline
            .prepare_indirect_draw(&mut encoder, self.slice_pipeline.counter_buffer());

        // Render pass
        let bg = &self.render_config.background_color;
        self.render_pipeline.render(
            &mut encoder,
            &view,
            self.slice_pipeline.output_buffer(),
            wgpu::Color {
                r: bg[0] as f64,
                g: bg[1] as f64,
                b: bg[2] as f64,
                a: bg[3] as f64,
            },
        );

        // Submit
        self.context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Get current surface size
    pub fn size(&self) -> (u32, u32) {
        (self.context.size.width, self.context.size.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_error_display() {
        assert_eq!(format!("{}", RenderError::SurfaceLost), "Surface lost");
        assert_eq!(format!("{}", RenderError::OutOfMemory), "Out of memory");
        assert_eq!(
            format!("{}", RenderError::Other("test".to_string())),
            "Render error: test"
        );
    }
}
