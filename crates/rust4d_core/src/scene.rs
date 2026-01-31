//! Scene serialization
//!
//! Provides Scene struct for loading/saving scenes from RON files.
//! Scenes contain entity templates, physics settings, and player spawn info.

use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use std::io;

use crate::entity::EntityTemplate;
use crate::shapes::ShapeTemplate;
use crate::World;
use rust4d_math::Vec4;
use rust4d_physics::{PhysicsConfig, RigidBody4D, StaticCollider, BodyType, PhysicsMaterial};

/// A serializable scene containing entity templates
///
/// Scenes are loaded from RON files and contain all the data needed
/// to populate a game world: entities, physics settings, and spawn points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Scene name (for display/debugging)
    pub name: String,
    /// Entity templates in this scene
    pub entities: Vec<EntityTemplate>,
    /// Gravity for physics (negative = downward)
    #[serde(default)]
    pub gravity: Option<f32>,
    /// Player spawn position [x, y, z, w]
    #[serde(default)]
    pub player_spawn: Option<[f32; 4]>,
}

impl Scene {
    /// Create a new empty scene
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: Vec::new(),
            gravity: None,
            player_spawn: None,
        }
    }

    /// Load a scene from a RON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SceneLoadError> {
        let contents = fs::read_to_string(path.as_ref())?;
        let scene: Scene = ron::from_str(&contents)?;
        log::debug!("Loaded scene '{}' with gravity={:?}, player_spawn={:?}",
            scene.name, scene.gravity, scene.player_spawn);
        Ok(scene)
    }

    /// Save a scene to a RON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SceneSaveError> {
        let pretty = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false);
        let contents = ron::ser::to_string_pretty(self, pretty)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Add an entity template to this scene
    pub fn add_entity(&mut self, entity: EntityTemplate) {
        self.entities.push(entity);
    }

    /// Set the gravity for this scene
    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = Some(gravity);
        self
    }

    /// Set the player spawn position
    pub fn with_player_spawn(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.player_spawn = Some([x, y, z, w]);
        self
    }
}

/// Error loading a scene
#[derive(Debug)]
pub enum SceneLoadError {
    /// IO error (file not found, permission denied, etc.)
    Io(io::Error),
    /// Parse error (invalid RON syntax)
    Parse(ron::error::SpannedError),
}

impl From<io::Error> for SceneLoadError {
    fn from(e: io::Error) -> Self {
        SceneLoadError::Io(e)
    }
}

impl From<ron::error::SpannedError> for SceneLoadError {
    fn from(e: ron::error::SpannedError) -> Self {
        SceneLoadError::Parse(e)
    }
}

impl std::fmt::Display for SceneLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneLoadError::Io(e) => write!(f, "IO error: {}", e),
            SceneLoadError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for SceneLoadError {}

/// Error saving a scene
#[derive(Debug)]
pub enum SceneSaveError {
    /// IO error (permission denied, disk full, etc.)
    Io(io::Error),
    /// Serialization error
    Serialize(ron::Error),
}

impl From<io::Error> for SceneSaveError {
    fn from(e: io::Error) -> Self {
        SceneSaveError::Io(e)
    }
}

impl From<ron::Error> for SceneSaveError {
    fn from(e: ron::Error) -> Self {
        SceneSaveError::Serialize(e)
    }
}

impl std::fmt::Display for SceneSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneSaveError::Io(e) => write!(f, "IO error: {}", e),
            SceneSaveError::Serialize(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for SceneSaveError {}

/// Unified error type for scene operations
///
/// This is used by SceneManager for all scene-related errors, providing a single
/// error type that covers loading, saving, and runtime scene management.
#[derive(Debug)]
pub enum SceneError {
    /// IO error (file not found, permission denied, etc.)
    Io(io::Error),
    /// Parse error (invalid RON syntax)
    Parse(ron::error::SpannedError),
    /// Serialization error
    Serialize(ron::Error),
    /// Scene not loaded (requested template doesn't exist)
    NotLoaded(String),
    /// No active scene on the stack
    NoActiveScene,
}

impl From<io::Error> for SceneError {
    fn from(e: io::Error) -> Self {
        SceneError::Io(e)
    }
}

impl From<ron::error::SpannedError> for SceneError {
    fn from(e: ron::error::SpannedError) -> Self {
        SceneError::Parse(e)
    }
}

impl From<ron::Error> for SceneError {
    fn from(e: ron::Error) -> Self {
        SceneError::Serialize(e)
    }
}

impl From<SceneLoadError> for SceneError {
    fn from(e: SceneLoadError) -> Self {
        match e {
            SceneLoadError::Io(io_err) => SceneError::Io(io_err),
            SceneLoadError::Parse(parse_err) => SceneError::Parse(parse_err),
        }
    }
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::Io(e) => write!(f, "IO error: {}", e),
            SceneError::Parse(e) => write!(f, "Parse error: {}", e),
            SceneError::Serialize(e) => write!(f, "Serialize error: {}", e),
            SceneError::NotLoaded(name) => write!(f, "Scene not loaded: {}", name),
            SceneError::NoActiveScene => write!(f, "No active scene"),
        }
    }
}

