//! GPU-compatible data types for the 4D slicing pipeline
//!
//! These types are designed to match the shader layouts exactly.
//! All types derive Pod and Zeroable for safe GPU buffer operations.

use bytemuck::{Pod, Zeroable};

/// A vertex in 4D space with color
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex4D {
    /// Position in 4D space (x, y, z, w)
    pub position: [f32; 4],
    /// RGBA color
    pub color: [f32; 4],
}

impl Vertex4D {
    /// Create a new 4D vertex
    pub fn new(position: [f32; 4], color: [f32; 4]) -> Self {
        Self { position, color }
    }

    /// Create a vertex from position with default white color
    pub fn from_position(position: [f32; 4]) -> Self {
        Self {
            position,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// A tetrahedron (3-simplex) for GPU processing
///
/// Stores indices into a vertex buffer rather than the vertices themselves.
/// When sliced by a hyperplane, produces 0-2 triangles:
/// - 0 or 4 vertices above: no intersection
/// - 1 or 3 vertices above: 1 triangle
/// - 2 vertices above: 2 triangles (quad split)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuTetrahedron {
    /// Indices of the 4 vertices (into the vertex buffer)
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
    pub v3: u32,
}

impl GpuTetrahedron {
    /// Create a new GPU tetrahedron from vertex indices
    pub fn new(v0: u32, v1: u32, v2: u32, v3: u32) -> Self {
        Self { v0, v1, v2, v3 }
    }

    /// Create from an array of indices
    pub fn from_indices(indices: [u32; 4]) -> Self {
        Self {
            v0: indices[0],
            v1: indices[1],
            v2: indices[2],
            v3: indices[3],
        }
    }
}

/// A vertex in the 3D cross-section output
///
/// Produced by the slice compute shader and consumed by the render pipeline.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex3D {
    /// Position in 3D space (x, y, z)
    pub position: [f32; 3],
    /// Surface normal for lighting
    pub normal: [f32; 3],
    /// RGBA color (interpolated from 4D vertices)
    pub color: [f32; 4],
    /// Original W depth (for depth-based effects)
    pub w_depth: f32,
    /// Padding to align to 16 bytes
    pub _padding: f32,
}

impl Default for Vertex3D {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            color: [1.0; 4],
            w_depth: 0.0,
            _padding: 0.0,
        }
    }
}

/// Parameters for the slice compute shader
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SliceParams {
    /// W-coordinate of the slicing hyperplane
    pub slice_w: f32,
    /// Number of tetrahedra to process
    pub tetrahedron_count: u32,
    /// Padding for 16-byte alignment
    pub _padding: [f32; 2],
    /// 4D camera rotation matrix (camera-local to world, needs transpose for view)
    pub camera_matrix: [[f32; 4]; 4],
    /// 4D camera position (for translating geometry to camera space)
    pub camera_position: [f32; 4],
}

impl Default for SliceParams {
    fn default() -> Self {
        Self {
            slice_w: 0.0,
            tetrahedron_count: 0,
            _padding: [0.0; 2],
            camera_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_position: [0.0, 0.0, 5.0, 0.0],
        }
    }
}

/// A point light for the 3D cross-section render pass.
///
/// Layout: 32 bytes, matching WGSL `PointLight`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PointLightUniform {
    /// XYZ position in camera/world 3-space plus attenuation radius.
    pub position_radius: [f32; 4],
    /// RGB color plus scalar intensity.
    pub color_intensity: [f32; 4],
}

impl PointLightUniform {
    /// Construct a point light from position, radius, color, and intensity.
    pub fn new(position: [f32; 3], radius: f32, color: [f32; 3], intensity: f32) -> Self {
        Self {
            position_radius: [position[0], position[1], position[2], radius],
            color_intensity: [color[0], color[1], color[2], intensity],
        }
    }
}

impl Default for PointLightUniform {
    fn default() -> Self {
        Self::new([0.0, 2.0, 2.0], 8.0, [1.0, 1.0, 1.0], 0.0)
    }
}

