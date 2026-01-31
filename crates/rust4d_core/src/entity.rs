//! Entity-related types: Material, ShapeRef, DirtyFlags, EntityTemplate
//!
//! These types represent visual and tracking properties of entities in the 4D world.
//! The Entity struct has been removed -- entities are now spawned directly as
//! ECS component tuples via `world.spawn(...)` or `EntityTemplate::spawn_in(...)`.

use std::collections::HashSet;
use std::sync::Arc;
use bitflags::bitflags;
use rust4d_math::ConvexShape4D;
use serde::{Serialize, Deserialize};
use crate::Transform4D;
use crate::shapes::ShapeTemplate;

bitflags! {
    /// Flags indicating which parts of an entity have changed and need updating
    ///
    /// Used for dirty tracking to avoid rebuilding all geometry when only
    /// some entities have changed.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct DirtyFlags: u8 {
        /// No changes
        const NONE = 0;
        /// Transform (position, rotation, scale) has changed
        const TRANSFORM = 1 << 0;
        /// Mesh/shape has changed
        const MESH = 1 << 1;
        /// Material has changed
        const MATERIAL = 1 << 2;
        /// All flags set - entity needs full rebuild
        const ALL = Self::TRANSFORM.bits() | Self::MESH.bits() | Self::MATERIAL.bits();
    }
}

/// A simple material with just a base color
///
/// This is minimal for now - can be extended with PBR properties later.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Material {
    /// Base color as RGBA (each component 0.0-1.0)
    pub base_color: [f32; 4],
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0], // White
        }
    }
}

impl Material {
    /// Create a new material with the given RGBA color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            base_color: [r, g, b, a],
        }
    }

    /// Create a new opaque material with the given RGB color
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// White material
    pub const WHITE: Self = Self { base_color: [1.0, 1.0, 1.0, 1.0] };

    /// Gray material
    pub const GRAY: Self = Self { base_color: [0.5, 0.5, 0.5, 1.0] };

    /// Red material
    pub const RED: Self = Self { base_color: [1.0, 0.0, 0.0, 1.0] };

    /// Green material
    pub const GREEN: Self = Self { base_color: [0.0, 1.0, 0.0, 1.0] };

    /// Blue material
    pub const BLUE: Self = Self { base_color: [0.0, 0.0, 1.0, 1.0] };
}

/// Reference to a shape - either shared (Arc) or owned (Box)
///
/// Use `Shared` for memory-efficient storage when multiple entities use the same shape.
/// Use `Owned` when an entity needs its own unique copy for modification.
pub enum ShapeRef {
    /// A shared reference to a shape (multiple entities can share this)
    Shared(Arc<dyn ConvexShape4D>),
    /// An owned shape (unique to this entity)
    Owned(Box<dyn ConvexShape4D>),
}

impl ShapeRef {
    /// Create a shared shape reference
    pub fn shared<S: ConvexShape4D + 'static>(shape: S) -> Self {
        Self::Shared(Arc::new(shape))
    }

    /// Create an owned shape reference
    pub fn owned<S: ConvexShape4D + 'static>(shape: S) -> Self {
        Self::Owned(Box::new(shape))
    }

    /// Get a reference to the underlying shape
    pub fn as_shape(&self) -> &dyn ConvexShape4D {
        match self {
            ShapeRef::Shared(arc) => arc.as_ref(),
            ShapeRef::Owned(boxed) => boxed.as_ref(),
        }
    }
}

/// A serializable entity template
///
/// EntityTemplate is used for scene serialization. It stores a ShapeTemplate (enum)
/// rather than a trait object, making it serializable. Use `spawn_in()` to
/// instantiate as an entity in a World.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTemplate {
    /// Optional name for this entity (for lookup)
    pub name: Option<String>,
    /// Tags for categorization (e.g., "dynamic", "static")
    #[serde(default)]
    pub tags: Vec<String>,
    /// The entity's transform in world space
    pub transform: Transform4D,
    /// The entity's shape template (serializable)
    pub shape: ShapeTemplate,
    /// The entity's material
    pub material: Material,
}

