//! Compute pipeline for 4D cross-section slicing
//!
//! This pipeline takes 4D geometry (tetrahedra) and produces 3D triangles
//! by intersecting with a hyperplane at a given W coordinate.

use wgpu::util::DeviceExt;

use super::types::{
    AtomicCounter, GpuTetrahedron, SliceParams, Vertex3D, Vertex4D, TRIANGLE_VERTEX_COUNT,
};

/// Compute pipeline for slicing 4D geometry
pub struct SlicePipeline {
    /// The compute pipeline for tetrahedra slicing
    pipeline: wgpu::ComputePipeline,
    /// Bind group layout for tetrahedra pipeline
    bind_group_layout: wgpu::BindGroupLayout,
    /// Vertex buffer (4D vertices)
    vertex_buffer: Option<wgpu::Buffer>,
    /// Tetrahedra buffer (indices into vertex buffer)
    tetra_buffer: Option<wgpu::Buffer>,
    tetra_count: u32,
    /// Bind group for pipeline
    bind_group: Option<wgpu::BindGroup>,

    /// Output buffer for triangles
    output_buffer: wgpu::Buffer,
    /// Atomic counter buffer for triangle count
    counter_buffer: wgpu::Buffer,
    /// Slice parameters uniform buffer
    params_buffer: wgpu::Buffer,
}

impl SlicePipeline {
    /// Create a new slice pipeline with the specified maximum triangle capacity
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `max_triangles` - Maximum number of triangles to allocate buffer space for.
    ///   Each triangle requires 3 vertices x 48 bytes = 144 bytes.
    ///   Will be clamped to the GPU's max_storage_buffer_binding_size limit.
    pub fn new(device: &wgpu::Device, max_triangles: usize) -> Self {
        // Calculate bytes per triangle and clamp to GPU limits
        let bytes_per_triangle = TRIANGLE_VERTEX_COUNT * std::mem::size_of::<Vertex3D>();
        let max_buffer_size = device.limits().max_storage_buffer_binding_size as usize;
        let max_triangles_for_gpu = max_buffer_size / bytes_per_triangle;

        let max_triangles = if max_triangles > max_triangles_for_gpu {
            log::warn!(
                "Requested {} triangles exceeds GPU limit of {} (max_storage_buffer_binding_size={}). Clamping.",
                max_triangles, max_triangles_for_gpu, max_buffer_size
            );
            max_triangles_for_gpu
        } else {
            max_triangles
        };

        // Bind group layout for tetrahedra slicing
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Slice Bind Group Layout"),
            entries: &[
                // Vertices buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Tetrahedra buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output triangles buffer (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Atomic counter buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Slice parameters uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Slice Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Load shader
        let shader_source = include_str!("../shaders/slice_tetra.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Slice Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Slice Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create output buffer sized by max_triangles parameter
        let output_size =
            (max_triangles * TRIANGLE_VERTEX_COUNT * std::mem::size_of::<Vertex3D>()) as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Output Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create counter buffer
        let counter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Counter Buffer"),
            size: std::mem::size_of::<AtomicCounter>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        // Create params buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Params Buffer"),
            size: std::mem::size_of::<SliceParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            vertex_buffer: None,
            tetra_buffer: None,
            tetra_count: 0,
            bind_group: None,
            output_buffer,
            counter_buffer,
            params_buffer,
        }
    }

    /// Upload tetrahedra and vertices to the GPU
    pub fn upload_tetrahedra(
        &mut self,
        device: &wgpu::Device,
        vertices: &[Vertex4D],
        tetrahedra: &[GpuTetrahedron],
    ) {
        self.tetra_count = tetrahedra.len() as u32;

        // Create vertex buffer
        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        // Create tetrahedra buffer
        self.tetra_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Tetrahedra Buffer"),
                contents: bytemuck::cast_slice(tetrahedra),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        // Recreate bind group
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.vertex_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.tetra_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    /// Update slice parameters
    pub fn update_params(&self, queue: &wgpu::Queue, params: &SliceParams) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));
    }

    /// Reset the triangle counter to zero
    pub fn reset_counter(&self, queue: &wgpu::Queue) {
        let zero = AtomicCounter { count: 0 };
        queue.write_buffer(&self.counter_buffer, 0, bytemuck::bytes_of(&zero));
    }

    /// Run the slice compute pass
    ///
    /// This dispatches the compute shader to process all geometry.
    /// Call reset_counter() before this and update_params() with current parameters.
    pub fn run_slice_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.bind_group.is_none() || self.tetra_count == 0 {
            return;
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Slice Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

        let workgroup_count = self.tetra_count.div_ceil(64);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Get the output buffer for use as vertex buffer in rendering
    pub fn output_buffer(&self) -> &wgpu::Buffer {
        &self.output_buffer
    }

    /// Get the counter buffer for indirect drawing
    pub fn counter_buffer(&self) -> &wgpu::Buffer {
        &self.counter_buffer
    }

    /// Get the number of tetrahedra currently loaded
    pub fn tetrahedron_count(&self) -> u32 {
        self.tetra_count
    }

    /// Get the primitive count (tetrahedra count)
    pub fn primitive_count(&self) -> u32 {
        self.tetra_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GPU tests require a wgpu device which isn't available in unit tests
    // Integration tests should be used for full pipeline testing

    #[test]
    fn test_output_buffer_size_calculation() {
        // Test the buffer size calculation for various triangle counts
        let vertex_size = std::mem::size_of::<Vertex3D>();
        assert_eq!(vertex_size, 48); // 48 bytes per vertex

        // 100,000 triangles * 3 vertices * 48 bytes = 14,400,000 bytes
        let size_100k = 100_000 * TRIANGLE_VERTEX_COUNT * vertex_size;
        assert_eq!(size_100k, 14_400_000);

        // 1,000,000 triangles (config default) * 3 vertices * 48 bytes = 144,000,000 bytes
        let size_1m = 1_000_000 * TRIANGLE_VERTEX_COUNT * vertex_size;
        assert_eq!(size_1m, 144_000_000);
    }
}