impl std::error::Error for SceneError {}

/// A runtime scene containing an instantiated World
///
/// ActiveScene wraps a World instance that has been instantiated from a Scene template
/// (or created programmatically). It tracks the scene name and player spawn position
/// from the original template.
pub struct ActiveScene {
    /// Scene name (from template or custom)
    pub name: String,
    /// Player spawn position (from template)
    pub player_spawn: Option<[f32; 4]>,
    /// The live world with entities and physics
    pub world: World,
}

impl ActiveScene {
    /// Create an active scene from a Scene template
    ///
    /// This instantiates all entities from the template into a new World,
    /// optionally enabling physics with the provided config.
    ///
    /// The `player_radius` parameter sets the collision radius for the player body.
    pub fn from_template(template: &Scene, physics_config: Option<PhysicsConfig>, player_radius: f32) -> Self {
        log::debug!("from_template: physics_config={:?}, template.gravity={:?}", physics_config, template.gravity);

        // Create world with physics
        let mut world = if let Some(config) = physics_config {
            log::debug!("Using provided physics_config with gravity={}", config.gravity);
            World::new().with_physics(config)
        } else if let Some(gravity) = template.gravity {
            log::debug!("Using template gravity={}", gravity);
            World::new().with_physics(PhysicsConfig::new(gravity))
        } else {
            log::debug!("No physics configured");
            World::new()
        };

        // Instantiate all entities from the template, setting up physics based on tags
        for entity_template in &template.entities {
            let mut entity = entity_template.to_entity();
            let is_static = entity_template.tags.contains(&"static".to_string());
            let is_dynamic = entity_template.tags.contains(&"dynamic".to_string());

            if let Some(physics) = world.physics_mut() {
                if is_static {
                    // Create bounded static collider for floor/walls (objects can fall off edges)
                    if let ShapeTemplate::Hyperplane { y, size, cell_size, thickness, .. } = &entity_template.shape {
                        log::debug!("Adding bounded floor collider: y={}, size={}, cell_size={}, thickness={}",
                            y, size, cell_size, thickness);
                        physics.add_static_collider(StaticCollider::floor_bounded(
                            *y,
                            *size,      // X/Z extent from hyperplane
                            *cell_size, // W extent
                            *thickness, // Y thickness
                            PhysicsMaterial::CONCRETE,
                        ));
                    }
                } else if is_dynamic {
                    // Create dynamic rigid body for movable objects
                    let position = Vec4::new(
                        entity_template.transform.position.x,
                        entity_template.transform.position.y,
                        entity_template.transform.position.z,
                        entity_template.transform.position.w,
                    );

                    // Get half-extent from shape
                    let half_extent = match &entity_template.shape {
                        ShapeTemplate::Tesseract { size } => size / 2.0,
                        ShapeTemplate::Hyperplane { .. } => 1.0, // shouldn't be dynamic, but fallback
                    };

                    let body = RigidBody4D::new_aabb(
                        position,
                        Vec4::new(half_extent, half_extent, half_extent, half_extent),
                    )
                    .with_body_type(BodyType::Dynamic)
                    .with_mass(10.0)
                    .with_material(PhysicsMaterial::WOOD);

                    let body_key = physics.add_body(body);
                    entity = entity.with_physics_body(body_key);
                }
            }

            world.add_entity(entity);
        }

        // Create player body from player_spawn
        if let (Some(spawn), Some(physics)) = (template.player_spawn, world.physics_mut()) {
            let position = Vec4::new(spawn[0], spawn[1], spawn[2], spawn[3]);
            let player_body = RigidBody4D::new_sphere(position, player_radius)
                .with_body_type(BodyType::Kinematic)
                .with_mass(1.0)
                .with_material(PhysicsMaterial::WOOD);

            let body_key = physics.add_body(player_body);
            physics.set_player_body(body_key);
        }

        Self {
            name: template.name.clone(),
            player_spawn: template.player_spawn,
            world,
        }
    }

