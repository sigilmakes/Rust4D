//! Renderable geometry - bridges World/Entity to GPU buffers
//!
//! This module converts the abstract shape data from rust4d_core into
//! GPU-compatible vertex and tetrahedra buffers.

use rust4d_core::{Entity, World, Material, Transform4D, ShapeRef};
use rust4d_math::Vec4;
use crate::pipeline::{Vertex4D, GpuTetrahedron};

/// GPU-ready geometry collected from entities
///
/// This struct holds the vertices and tetrahedra in a format ready for
/// upload to GPU buffers.
pub struct RenderableGeometry {
    /// Vertices with 4D positions and colors
    pub vertices: Vec<Vertex4D>,
    /// Tetrahedra as indices into the vertex buffer
    pub tetrahedra: Vec<GpuTetrahedron>,
}

impl RenderableGeometry {
    /// Create an empty renderable geometry
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            tetrahedra: Vec::new(),
        }
    }

    /// Create renderable geometry with pre-allocated capacity
    pub fn with_capacity(vertex_capacity: usize, tetrahedron_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            tetrahedra: Vec::with_capacity(tetrahedron_capacity),
        }
    }

    /// Collect geometry from a single entity
    ///
    /// Uses the entity's material base_color for all vertices.
    pub fn from_entity(entity: &Entity) -> Self {
        Self::from_entity_with_color(entity, &default_color_fn)
    }

    /// Collect geometry from a single entity with a custom color function
    pub fn from_entity_with_color(entity: &Entity, color_fn: &dyn Fn(&Vec4, &Material) -> [f32; 4]) -> Self {
        let mut result = Self::new();
        result.add_entity_with_color(entity, color_fn);
        result
    }

    /// Collect geometry from all entities in a world
    ///
    /// Uses each entity's material base_color for all its vertices.
    pub fn from_world(world: &World) -> Self {
        Self::from_world_with_color(world, &default_color_fn)
    }

    /// Collect geometry from all entities in a world with a custom color function
    ///
    /// Iterates all entities with Transform4D, ShapeRef, and Material components
    /// using ECS queries.
    pub fn from_world_with_color(world: &World, color_fn: &dyn Fn(&Vec4, &Material) -> [f32; 4]) -> Self {
        let mut result = Self::new();
        for (_entity, (transform, shape, material)) in world.ecs().query::<(&Transform4D, &ShapeRef, &Material)>().iter() {
            result.add_components_with_color(transform, shape.as_shape(), material, color_fn);
        }
        result
    }

    /// Add an entity's geometry to this collection
    ///
    /// Uses the entity's material base_color for all vertices.
    pub fn add_entity(&mut self, entity: &Entity) {
        self.add_entity_with_color(entity, &default_color_fn);
    }

    /// Add an entity's geometry with a custom color function
    pub fn add_entity_with_color(&mut self, entity: &Entity, color_fn: &dyn Fn(&Vec4, &Material) -> [f32; 4]) {
        self.add_components_with_color(&entity.transform, entity.shape(), &entity.material, color_fn);
    }

    /// Add geometry from individual ECS components with a custom color function
    ///
    /// This is the core method that works with decomposed components rather than
    /// a monolithic Entity struct.
    pub fn add_components_with_color(
        &mut self,
        transform: &Transform4D,
        shape: &dyn rust4d_math::ConvexShape4D,
        material: &Material,
        color_fn: &dyn Fn(&Vec4, &Material) -> [f32; 4],
    ) {
        let vertex_offset = self.vertices.len();

        // Transform and add vertices
        for v in shape.vertices() {
            let world_pos = transform.transform_point(*v);
            let color = color_fn(v, material);
            self.vertices.push(Vertex4D::new(
                [world_pos.x, world_pos.y, world_pos.z, world_pos.w],
                color,
            ));
        }

        // Add tetrahedra with offset indices
        for tet in shape.tetrahedra() {
            self.tetrahedra.push(GpuTetrahedron::from_indices([
                (tet.indices[0] + vertex_offset) as u32,
                (tet.indices[1] + vertex_offset) as u32,
                (tet.indices[2] + vertex_offset) as u32,
                (tet.indices[3] + vertex_offset) as u32,
            ]));
        }
    }

    /// Clear all geometry
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.tetrahedra.clear();
    }

    /// Get the number of vertices
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of tetrahedra
    #[inline]
    pub fn tetrahedron_count(&self) -> usize {
        self.tetrahedra.len()
    }
}

impl Default for RenderableGeometry {
    fn default() -> Self {
        Self::new()
    }
}

/// Default color function - uses material's base_color for all vertices
fn default_color_fn(_vertex: &Vec4, material: &Material) -> [f32; 4] {
    material.base_color
}

/// Color function that creates a gradient based on vertex position
///
/// Maps each coordinate component to RGB channels.
pub fn position_gradient_color(vertex: &Vec4, _material: &Material) -> [f32; 4] {
    [
        (vertex.x + 1.0) / 2.0, // Red from x
        (vertex.y + 1.0) / 2.0, // Green from y
        (vertex.z + 1.0) / 2.0, // Blue from z
        1.0,
    ]
}