/// Render uniforms for the 3D rendering pass.
/// Layout: 336 bytes total (must match render.wgsl RenderUniforms).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct RenderUniforms {
    /// View matrix (64 bytes)
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix (64 bytes)
    pub projection_matrix: [[f32; 4]; 4],
    /// Directional light direction (normalized) + padding (16 bytes)
    pub light_dir: [f32; 3],
    pub _padding: f32,
    /// Camera position for specular highlights (16 bytes)
    pub camera_pos: [f32; 3],
    pub _padding_camera: f32,
    /// Lighting parameters (16 bytes)
    pub ambient_strength: f32,
    pub diffuse_strength: f32,
    pub specular_strength: f32,
    pub specular_power: f32,
    /// W-color and fog parameters (16 bytes)
    pub w_color_strength: f32,
    pub w_range: f32,
    pub fog_density: f32,
    pub point_light_count: f32,
    /// Fog color (16 bytes)
    pub fog_color: [f32; 3],
    pub _padding_fog: f32,
    /// Up to four point lights (128 bytes)
    pub point_lights: [PointLightUniform; 4],
}

impl Default for RenderUniforms {
    fn default() -> Self {
        Self {
            view_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            projection_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            light_dir: [0.5, 1.0, 0.3],
            _padding: 0.0,
            camera_pos: [0.0, 0.0, 0.0],
            _padding_camera: 0.0,
            ambient_strength: 0.28,
            diffuse_strength: 0.72,
            specular_strength: 0.35,
            specular_power: 48.0,
            w_color_strength: 0.5,
            w_range: 2.0,
            fog_density: 0.018,
            point_light_count: 1.0,
            fog_color: [0.035, 0.035, 0.055],
            _padding_fog: 0.0,
            point_lights: [
                PointLightUniform::new([2.5, 3.0, 4.0], 10.0, [1.0, 0.86, 0.65], 0.65),
                PointLightUniform::default(),
                PointLightUniform::default(),
                PointLightUniform::default(),
            ],
        }
    }
}

/// Atomic counter for triangle output
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct AtomicCounter {
    pub count: u32,
}

/// Default maximum triangles for tests and fallback.
/// Production code should use the value from config (rendering.max_triangles).
pub const MAX_OUTPUT_TRIANGLES: usize = 100_000;

/// Size of a single triangle in Vertex3D units (3 vertices)
pub const TRIANGLE_VERTEX_COUNT: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_vertex4d_size() {
        // 4 floats position + 4 floats color = 32 bytes
        assert_eq!(size_of::<Vertex4D>(), 32);
    }

    #[test]
    fn test_gpu_tetrahedron_size() {
        // 4 u32 indices = 16 bytes
        assert_eq!(size_of::<GpuTetrahedron>(), 16);
    }

    #[test]
    fn test_vertex3d_size() {
        // 3 floats position + 3 floats normal + 4 floats color + 1 float w_depth + 1 float padding
        // = 12 floats = 48 bytes
        assert_eq!(size_of::<Vertex3D>(), 48);
    }

    #[test]
    fn test_slice_params_size() {
        // 1 float + 1 u32 + 2 floats padding + 16 floats matrix + 4 floats position = 96 bytes
        assert_eq!(size_of::<SliceParams>(), 96);
    }

    #[test]
    fn test_point_light_uniform_size() {
        // position+radius vec4 + color+intensity vec4 = 8 floats = 32 bytes
        assert_eq!(size_of::<PointLightUniform>(), 32);
    }

    #[test]
    fn test_render_uniforms_size() {
        // 16 floats view + 16 projection + 4 directional-light slot +
        // 4 camera slot + 4 lighting params + 4 w/fog params + 4 fog slot +
        // 4 point lights × 8 floats = 84 floats = 336 bytes
        assert_eq!(size_of::<RenderUniforms>(), 336);
    }

    #[test]
    fn test_alignment() {
        // All types should be 4-byte aligned (f32 alignment)
        assert_eq!(std::mem::align_of::<Vertex4D>(), 4);
        assert_eq!(std::mem::align_of::<Vertex3D>(), 4);
        assert_eq!(std::mem::align_of::<SliceParams>(), 4);
        assert_eq!(std::mem::align_of::<PointLightUniform>(), 4);
        assert_eq!(std::mem::align_of::<RenderUniforms>(), 4);
    }
}