    /// Create a new empty active scene with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            player_spawn: None,
            world: World::new(),
        }
    }

    /// Create with physics enabled
    pub fn with_physics(mut self, config: PhysicsConfig) -> Self {
        self.world = self.world.with_physics(config);
        self
    }

    /// Set the player spawn position
    pub fn with_player_spawn(mut self, spawn: [f32; 4]) -> Self {
        self.player_spawn = Some(spawn);
        self
    }

    /// Update the scene (steps physics)
    pub fn update(&mut self, dt: f32) {
        self.world.update(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transform4D, Material};
    use crate::shapes::ShapeTemplate;
    use rust4d_math::Vec4;

    #[test]
    fn test_scene_new() {
        let scene = Scene::new("Test Scene");
        assert_eq!(scene.name, "Test Scene");
        assert!(scene.entities.is_empty());
        assert!(scene.gravity.is_none());
        assert!(scene.player_spawn.is_none());
    }

    #[test]
    fn test_scene_with_gravity() {
        let scene = Scene::new("Test").with_gravity(-20.0);
        assert_eq!(scene.gravity, Some(-20.0));
    }

    #[test]
    fn test_scene_with_player_spawn() {
        let scene = Scene::new("Test").with_player_spawn(1.0, 2.0, 3.0, 4.0);
        assert_eq!(scene.player_spawn, Some([1.0, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_scene_add_entity() {
        let mut scene = Scene::new("Test");
        let entity = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::identity(),
            Material::WHITE,
        );
        scene.add_entity(entity);
        assert_eq!(scene.entities.len(), 1);
    }

    #[test]
    fn test_scene_serialization() {
        let mut scene = Scene::new("Test Scene")
            .with_gravity(-20.0)
            .with_player_spawn(0.0, 2.0, 5.0, 0.0);

        let entity = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 0.0, 0.0, 0.0)),
            Material::RED,
        ).with_name("test_cube").with_tag("dynamic");

        scene.add_entity(entity);

        // Serialize to RON
        let pretty = ron::ser::PrettyConfig::new().struct_names(true);
        let serialized = ron::ser::to_string_pretty(&scene, pretty).unwrap();

        // Verify it contains expected content
        assert!(serialized.contains("Test Scene"));
        assert!(serialized.contains("test_cube"));
        assert!(serialized.contains("Tesseract"));

        // Deserialize back
        let deserialized: Scene = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "Test Scene");
        assert_eq!(deserialized.gravity, Some(-20.0));
        assert_eq!(deserialized.entities.len(), 1);
        assert_eq!(deserialized.entities[0].name, Some("test_cube".to_string()));
    }

    #[test]
    fn test_parse_scene_file_format() {
        // Test parsing a scene matching the actual serialization format
        let scene_ron = r#"
Scene(
    name: "Test Scene",
    entities: [
        EntityTemplate(
            name: Some("floor"),
            tags: ["static"],
            transform: Transform4D(
                position: Vec4(x: 0.0, y: -2.0, z: 0.0, w: 0.0),
                rotation: (s: 1.0, b_xy: 0.0, b_xz: 0.0, b_xw: 0.0, b_yz: 0.0, b_yw: 0.0, b_zw: 0.0, p: 0.0),
                scale: 1.0,
            ),
            shape: ShapeTemplate(
                type: "Hyperplane",
                y: -2.0,
                size: 10.0,
                subdivisions: 10,
                cell_size: 5.0,
                thickness: 0.001,
            ),
            material: Material(base_color: (0.5, 0.5, 0.5, 1.0)),
        ),
        EntityTemplate(
            name: Some("tesseract"),
            tags: ["dynamic"],
            transform: Transform4D(
                position: Vec4(x: 0.0, y: 0.0, z: 0.0, w: 0.0),
                rotation: (s: 1.0, b_xy: 0.0, b_xz: 0.0, b_xw: 0.0, b_yz: 0.0, b_yw: 0.0, b_zw: 0.0, p: 0.0),
                scale: 1.0,
            ),
            shape: ShapeTemplate(
                type: "Tesseract",
                size: 2.0,
            ),
            material: Material(base_color: (1.0, 1.0, 1.0, 1.0)),
        ),
    ],
    gravity: Some(-20.0),
    player_spawn: Some((0.0, 2.0, 5.0, 0.0)),
)
"#;
        let scene: Scene = ron::from_str(scene_ron).unwrap();
        assert_eq!(scene.name, "Test Scene");
        assert_eq!(scene.gravity, Some(-20.0));
        assert_eq!(scene.player_spawn, Some([0.0, 2.0, 5.0, 0.0]));
        assert_eq!(scene.entities.len(), 2);

        // Check floor entity
        assert_eq!(scene.entities[0].name, Some("floor".to_string()));
        assert_eq!(scene.entities[0].tags, vec!["static"]);
        match &scene.entities[0].shape {
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                assert_eq!(*y, -2.0);
                assert_eq!(*size, 10.0);
                assert_eq!(*subdivisions, 10);
                assert_eq!(*cell_size, 5.0);
                assert_eq!(*thickness, 0.001);
            }
            _ => panic!("Expected Hyperplane shape"),
        }

        // Check tesseract entity
        assert_eq!(scene.entities[1].name, Some("tesseract".to_string()));
        assert_eq!(scene.entities[1].tags, vec!["dynamic"]);
        match &scene.entities[1].shape {
            ShapeTemplate::Tesseract { size } => {
                assert_eq!(*size, 2.0);
            }
            _ => panic!("Expected Tesseract shape"),
        }
    }

    #[test]
    fn test_entity_template_to_entity() {
        let template = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 2.0, 3.0, 4.0)),
            Material::RED,
        ).with_name("my_cube").with_tag("dynamic");

        let entity = template.to_entity();

        assert_eq!(entity.name, Some("my_cube".to_string()));
        assert!(entity.has_tag("dynamic"));
        assert_eq!(entity.transform.position.x, 1.0);
        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(entity.shape().vertex_count(), 16); // Tesseract has 16 vertices
    }

    // --- SceneError tests ---

    #[test]
    fn test_scene_error_display() {
        let err = SceneError::NotLoaded("test_scene".to_string());
        assert_eq!(format!("{}", err), "Scene not loaded: test_scene");

        let err = SceneError::NoActiveScene;
        assert_eq!(format!("{}", err), "No active scene");
    }

    #[test]
    fn test_scene_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let scene_err: SceneError = io_err.into();
        match scene_err {
            SceneError::Io(_) => {}
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_scene_error_from_scene_load_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let load_err = SceneLoadError::Io(io_err);
        let scene_err: SceneError = load_err.into();
        match scene_err {
            SceneError::Io(_) => {}
            _ => panic!("Expected Io variant"),
        }
    }

    // --- ActiveScene tests ---

    #[test]
    fn test_active_scene_new() {
        let scene = ActiveScene::new("Test Scene");
        assert_eq!(scene.name, "Test Scene");
        assert!(scene.player_spawn.is_none());
        assert!(scene.world.is_empty());
    }

    #[test]
    fn test_active_scene_with_physics() {
        let scene = ActiveScene::new("Physics Scene")
            .with_physics(PhysicsConfig::new(-20.0));
        assert!(scene.world.physics().is_some());
        assert_eq!(scene.world.physics().unwrap().config.gravity, -20.0);
    }

    #[test]
    fn test_active_scene_with_player_spawn() {
        let scene = ActiveScene::new("Spawn Scene")
            .with_player_spawn([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(scene.player_spawn, Some([1.0, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_active_scene_from_template() {
        // Create a scene template with entities
        let mut template = Scene::new("Template Scene")
            .with_gravity(-15.0)
            .with_player_spawn(0.0, 1.0, 5.0, 0.0);

        template.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 0.0, 0.0, 0.0)),
            Material::RED,
        ).with_name("cube"));

        // Instantiate from template
        let active = ActiveScene::from_template(&template, None, 0.5);

        assert_eq!(active.name, "Template Scene");
        assert_eq!(active.player_spawn, Some([0.0, 1.0, 5.0, 0.0]));
        assert_eq!(active.world.entity_count(), 1);

        // Check physics was set from template gravity
        assert!(active.world.physics().is_some());
        assert_eq!(active.world.physics().unwrap().config.gravity, -15.0);

        // Check entity was instantiated
        let entity_handle = active.world.get_by_name("cube").unwrap();
        let material = active.world.ecs().get::<&Material>(entity_handle).unwrap();
        assert_eq!(material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_active_scene_from_template_override_physics() {
        let template = Scene::new("Template").with_gravity(-10.0);

        // Override physics config
        let active = ActiveScene::from_template(
            &template,
            Some(PhysicsConfig::new(-30.0)),
            0.5,
        );

        // Should use overridden config, not template gravity
        assert_eq!(active.world.physics().unwrap().config.gravity, -30.0);
    }

    #[test]
    fn test_active_scene_update() {
        let mut scene = ActiveScene::new("Update Test")
            .with_physics(PhysicsConfig::new(-20.0));

        // Just verify update doesn't panic
        scene.update(0.016);
    }
}
