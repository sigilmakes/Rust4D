//! Headless visual showcase for the Rust4D primitive catalog.
//!
//! This is both a demo generator and a regression tool. It renders each
//! primitive through the real GPU slice + render pipelines into offscreen
//! textures and writes PPM frames, no window needed.
//!
//! Usage:
//!
//! ```bash
//! cargo run --example shape_showcase .scratchpad/captures-gallery
//! magick .scratchpad/captures-gallery/duocylinder_mid_identity.ppm duocylinder.png
//! ```
//!
//! What to look for:
//! - hypersphere: sphere grows/shrinks across W offsets
//! - spherinder: sphere remains roughly constant through its W tube
//! - duocylinder: a torus-like slice at W=0
//! - no cracks or hairline T-junctions (would indicate a broken primitive
//!   tetrahedralization despite the CPU watertightness tests)

use rust4d_core::{Material, ShapeTemplate, Transform4D};
use rust4d_render::camera4d::Camera4D;
use rust4d_render::pipeline::{
    perspective_matrix, RenderPipeline, RenderUniforms, SliceParams, SlicePipeline,
};
use rust4d_render::RenderableGeometry;

use std::path::{Path, PathBuf};

const WIDTH: u32 = 900;
const HEIGHT: u32 = 700;
const MAX_TRIANGLES: usize = 1_200_000;

struct ShowcaseShape {
    name: &'static str,
    template: ShapeTemplate,
    color: [f32; 4],
    radius: f32,
}

impl ShowcaseShape {
    fn geometry(&self) -> RenderableGeometry {
        let shape = self.template.create_shape();
        let mut geom = RenderableGeometry::new();
        let material = Material { base_color: self.color };
        geom.add_components_with_color(
            &Transform4D::identity(),
            shape.as_ref(),
            &material,
            &|_v, material| material.base_color,
        );
        geom
    }
}

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
                label: Some("Shape Showcase Headless Device"),
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
            label: Some("Shape Showcase Color"),
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

        let unpadded = WIDTH * 4;
        let padded_bytes_per_row = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Showcase Readback"),
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

    fn upload_geometry(&mut self, geometry: &RenderableGeometry) {
        self.slice_pipeline
            .upload_tetrahedra(&self.device, &geometry.vertices, &geometry.tetrahedra);
        println!(
            "[GPU] uploaded {} vertices, {} tetrahedra",
            geometry.vertex_count(),
            geometry.tetrahedron_count()
        );
    }

    fn capture(&mut self, camera: &Camera4D, tetrahedron_count: u32, path: &Path) -> u32 {
        let pos = camera.position;
        let slice_params = SliceParams {
            slice_w: camera.get_slice_w(),
            tetrahedron_count,
            _padding: [0.0; 2],
            camera_matrix: camera.rotation_matrix(),
            camera_position: [pos.x, pos.y, pos.z, pos.w],
        };
        self.slice_pipeline.update_params(&self.queue, &slice_params);

        let projection_matrix = perspective_matrix(
            40.0_f32.to_radians(),
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
                projection_matrix,
                light_dir: [0.4, 0.9, 0.35],
                _padding: 0.0,
                ambient_strength: 0.35,
                diffuse_strength: 0.75,
                w_color_strength: 0.25,
                w_range: 2.5,
            },
        );

        let view = self
            .color_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Shape Showcase Encoder"),
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
                r: 0.035,
                g: 0.035,
                b: 0.055,
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

        let vertex_count = self.read_counter();
        let tris = vertex_count / 3;
        println!(
            "[CAPTURE] {} slice={:.3} triangles={tris}",
            path.file_name().unwrap().to_string_lossy(),
            camera.get_slice_w(),
        );
        tris
    }

    fn read_counter(&self) -> u32 {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Showcase Counter Staging"),
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
            out.extend_from_slice(&data[px..px + 3]);
        }
    }
    std::fs::write(path, out).expect("failed to write ppm");
}