/// Utility struct for building geometry with checkerboard patterns
pub struct CheckerboardGeometry {
    /// Colors for the checkerboard pattern
    pub color_a: [f32; 4],
    pub color_b: [f32; 4],
    /// Size of each checker cell
    pub cell_size: f32,
}

impl CheckerboardGeometry {
    /// Create a new checkerboard with the given colors and cell size
    pub fn new(color_a: [f32; 4], color_b: [f32; 4], cell_size: f32) -> Self {
        Self { color_a, color_b, cell_size }
    }

    /// Get the color for a vertex based on its XZ position
    pub fn color_for_position(&self, x: f32, z: f32) -> [f32; 4] {
        let cell_x = (x / self.cell_size).floor() as i32;
        let cell_z = (z / self.cell_size).floor() as i32;

        if (cell_x + cell_z) % 2 == 0 {
            self.color_a
        } else {
            self.color_b
        }
    }

    /// Create a color function that applies checkerboard pattern
    pub fn color_fn(&self) -> impl Fn(&Vec4, &Material) -> [f32; 4] + '_ {
        move |vertex, _material| {
            self.color_for_position(vertex.x, vertex.z)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_core::{ShapeRef, Tesseract4D, Transform4D};

    fn make_test_entity() -> Entity {
        let tesseract = Tesseract4D::new(2.0);
        Entity::with_material(
            ShapeRef::shared(tesseract),
            Material::from_rgb(1.0, 0.5, 0.25),
        )
    }

    #[test]
    fn test_renderable_geometry_new() {
        let geom = RenderableGeometry::new();
        assert_eq!(geom.vertex_count(), 0);
        assert_eq!(geom.tetrahedron_count(), 0);
    }

    #[test]
    fn test_renderable_geometry_from_entity() {
        let entity = make_test_entity();
        let geom = RenderableGeometry::from_entity(&entity);

        assert_eq!(geom.vertex_count(), 16); // Tesseract has 16 vertices
        assert!(geom.tetrahedron_count() > 0);

        // Check that all vertices have the material color
        for v in &geom.vertices {
            assert_eq!(v.color, [1.0, 0.5, 0.25, 1.0]);
        }
    }

    #[test]
    fn test_renderable_geometry_from_world() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        let geom = RenderableGeometry::from_world(&world);

        assert_eq!(geom.vertex_count(), 32); // 2 tesseracts * 16 vertices
    }

    #[test]
    fn test_renderable_geometry_add_entity() {
        let mut geom = RenderableGeometry::new();
        let entity = make_test_entity();

        geom.add_entity(&entity);
        assert_eq!(geom.vertex_count(), 16);

        geom.add_entity(&entity);
        assert_eq!(geom.vertex_count(), 32);
    }

    #[test]
    fn test_renderable_geometry_clear() {
        let entity = make_test_entity();
        let mut geom = RenderableGeometry::from_entity(&entity);

        assert!(geom.vertex_count() > 0);
        geom.clear();
        assert_eq!(geom.vertex_count(), 0);
        assert_eq!(geom.tetrahedron_count(), 0);
    }

    #[test]
    fn test_position_gradient_color() {
        let v = Vec4::new(1.0, 1.0, 1.0, 0.0);
        let m = Material::default();
        let color = position_gradient_color(&v, &m);

        assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);

        let v2 = Vec4::new(-1.0, -1.0, -1.0, 0.0);
        let color2 = position_gradient_color(&v2, &m);
        assert_eq!(color2, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_checkerboard_color() {
        let checker = CheckerboardGeometry::new(
            [1.0, 1.0, 1.0, 1.0], // white
            [0.0, 0.0, 0.0, 1.0], // black
            1.0,
        );

        // (0, 0) -> cell (0, 0) -> even -> white
        let c1 = checker.color_for_position(0.5, 0.5);
        assert_eq!(c1, [1.0, 1.0, 1.0, 1.0]);

        // (1, 0) -> cell (1, 0) -> odd -> black
        let c2 = checker.color_for_position(1.5, 0.5);
        assert_eq!(c2, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_transform_applied() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));

        // Translate by (10, 0, 0, 0)
        entity.transform = Transform4D::from_position(Vec4::new(10.0, 0.0, 0.0, 0.0));

        let geom = RenderableGeometry::from_entity(&entity);

        // All vertices should be offset by 10 in x
        for v in &geom.vertices {
            assert!(v.position[0] >= 9.0 && v.position[0] <= 11.0,
                "Vertex x should be around 10, got {}", v.position[0]);
        }
    }

    #[test]
    fn test_tetrahedra_indices_offset() {
        let mut geom = RenderableGeometry::new();
        let entity1 = make_test_entity();
        let entity2 = make_test_entity();

        geom.add_entity(&entity1);
        let first_entity_verts = geom.vertex_count();

        geom.add_entity(&entity2);

        // Second entity's tetrahedra should have indices >= first_entity_verts
        let second_tet = geom.tetrahedra.last().unwrap();
        assert!(second_tet.v0 >= first_entity_verts as u32,
            "Second entity's tetrahedra should have offset indices");
    }
}
