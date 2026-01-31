//! Scene management with scene stack support
//!
//! The SceneManager provides a centralized way to manage multiple scenes:
//! - Load scene templates from RON files
//! - Instantiate runtime scenes from templates
//! - Manage a scene stack for overlays (menus, pause screens, etc.)
//!
//! # Example
//! ```ignore
//! let mut manager = SceneManager::new()
//!     .with_physics(PhysicsConfig::new(-20.0));
//!
//! // Load a scene template
//! manager.load_scene("assets/scenes/level1.ron")?;
//!
//! // Instantiate and make it active
//! manager.instantiate("Level 1")?;
//! manager.push_scene("Level 1")?;
//!
//! // Game loop
//! manager.update(dt);
//! ```

use std::collections::HashMap;
use crate::{Scene, World};
use crate::scene::{SceneError, ActiveScene};
use crate::scene_transition::{SceneTransition, TransitionEffect};
use crate::scene_loader::SceneLoader;
use rust4d_physics::PhysicsConfig;

/// Manages multiple scenes with a stack for overlays
///
/// SceneManager provides:
/// - Template loading from RON files
/// - Runtime scene instantiation from templates
/// - A scene stack for overlay management (menus, pause screens)
/// - Centralized update loop for the active scene
pub struct SceneManager {
    /// Loaded scene templates (from files)
    templates: HashMap<String, Scene>,
    /// Instantiated runtime scenes
    scenes: HashMap<String, ActiveScene>,
    /// Stack of active scene names (top = current, for overlays/menus)
    active_stack: Vec<String>,
    /// Default physics config for new scenes
    default_physics: Option<PhysicsConfig>,
    /// Active transition between scenes
    transition: Option<SceneTransition>,
    /// Overlay scene names (rendered on top of active scene)
    overlay_stack: Vec<String>,
    /// Background scene loader
    loader: SceneLoader,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    /// Create a new empty scene manager
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            scenes: HashMap::new(),
            active_stack: Vec::new(),
            default_physics: None,
            transition: None,
            overlay_stack: Vec::new(),
            loader: SceneLoader::new(),
        }
    }

    /// Set the default physics config for new scenes
    pub fn with_physics(mut self, config: PhysicsConfig) -> Self {
        self.default_physics = Some(config);
        self
    }

    // --- Template management ---

    /// Load a scene template from a RON file
    ///
    /// Returns the scene name on success.
    pub fn load_scene(&mut self, path: &str) -> Result<String, SceneError> {
        let scene = Scene::load(path)?;
        let name = scene.name.clone();
        self.templates.insert(name.clone(), scene);
        Ok(name)
    }

    /// Get a loaded scene template by name
    pub fn get_template(&self, name: &str) -> Option<&Scene> {
        self.templates.get(name)
    }

    /// Register a template directly (without loading from file)
    pub fn register_template(&mut self, template: Scene) {
        self.templates.insert(template.name.clone(), template);
    }

    // --- Active scene management ---

    /// Register an active scene directly (bypassing templates)
    ///
    /// This is useful for scenes built programmatically via SceneBuilder.
    pub fn register_active_scene(&mut self, name: &str, scene: ActiveScene) {
        self.scenes.insert(name.to_string(), scene);
    }

    /// Instantiate a runtime scene from a loaded template
    ///
    /// The instantiated scene is stored but not automatically made active.
    /// Use `push_scene` to make it the current scene.
    pub fn instantiate(&mut self, template_name: &str) -> Result<(), SceneError> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| SceneError::NotLoaded(template_name.to_string()))?;

        let active = ActiveScene::from_template(template, self.default_physics.clone());
        self.scenes.insert(template_name.to_string(), active);
        Ok(())
    }

    // --- Scene stack ---

    /// Push a scene onto the stack, making it the active scene
    ///
    /// The scene must already be instantiated or registered.
    pub fn push_scene(&mut self, name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }
        self.active_stack.push(name.to_string());
        Ok(())
    }

    /// Pop the top scene from the stack
    ///
    /// Returns the name of the popped scene, or None if the stack is empty.
    /// Note: This does not remove the scene from storage, just from the active stack.
    pub fn pop_scene(&mut self) -> Option<String> {
        self.active_stack.pop()
    }

    /// Switch to a specific scene, replacing the current top of the stack
    ///
    /// If the stack is empty, this is equivalent to `push_scene`.
    pub fn switch_to(&mut self, name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }
        if !self.active_stack.is_empty() {
            self.active_stack.pop();
        }
        self.active_stack.push(name.to_string());
        Ok(())
    }

    // --- Active scene access ---

    /// Get a reference to the currently active scene (top of stack)
    pub fn active_scene(&self) -> Option<&ActiveScene> {
        self.active_stack.last()
            .and_then(|name| self.scenes.get(name))
    }

    /// Get a mutable reference to the currently active scene (top of stack)
    pub fn active_scene_mut(&mut self) -> Option<&mut ActiveScene> {
        if let Some(name) = self.active_stack.last().cloned() {
            self.scenes.get_mut(&name)
        } else {
            None
        }
    }

    /// Get a reference to the active scene's world
    pub fn active_world(&self) -> Option<&World> {
        self.active_scene().map(|scene| &scene.world)
    }

    /// Get a mutable reference to the active scene's world
    pub fn active_world_mut(&mut self) -> Option<&mut World> {
        self.active_scene_mut().map(|scene| &mut scene.world)
    }

    /// Get a scene by name (whether active or not)
    pub fn get_scene(&self, name: &str) -> Option<&ActiveScene> {
        self.scenes.get(name)
    }

    /// Get a mutable reference to a scene by name
    pub fn get_scene_mut(&mut self, name: &str) -> Option<&mut ActiveScene> {
        self.scenes.get_mut(name)
    }

    /// Get the name of the currently active scene
    pub fn active_scene_name(&self) -> Option<&str> {
        self.active_stack.last().map(|s| s.as_str())
    }

    /// Get the number of scenes in the stack
    pub fn stack_depth(&self) -> usize {
        self.active_stack.len()
    }

    /// Check if a scene is currently active (on the stack)
    pub fn is_scene_active(&self, name: &str) -> bool {
        self.active_stack.contains(&name.to_string())
    }

    // --- Update ---

    /// Update the active scene (steps physics, etc.)
    ///
    /// Only updates the top scene on the stack.
    pub fn update(&mut self, dt: f32) {
        if let Some(scene) = self.active_scene_mut() {
            scene.update(dt);
        }
    }

    // --- Transitions ---

    /// Switch to a scene with a transition effect
    ///
    /// Begins a transition from the current active scene to the named scene.
    /// The target scene must already be instantiated or registered.
    /// During the transition, both scenes may need to be rendered (e.g. crossfade).
    pub fn switch_to_with_transition(
        &mut self,
        name: &str,
        effect: TransitionEffect,
    ) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }

        let from = self.active_scene_name()
            .unwrap_or("")
            .to_string();

        self.transition = Some(SceneTransition::new(
            from,
            name.to_string(),
            effect,
        ));

        Ok(())
    }

    /// Update active transition, returns true if transition completed this frame
    ///
    /// When the transition completes, the scene is automatically switched
    /// to the target scene.
    pub fn update_transition(&mut self) -> bool {
        if let Some(ref mut transition) = self.transition {
            if transition.update() {
                // Transition complete - switch to the target scene
                let to_scene = transition.to_scene().to_string();
                self.transition = None;
                // Perform the actual switch (we already verified scene exists)
                let _ = self.switch_to(&to_scene);
                return true;
            }
        }
        false
    }

    /// Get current transition (for rendering)
    pub fn current_transition(&self) -> Option<&SceneTransition> {
        self.transition.as_ref()
    }

    /// Check if a transition is in progress
    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    // --- Overlays ---

    /// Push an overlay scene (renders on top of active scene)
    ///
    /// Overlay scenes are independent from the main scene stack and are
    /// rendered on top of the active scene. The scene must already be
    /// instantiated or registered.
    pub fn push_overlay(&mut self, name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }
        self.overlay_stack.push(name.to_string());
        Ok(())
    }

    /// Pop the top overlay
    ///
    /// Returns the name of the popped overlay, or None if the overlay stack is empty.
    pub fn pop_overlay(&mut self) -> Option<String> {
        self.overlay_stack.pop()
    }

    /// Get the overlay stack
    pub fn overlays(&self) -> &[String] {
        &self.overlay_stack
    }

    /// Check if a scene is an overlay
    pub fn is_overlay(&self, name: &str) -> bool {
        self.overlay_stack.iter().any(|n| n == name)
    }

    // --- Async Loading ---

    /// Start loading a scene in the background
    ///
    /// The scene file will be loaded asynchronously by a worker thread.
    /// Use [`poll_loading`](SceneManager::poll_loading) to check for completed loads.
    pub fn load_scene_async(&self, path: &str, scene_name: &str) {
        self.loader.load_async(path, scene_name);
    }

    /// Poll for completed async scene loads, returns names of newly loaded scenes
    ///
    /// Checks the background loader for completed scene loads. Successfully loaded
    /// scenes are automatically registered as templates. Returns the names of
    /// all scenes that were loaded this call.
    pub fn poll_loading(&mut self) -> Vec<String> {
        let results = self.loader.poll_all();
        let mut loaded_names = Vec::new();
        for result in results {
            match result.result {
                Ok(scene) => {
                    let name = result.scene_name.clone();
                    self.templates.insert(name.clone(), scene);
                    loaded_names.push(name);
                }
                Err(e) => {
                    log::warn!("Failed to load scene '{}': {}", result.scene_name, e);
                }
            }
        }
        loaded_names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ShapeRef, Material, DirtyFlags, Transform4D, Name};
    use rust4d_math::Tesseract4D;

    fn spawn_test_entity(world: &mut World) -> hecs::Entity {
        let tesseract = Tesseract4D::new(2.0);
        world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
        ))
    }

    #[test]
    fn test_new() {
        let manager = SceneManager::new();
        assert!(manager.active_scene().is_none());
        assert_eq!(manager.stack_depth(), 0);
    }

    #[test]
    fn test_with_physics() {
        let manager = SceneManager::new()
            .with_physics(PhysicsConfig::new(-20.0));
        assert!(manager.default_physics.is_some());
        assert_eq!(manager.default_physics.unwrap().gravity, -20.0);
    }

    #[test]
    fn test_register_active_scene() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Test Scene");
        manager.register_active_scene("test", scene);

        assert!(manager.get_scene("test").is_some());
        assert_eq!(manager.get_scene("test").unwrap().name, "Test Scene");
    }

    #[test]
    fn test_push_scene() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Main Scene");
        manager.register_active_scene("main", scene);

        // Push scene onto stack
        let result = manager.push_scene("main");
        assert!(result.is_ok());
        assert_eq!(manager.stack_depth(), 1);
        assert_eq!(manager.active_scene_name(), Some("main"));
    }

    #[test]
    fn test_push_scene_not_loaded() {
        let mut manager = SceneManager::new();

        // Try to push a scene that doesn't exist
        let result = manager.push_scene("nonexistent");
        assert!(result.is_err());
        match result {
            Err(SceneError::NotLoaded(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected NotLoaded error"),
        }
    }

    #[test]
    fn test_pop_scene() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Test");
        manager.register_active_scene("test", scene);
        manager.push_scene("test").unwrap();

        // Pop scene
        let popped = manager.pop_scene();
        assert_eq!(popped, Some("test".to_string()));
        assert_eq!(manager.stack_depth(), 0);
        assert!(manager.active_scene().is_none());

        // Scene should still exist in storage
        assert!(manager.get_scene("test").is_some());
    }

    #[test]
    fn test_switch_to() {
        let mut manager = SceneManager::new();
        let scene1 = ActiveScene::new("Scene 1");
        let scene2 = ActiveScene::new("Scene 2");
        manager.register_active_scene("scene1", scene1);
        manager.register_active_scene("scene2", scene2);

        // Push first scene
        manager.push_scene("scene1").unwrap();
        assert_eq!(manager.active_scene_name(), Some("scene1"));

        // Switch to second scene
        let result = manager.switch_to("scene2");
        assert!(result.is_ok());
        assert_eq!(manager.stack_depth(), 1);
        assert_eq!(manager.active_scene_name(), Some("scene2"));
    }

    #[test]
    fn test_switch_to_empty_stack() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Test");
        manager.register_active_scene("test", scene);

        // Switch on empty stack should act like push
        let result = manager.switch_to("test");
        assert!(result.is_ok());
        assert_eq!(manager.stack_depth(), 1);
        assert_eq!(manager.active_scene_name(), Some("test"));
    }

    #[test]
    fn test_scene_stack_overlay() {
        let mut manager = SceneManager::new();

        // Create game scene
        let game = ActiveScene::new("Game");
        manager.register_active_scene("game", game);

        // Create pause menu
        let pause = ActiveScene::new("Pause Menu");
        manager.register_active_scene("pause", pause);

        // Start with game scene
        manager.push_scene("game").unwrap();
        assert_eq!(manager.active_scene_name(), Some("game"));

        // Push pause menu as overlay
        manager.push_scene("pause").unwrap();
        assert_eq!(manager.stack_depth(), 2);
        assert_eq!(manager.active_scene_name(), Some("pause"));

        // Both scenes should be on the stack
        assert!(manager.is_scene_active("game"));
        assert!(manager.is_scene_active("pause"));

        // Pop pause menu to return to game
        manager.pop_scene();
        assert_eq!(manager.stack_depth(), 1);
        assert_eq!(manager.active_scene_name(), Some("game"));
    }

    #[test]
    fn test_active_world_access() {
        let mut manager = SceneManager::new();

        // Create scene with an entity
        let mut scene = ActiveScene::new("Test");
        let tesseract = Tesseract4D::new(2.0);
        scene.world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            Name("cube".to_string()),
        ));
        manager.register_active_scene("test", scene);
        manager.push_scene("test").unwrap();

        // Should be able to access the world
        let world = manager.active_world();
        assert!(world.is_some());
        assert_eq!(world.unwrap().entity_count(), 1);

        // Should be able to access entity by name
        let entity = world.unwrap().get_by_name("cube");
        assert!(entity.is_some());
    }

    #[test]
    fn test_active_world_mut_access() {
        let mut manager = SceneManager::new();

        let scene = ActiveScene::new("Test");
        manager.register_active_scene("test", scene);
        manager.push_scene("test").unwrap();

        // Mutably access the world and add an entity
        {
            let world = manager.active_world_mut();
            assert!(world.is_some());
            spawn_test_entity(world.unwrap());
        }

        // Verify entity was added
        assert_eq!(manager.active_world().unwrap().entity_count(), 1);
    }

    #[test]
    fn test_scene_not_loaded_error() {
        let mut manager = SceneManager::new();

        // Try to instantiate a template that doesn't exist
        let result = manager.instantiate("nonexistent");
        assert!(result.is_err());
        match result {
            Err(SceneError::NotLoaded(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected NotLoaded error"),
        }
    }

    #[test]
    fn test_update() {
        let mut manager = SceneManager::new()
            .with_physics(PhysicsConfig::new(-20.0));

        // Create scene with physics
        let scene = ActiveScene::new("Test")
            .with_physics(PhysicsConfig::new(-20.0));
        manager.register_active_scene("test", scene);
        manager.push_scene("test").unwrap();

        // Update should not panic
        manager.update(0.016);
    }

    #[test]
    fn test_register_template() {
        let mut manager = SceneManager::new();

        // Create and register a template
        let template = Scene::new("My Template")
            .with_gravity(-15.0);
        manager.register_template(template);

        // Should be able to retrieve it
        let retrieved = manager.get_template("My Template");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().gravity, Some(-15.0));
    }

    #[test]
    fn test_instantiate_from_template() {
        let mut manager = SceneManager::new();

        // Register a template
        let template = Scene::new("Level 1")
            .with_gravity(-10.0)
            .with_player_spawn(0.0, 1.0, 5.0, 0.0);
        manager.register_template(template);

        // Instantiate it
        let result = manager.instantiate("Level 1");
        assert!(result.is_ok());

        // The instantiated scene should exist
        let scene = manager.get_scene("Level 1");
        assert!(scene.is_some());
        assert_eq!(scene.unwrap().name, "Level 1");
        assert_eq!(scene.unwrap().player_spawn, Some([0.0, 1.0, 5.0, 0.0]));
    }

    #[test]
    fn test_default() {
        let manager = SceneManager::default();
        assert!(manager.active_scene().is_none());
    }

    // --- Transition tests ---

    #[test]
    fn test_switch_with_transition() {
        let mut manager = SceneManager::new();
        let scene1 = ActiveScene::new("Scene 1");
        let scene2 = ActiveScene::new("Scene 2");
        manager.register_active_scene("scene1", scene1);
        manager.register_active_scene("scene2", scene2);
        manager.push_scene("scene1").unwrap();

        // Start a transition
        let result = manager.switch_to_with_transition(
            "scene2",
            TransitionEffect::Fade {
                duration: std::time::Duration::from_secs(1),
            },
        );
        assert!(result.is_ok());
        assert!(manager.is_transitioning());
        assert!(manager.current_transition().is_some());

        // Should still be on scene1 during transition
        assert_eq!(manager.active_scene_name(), Some("scene1"));
    }

    #[test]
    fn test_switch_with_transition_not_loaded() {
        let mut manager = SceneManager::new();

        let result = manager.switch_to_with_transition(
            "nonexistent",
            TransitionEffect::Instant,
        );
        assert!(result.is_err());
        match result {
            Err(SceneError::NotLoaded(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected NotLoaded error"),
        }
    }

    #[test]
    fn test_update_transition_completes_instant() {
        let mut manager = SceneManager::new();
        let scene1 = ActiveScene::new("Scene 1");
        let scene2 = ActiveScene::new("Scene 2");
        manager.register_active_scene("scene1", scene1);
        manager.register_active_scene("scene2", scene2);
        manager.push_scene("scene1").unwrap();

        // Instant transition should complete in one update
        manager.switch_to_with_transition("scene2", TransitionEffect::Instant).unwrap();
        let completed = manager.update_transition();
        assert!(completed);
        assert!(!manager.is_transitioning());

        // Should now be on scene2
        assert_eq!(manager.active_scene_name(), Some("scene2"));
    }

    #[test]
    fn test_is_transitioning() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Test");
        manager.register_active_scene("test", scene);

        // No transition initially
        assert!(!manager.is_transitioning());

        // Start transition
        manager.switch_to_with_transition(
            "test",
            TransitionEffect::Fade {
                duration: std::time::Duration::from_secs(10),
            },
        ).unwrap();
        assert!(manager.is_transitioning());
    }

    #[test]
    fn test_update_transition_no_transition() {
        let mut manager = SceneManager::new();
        // Updating with no transition should return false
        let completed = manager.update_transition();
        assert!(!completed);
    }

    // --- Overlay tests ---

    #[test]
    fn test_push_overlay() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Game");
        let hud = ActiveScene::new("HUD");
        manager.register_active_scene("game", scene);
        manager.register_active_scene("hud", hud);
        manager.push_scene("game").unwrap();

        // Push an overlay
        let result = manager.push_overlay("hud");
        assert!(result.is_ok());
        assert_eq!(manager.overlays().len(), 1);
        assert_eq!(manager.overlays()[0], "hud");
    }

    #[test]
    fn test_pop_overlay() {
        let mut manager = SceneManager::new();
        let scene = ActiveScene::new("Game");
        let hud = ActiveScene::new("HUD");
        manager.register_active_scene("game", scene);
        manager.register_active_scene("hud", hud);
        manager.push_scene("game").unwrap();

        manager.push_overlay("hud").unwrap();
        let popped = manager.pop_overlay();
        assert_eq!(popped, Some("hud".to_string()));
        assert!(manager.overlays().is_empty());
    }

    #[test]
    fn test_pop_overlay_empty() {
        let mut manager = SceneManager::new();
        let popped = manager.pop_overlay();
        assert!(popped.is_none());
    }

    #[test]
    fn test_overlay_not_loaded_error() {
        let mut manager = SceneManager::new();
        let result = manager.push_overlay("nonexistent");
        assert!(result.is_err());
        match result {
            Err(SceneError::NotLoaded(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected NotLoaded error"),
        }
    }

    #[test]
    fn test_overlays_accessor() {
        let mut manager = SceneManager::new();
        let hud = ActiveScene::new("HUD");
        let minimap = ActiveScene::new("Minimap");
        manager.register_active_scene("hud", hud);
        manager.register_active_scene("minimap", minimap);

        assert!(manager.overlays().is_empty());

        manager.push_overlay("hud").unwrap();
        manager.push_overlay("minimap").unwrap();

        let overlays = manager.overlays();
        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0], "hud");
        assert_eq!(overlays[1], "minimap");
    }

    #[test]
    fn test_is_overlay() {
        let mut manager = SceneManager::new();
        let hud = ActiveScene::new("HUD");
        let game = ActiveScene::new("Game");
        manager.register_active_scene("hud", hud);
        manager.register_active_scene("game", game);

        manager.push_overlay("hud").unwrap();

        assert!(manager.is_overlay("hud"));
        assert!(!manager.is_overlay("game"));
        assert!(!manager.is_overlay("nonexistent"));
    }

    // --- Async loading tests ---

    #[test]
    fn test_poll_loading_empty() {
        let mut manager = SceneManager::new();
        let loaded = manager.poll_loading();
        assert!(loaded.is_empty());
    }
}