fn shapes() -> Vec<ShowcaseShape> {
    vec![
        ShowcaseShape {
            name: "tesseract",
            template: ShapeTemplate::tesseract(2.2),
            color: [1.0, 0.82, 0.32, 1.0],
            radius: 1.9,
        },
        ShowcaseShape {
            name: "hypersphere",
            template: ShapeTemplate::Hypersphere { radius: 1.25, subdivisions: 2 },
            color: [0.35, 0.70, 1.0, 1.0],
            radius: 1.25,
        },
        ShowcaseShape {
            name: "pentachoron",
            template: ShapeTemplate::Pentachoron { circumradius: 1.4 },
            color: [1.0, 0.38, 0.34, 1.0],
            radius: 1.4,
        },
        ShowcaseShape {
            name: "hexadecachoron",
            template: ShapeTemplate::Hexadecachoron { circumradius: 1.35 },
            color: [0.55, 1.0, 0.48, 1.0],
            radius: 1.35,
        },
        ShowcaseShape {
            name: "icositetrachoron",
            template: ShapeTemplate::Icositetrachoron { circumradius: 1.5 },
            color: [0.72, 0.45, 1.0, 1.0],
            radius: 1.5,
        },
        ShowcaseShape {
            name: "hexacosichoron",
            template: ShapeTemplate::Hexacosichoron { circumradius: 1.2 },
            color: [1.0, 0.55, 0.88, 1.0],
            radius: 1.2,
        },
        ShowcaseShape {
            name: "spherinder",
            template: ShapeTemplate::Spherinder { radius: 1.05, half_height: 1.25, subdivisions: 2 },
            color: [0.28, 0.95, 0.88, 1.0],
            radius: 1.65,
        },
        ShowcaseShape {
            name: "cubinder",
            template: ShapeTemplate::Cubinder { radius: 1.05, half_size: 0.9, segments: 32 },
            color: [0.95, 0.72, 0.24, 1.0],
            radius: 1.65,
        },
        ShowcaseShape {
            name: "duocylinder",
            template: ShapeTemplate::Duocylinder { radius_xy: 1.0, radius_zw: 1.0, segments: 32 },
            color: [0.40, 0.65, 1.0, 1.0],
            radius: 1.45,
        },
    ]
}

fn camera_for(angle: &str, slice_offset: f32) -> Camera4D {
    let mut camera = Camera4D::new();
    camera.slice_offset = slice_offset;
    match angle {
        "xw" => camera.rotate_xw(0.45),
        "zw" => camera.rotate_w(0.45),
        _ => {}
    }
    // Keep the object at the origin centered in the slice plane for every
    // orientation. Leaving the camera at world Z=5 after a ZW rotation makes
    // the plane miss small objects entirely; positioning along `-forward`
    // preserves the usual identity-camera view distance while maintaining
    // dot(ana, origin - camera.position) == 0.
    camera.position = -camera.forward() * 5.0;
    camera
}

fn main() {
    env_logger::init();

    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".scratchpad/captures-gallery"));
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let mut gpu = HeadlessGpu::new();
    let mut total_captures = 0usize;
    let mut zero_captures = 0usize;

    for shape in shapes() {
        println!("\n[SHAPE] {}", shape.name);
        let geom = shape.geometry();
        let tetrahedron_count = geom.tetrahedron_count() as u32;
        gpu.upload_geometry(&geom);

        for angle in ["identity", "xw", "zw"] {
            for (label, slice_offset) in [
                ("minus", -0.20 * shape.radius),
                ("mid", 0.0),
                ("plus", 0.20 * shape.radius),
            ] {
                let camera = camera_for(angle, slice_offset);
                let path = out_dir.join(format!("{}_{}_{}.ppm", shape.name, label, angle));
                let tris = gpu.capture(&camera, tetrahedron_count, &path);
                total_captures += 1;
                if tris == 0 {
                    zero_captures += 1;
                    eprintln!(
                        "[WARN] zero triangles for {} {label} {angle}; inspect if intentional",
                        shape.name
                    );
                }
            }
        }
    }

    println!(
        "\n[SUMMARY] {total_captures} captures written to {}; zero-triangle captures: {zero_captures}",
        out_dir.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_showcase_shapes_create_geometry() {
        for shape in shapes() {
            let geom = shape.geometry();
            assert!(geom.vertex_count() > 0, "{} has no vertices", shape.name);
            assert!(geom.tetrahedron_count() > 0, "{} has no tetrahedra", shape.name);
        }
    }

    #[test]
    fn test_showcase_gpu_types_still_match() {
        // Keeps this example honest if the pipeline types change.
        assert_eq!(std::mem::size_of::<rust4d_render::pipeline::Vertex4D>(), 32);
        assert_eq!(std::mem::size_of::<rust4d_render::pipeline::GpuTetrahedron>(), 16);
    }
}