impl EntityTemplate {
    /// Create a new entity template
    pub fn new(shape: ShapeTemplate, transform: Transform4D, material: Material) -> Self {
        Self {
            name: None,
            tags: Vec::new(),
            transform,
            shape,
            material,
        }
    }

    /// Set the name of this template
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag to this template
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Spawn this template as an entity in the given World
    ///
    /// Creates the shape from the template and spawns an entity with all
    /// configured components (shape, transform, material, dirty flags,
    /// optional name and tags).
    pub fn spawn_in(&self, world: &mut crate::World) -> hecs::Entity {
        let shape = self.shape.create_shape();
        let mut builder = hecs::EntityBuilder::new();
        builder.add(ShapeRef::Owned(shape));
        builder.add(self.transform);
        builder.add(self.material);
        builder.add(DirtyFlags::ALL);
        if let Some(ref name) = self.name {
            builder.add(crate::components::Name(name.clone()));
        }
        if !self.tags.is_empty() {
            let tags: HashSet<String> = self.tags.iter().cloned().collect();
            builder.add(crate::components::Tags(tags));
        }
        world.spawn(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_math::Tesseract4D;

    #[test]
    fn test_material_default() {
        let m = Material::default();
        assert_eq!(m.base_color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_material_new() {
        let m = Material::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(m.base_color, [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn test_material_from_rgb() {
        let m = Material::from_rgb(0.5, 0.6, 0.7);
        assert_eq!(m.base_color, [0.5, 0.6, 0.7, 1.0]);
    }

    #[test]
    fn test_shape_ref_shared() {
        let tesseract = Tesseract4D::new(2.0);
        let shape_ref = ShapeRef::shared(tesseract);

        match &shape_ref {
            ShapeRef::Shared(_) => {}
            _ => panic!("Expected Shared variant"),
        }

        assert_eq!(shape_ref.as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_shape_ref_owned() {
        let tesseract = Tesseract4D::new(2.0);
        let shape_ref = ShapeRef::owned(tesseract);

        match &shape_ref {
            ShapeRef::Owned(_) => {}
            _ => panic!("Expected Owned variant"),
        }

        assert_eq!(shape_ref.as_shape().vertex_count(), 16);
    }

    // --- Dirty tracking tests ---

    #[test]
    fn test_dirty_flags_default() {
        let flags = DirtyFlags::default();
        assert_eq!(flags, DirtyFlags::NONE);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_dirty_flags_all() {
        let flags = DirtyFlags::ALL;
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(flags.contains(DirtyFlags::MESH));
        assert!(flags.contains(DirtyFlags::MATERIAL));
    }

    #[test]
    fn test_dirty_flags_combine() {
        let flags = DirtyFlags::TRANSFORM | DirtyFlags::MATERIAL;
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(!flags.contains(DirtyFlags::MESH));
        assert!(flags.contains(DirtyFlags::MATERIAL));
    }

    #[test]
    fn test_entity_template_spawn_in() {
        use crate::shapes::ShapeTemplate;
        use rust4d_math::Vec4;

        let template = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 2.0, 3.0, 4.0)),
            Material::RED,
        ).with_name("my_cube").with_tag("dynamic");

        let mut world = crate::World::new();
        let entity = template.spawn_in(&mut world);

        // Verify components
        let name = world.ecs().get::<&crate::components::Name>(entity).unwrap();
        assert_eq!(name.0, "my_cube");

        let tags = world.ecs().get::<&crate::components::Tags>(entity).unwrap();
        assert!(tags.has("dynamic"));

        let transform = world.ecs().get::<&Transform4D>(entity).unwrap();
        assert_eq!(transform.position.x, 1.0);

        let material = world.ecs().get::<&Material>(entity).unwrap();
        assert_eq!(material.base_color, [1.0, 0.0, 0.0, 1.0]);

        let shape = world.ecs().get::<&ShapeRef>(entity).unwrap();
        assert_eq!(shape.as_shape().vertex_count(), 16);

        // Verify name index works
        assert!(world.get_by_name("my_cube").is_some());
    }
}
