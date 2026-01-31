//! World container for entities (hecs ECS backend)
//!
//! The World manages all entities in the simulation using hecs for storage.
//! It provides side-tables for name lookups and physics integration,
//! and stores hierarchy as Parent/Children components.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use crate::components::*;
use crate::{DirtyFlags, Transform4D};
use rust4d_physics::{PhysicsConfig, PhysicsWorld};

/// Error type for hierarchy operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyError {
    /// One or both entities don't exist in the world
    InvalidEntity,
    /// Adding this child would create a cycle in the hierarchy
    CyclicHierarchy,
    /// The entity is already a child of the specified parent
    AlreadyChild,
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HierarchyError::InvalidEntity => write!(f, "one or both entities do not exist"),
            HierarchyError::CyclicHierarchy => write!(f, "adding this child would create a cycle"),
            HierarchyError::AlreadyChild => write!(f, "entity is already a child of this parent"),
        }
    }
}

impl std::error::Error for HierarchyError {}

/// The 4D world containing all entities (ECS-backed)
///
/// Thin wrapper around hecs::World with side-tables for:
/// - Name lookups (HashMap<String, hecs::Entity>)
/// - Physics simulation (optional PhysicsWorld)
///
/// Hierarchy is stored as ECS components (Parent, Children) rather
/// than separate HashMaps.
pub struct World {
    /// The hecs ECS world
    ecs: hecs::World,
    /// Index from entity names to hecs entities (for fast name lookup)
    name_index: HashMap<String, hecs::Entity>,
    /// Optional physics simulation
    physics_world: Option<PhysicsWorld>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            ecs: hecs::World::new(),
            name_index: HashMap::new(),
            physics_world: None,
        }
    }

    /// Enable physics for this world
    pub fn with_physics(mut self, config: PhysicsConfig) -> Self {
        self.physics_world = Some(PhysicsWorld::with_config(config));
        self
    }

    /// Get the physics world (if enabled)
    pub fn physics(&self) -> Option<&PhysicsWorld> {
        self.physics_world.as_ref()
    }

    /// Get mutable physics world (if enabled)
    pub fn physics_mut(&mut self) -> Option<&mut PhysicsWorld> {
        self.physics_world.as_mut()
    }

    // === Entity Spawning ===

    /// Spawn an entity from a component bundle (tuple or EntityBuilder)
    ///
    /// This is the PRIMARY way to create entities in the ECS API.
    /// Returns the hecs::Entity handle.
    ///
    /// After spawning, checks for a Name component and updates the name index.
    pub fn spawn(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
        let entity = self.ecs.spawn(components);

        // Check if spawned entity has a Name component and index it
        if let Ok(name) = self.ecs.get::<&Name>(entity) {
            self.name_index.insert(name.0.clone(), entity);
        }

        entity
    }

    /// Legacy compatibility: spawn from the old Entity struct
    ///
    /// TRANSITIONAL METHOD -- decomposes the monolithic Entity into individual
    /// ECS components and spawns them.
    pub fn add_entity(&mut self, entity: crate::entity::Entity) -> hecs::Entity {
        let mut builder = hecs::EntityBuilder::new();

        builder.add(entity.transform);
        builder.add(entity.shape);
        builder.add(entity.material);
        builder.add(DirtyFlags::ALL);

        if let Some(name) = entity.name {
            builder.add(Name(name));
        }

        if !entity.tags.is_empty() {
            builder.add(Tags(entity.tags));
        }

        if let Some(body_key) = entity.physics_body {
            builder.add(PhysicsBody(body_key));
        }

        self.spawn(builder.build())
    }

    // === Entity Removal ===

    /// Remove an entity from the world
    ///
    /// Cleans up:
    /// - Name index (if entity had Name component)
    /// - Physics body (if entity had PhysicsBody component)
    /// - Hierarchy (removes from parent, orphans children)
    ///
    /// Returns true if entity existed and was removed.
    pub fn despawn(&mut self, entity: hecs::Entity) -> bool {
        if !self.ecs.contains(entity) {
            return false;
        }

        // Clean up name index
        if let Ok(name) = self.ecs.get::<&Name>(entity) {
            let name_str = name.0.clone();
            drop(name);
            self.name_index.remove(&name_str);
        }

        // Clean up physics body
        if let Ok(body) = self.ecs.get::<&PhysicsBody>(entity) {
            let body_key = body.0;
            drop(body);
            if let Some(ref mut physics) = self.physics_world {
                physics.remove_body(body_key);
            }
        }

        // Clean up hierarchy: remove from parent's Children list
        let parent_entity = self.ecs.get::<&Parent>(entity).ok().map(|p| p.0);
        if let Some(parent_entity) = parent_entity {
            if let Ok(mut children) = self.ecs.get::<&mut Children>(parent_entity) {
                children.remove(entity);
            }
        }

        // Orphan all children (remove Parent component from them)
        let child_list: Vec<hecs::Entity> = self.ecs.get::<&Children>(entity)
            .ok()
            .map(|c| c.0.clone())
            .unwrap_or_default();
        for child in child_list {
            let _ = self.ecs.remove_one::<Parent>(child);
        }

        self.ecs.despawn(entity).is_ok()
    }

    /// Legacy alias for despawn
    pub fn remove_entity(&mut self, entity: hecs::Entity) -> bool {
        self.despawn(entity)
    }

    // === Entity Access ===

    /// Check if an entity exists
    pub fn contains(&self, entity: hecs::Entity) -> bool {
        self.ecs.contains(entity)
    }

    /// Direct access to the underlying hecs World (for advanced queries)
    pub fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    /// Mutable access to the underlying hecs World
    pub fn ecs_mut(&mut self) -> &mut hecs::World {
        &mut self.ecs
    }

    // === Name Lookups ===

    /// Get an entity by name
    ///
    /// Returns the Entity handle if found.
    pub fn get_by_name(&self, name: &str) -> Option<hecs::Entity> {
        self.name_index.get(name).copied()
    }

    // === Tag Queries ===

    /// Get all entities with a specific tag
    ///
    /// This performs a linear scan of all entities with Tags component.
    pub fn get_by_tag(&self, tag: &str) -> Vec<hecs::Entity> {
        let mut result = Vec::new();
        for (entity, tags) in self.ecs.query::<&Tags>().iter() {
            if tags.has(tag) {
                result.push(entity);
            }
        }
        result
    }

    // === Entity Count and Iteration ===

    /// Get the number of entities
    #[inline]
    pub fn entity_count(&self) -> usize {
        // hecs::World::len() returns u32
        self.ecs.len() as usize
    }

    /// Check if the world is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ecs.is_empty()
    }

    /// Clear all entities from the world
    pub fn clear(&mut self) {
        self.ecs.clear();
        self.name_index.clear();
    }

    // === Physics Update ===

    /// Update the world by stepping physics and syncing entity transforms
    ///
    /// This method:
    /// 1. Steps the physics simulation (if enabled)
    /// 2. Syncs entity transforms from their associated physics bodies
    /// 3. Marks entities dirty when their transforms change
    pub fn update(&mut self, dt: f32) {
        // Update the physics simulation (fixed timestep accumulator)
        if let Some(ref mut physics) = self.physics_world {
            physics.update(dt);
        }

        // Sync entity transforms from their physics bodies
        if let Some(ref physics) = self.physics_world {
            for (_entity, (transform, body, dirty)) in
                self.ecs.query_mut::<(&mut Transform4D, &PhysicsBody, &mut DirtyFlags)>()
            {
                if let Some(phys_body) = physics.get_body(body.0) {
                    if transform.position != phys_body.position {
                        transform.position = phys_body.position;
                        *dirty |= DirtyFlags::TRANSFORM;
                    }
                }
            }
        }
    }

    // === Dirty Tracking ===

    /// Check if any entity in the world has dirty flags set
    pub fn has_dirty_entities(&self) -> bool {
        for (_, dirty) in self.ecs.query::<&DirtyFlags>().iter() {
            if !dirty.is_empty() {
                return true;
            }
        }
        false
    }

    /// Clear dirty flags on all entities
    pub fn clear_all_dirty(&mut self) {
        for (_, dirty) in self.ecs.query_mut::<&mut DirtyFlags>() {
            *dirty = DirtyFlags::NONE;
        }
    }

    // === Hierarchy Methods ===

    /// Get an entity's parent
    pub fn parent_of(&self, entity: hecs::Entity) -> Option<hecs::Entity> {
        self.ecs.get::<&Parent>(entity).ok().map(|p| p.0)
    }

    /// Get an entity's children
    pub fn children_of(&self, entity: hecs::Entity) -> Vec<hecs::Entity> {
        self.ecs.get::<&Children>(entity)
            .ok()
            .map(|c| c.0.clone())
            .unwrap_or_default()
    }

    /// Check if an entity has any children
    pub fn has_children(&self, entity: hecs::Entity) -> bool {
        self.ecs.get::<&Children>(entity)
            .ok()
            .is_some_and(|c| !c.0.is_empty())
    }

    /// Check if an entity has a parent
    pub fn has_parent(&self, entity: hecs::Entity) -> bool {
        self.ecs.get::<&Parent>(entity).is_ok()
    }

    /// Add a child entity to a parent entity
    ///
    /// If the child already has a different parent, it is first removed from
    /// that parent (reparenting). Returns an error if either entity does not
    /// exist, if the relationship would create a cycle, or if the child is
    /// already a child of the specified parent.
    pub fn add_child(&mut self, parent: hecs::Entity, child: hecs::Entity) -> Result<(), HierarchyError> {
        // Validate both entities exist
        if !self.ecs.contains(parent) || !self.ecs.contains(child) {
            return Err(HierarchyError::InvalidEntity);
        }

        // Cannot parent an entity to itself
        if parent == child {
            return Err(HierarchyError::CyclicHierarchy);
        }

        // Check if child is already a child of this parent
        if let Ok(existing_parent) = self.ecs.get::<&Parent>(child) {
            if existing_parent.0 == parent {
                return Err(HierarchyError::AlreadyChild);
            }
        }

        // Check for cycles: walk up from parent; if we reach child, it would create a cycle
        if self.is_ancestor(child, parent) {
            return Err(HierarchyError::CyclicHierarchy);
        }

        // If child already has a different parent, remove it from that parent first
        let old_parent = self.ecs.get::<&Parent>(child).ok().map(|p| p.0);
        if let Some(old_parent_entity) = old_parent {
            if let Ok(mut old_children) = self.ecs.get::<&mut Children>(old_parent_entity) {
                old_children.remove(child);
            }
        }

        // Set the Parent component on child (insert_one replaces if exists, adds if not)
        let _ = self.ecs.insert_one(child, Parent(parent));

        // Add to parent's Children component
        let has_children = self.ecs.get::<&Children>(parent).is_ok();
        if has_children {
            if let Ok(mut children) = self.ecs.get::<&mut Children>(parent) {
                children.add(child);
            }
        } else {
            // Parent doesn't have Children component yet -- add it
            let _ = self.ecs.insert_one(parent, Children(vec![child]));
        }

        Ok(())
    }

    /// Remove an entity from its parent, making it a root entity
    ///
    /// Does nothing if the entity has no parent or does not exist.
    pub fn remove_from_parent(&mut self, child: hecs::Entity) {
        let parent_entity = self.ecs.get::<&Parent>(child).ok().map(|p| p.0);
        if let Some(parent_entity) = parent_entity {
            // Remove from parent's Children
            if let Ok(mut children) = self.ecs.get::<&mut Children>(parent_entity) {
                children.remove(child);
            }

            // Remove Parent component from child
            let _ = self.ecs.remove_one::<Parent>(child);
        }
    }

    /// Get the world-space transform of an entity
    ///
    /// For root entities (no parent), this is just their own local transform.
    /// For children, this composes transforms from root to leaf.
    ///
    /// Returns `None` if the entity does not exist.
    pub fn world_transform(&self, entity: hecs::Entity) -> Option<Transform4D> {
        let local_transform = *self.ecs.get::<&Transform4D>(entity).ok()?;

        // Build the chain of ancestors from leaf to root
        let mut chain = vec![local_transform];
        let mut current = entity;
        while let Ok(parent) = self.ecs.get::<&Parent>(current) {
            let parent_entity = parent.0;
            drop(parent);
            if let Ok(parent_transform) = self.ecs.get::<&Transform4D>(parent_entity) {
                chain.push(*parent_transform);
                current = parent_entity;
            } else {
                break;
            }
        }

        // Compose from root (last element) to leaf (first element)
        let mut result = Transform4D::identity();
        for transform in chain.into_iter().rev() {
            result = result.compose(&transform);
        }

        Some(result)
    }

    /// Delete an entity and all its descendants recursively
    ///
    /// Returns a vector of all removed entity handles.
    pub fn delete_recursive(&mut self, entity: hecs::Entity) -> Vec<hecs::Entity> {
        let mut removed = Vec::new();

        // Collect all descendants (breadth-first)
        let mut queue = VecDeque::new();
        queue.push_back(entity);

        let mut to_remove = Vec::new();
        while let Some(e) = queue.pop_front() {
            to_remove.push(e);
            if let Ok(children) = self.ecs.get::<&Children>(e) {
                for &child in &children.0 {
                    queue.push_back(child);
                }
            }
        }

        // Detach root from its parent
        self.remove_from_parent(entity);

        // Despawn all (despawn handles name + physics cleanup)
        for e in to_remove {
            if self.despawn(e) {
                removed.push(e);
            }
        }

        removed
    }

    /// Get all descendants of an entity (breadth-first order)
    ///
    /// Returns an empty vector if the entity has no descendants or does not exist.
    /// Does not include the entity itself.
    pub fn descendants(&self, entity: hecs::Entity) -> Vec<hecs::Entity> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Seed with direct children
        if let Ok(children) = self.ecs.get::<&Children>(entity) {
            for &child in &children.0 {
                queue.push_back(child);
            }
        }

        while let Some(e) = queue.pop_front() {
            result.push(e);
            if let Ok(children) = self.ecs.get::<&Children>(e) {
                for &child in &children.0 {
                    queue.push_back(child);
                }
            }
        }

        result
    }

    /// Get all root entities (entities with no Parent component)
    pub fn root_entities(&self) -> Vec<hecs::Entity> {
        let mut roots = Vec::new();
        for entity in self.ecs.iter() {
            if self.ecs.get::<&Parent>(entity.entity()).is_err() {
                roots.push(entity.entity());
            }
        }
        roots
    }

    /// Check if `ancestor` is an ancestor of `entity`
    ///
    /// Walks up the hierarchy from `entity`. Returns `false` if either
    /// entity does not exist, or if `ancestor == entity`.
    pub fn is_ancestor(&self, ancestor: hecs::Entity, entity: hecs::Entity) -> bool {
        let mut current = entity;
        while let Ok(parent) = self.ecs.get::<&Parent>(current) {
            if parent.0 == ancestor {
                return true;
            }
            current = parent.0;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, ShapeRef, Entity};
    use rust4d_math::Tesseract4D;

    fn make_test_entity() -> Entity {
        let tesseract = Tesseract4D::new(2.0);
        Entity::new(ShapeRef::shared(tesseract))
    }

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert!(world.is_empty());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_world_add_entity() {
        let mut world = World::new();
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Key should be valid
        assert!(world.contains(key));
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_world_get_entity() {
        let mut world = World::new();
        let entity = make_test_entity();
        let handle = world.add_entity(entity);

        let shape = world.ecs().get::<&ShapeRef>(handle);
        assert!(shape.is_ok());
        assert_eq!(shape.unwrap().as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_world_get_entity_mut() {
        let mut world = World::new();
        let entity = make_test_entity();
        let handle = world.add_entity(entity);

        {
            let mut material = world.ecs_mut().get::<&mut Material>(handle).unwrap();
            *material = Material::RED;
        }

        let material = world.ecs().get::<&Material>(handle).unwrap();
        assert_eq!(material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_world_entity_count() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn test_world_clear() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        world.clear();
        assert!(world.is_empty());
    }

    #[test]
    fn test_world_ecs_query() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        let count = world.ecs().query::<&Transform4D>().iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_world_update() {
        let mut world = World::new();
        world.add_entity(make_test_entity());

        // Just verify it doesn't panic for now
        world.update(0.016);
    }

    #[test]
    fn test_world_default() {
        let world = World::default();
        assert!(world.is_empty());
    }

    #[test]
    fn test_stale_entity_returns_false() {
        let mut world = World::new();
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Key is valid initially
        assert!(world.contains(key));

        // Remove the entity
        let removed = world.remove_entity(key);
        assert!(removed);

        // Key is now stale
        assert!(!world.contains(key));

        // Add a new entity
        let new_entity = make_test_entity();
        let new_key = world.add_entity(new_entity);

        // Old key still invalid
        assert!(!world.contains(key));
        // New key works
        assert!(world.contains(new_key));
    }

    #[test]
    fn test_world_with_physics() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics enabled (no gravity for predictable test)
        let config = PhysicsConfig::new(0.0);
        let mut world = World::new().with_physics(config);

        assert!(world.physics().is_some());

        // Add a physics body with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Verify initial position
        {
            let transform = world.ecs().get::<&Transform4D>(entity_handle).unwrap();
            assert_eq!(transform.position.x, 0.0);
        }

        // Simulate 1 second in realistic frame-sized increments
        for _ in 0..60 {
            world.update(1.0 / 60.0);
        }

        // Entity transform should now reflect the physics body position
        let transform = world.ecs().get::<&Transform4D>(entity_handle).unwrap();
        assert!((transform.position.x - 10.0).abs() < 0.2,
            "Expected ~10.0, got {}", transform.position.x);
        assert!((transform.position.y - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_physics_sync_with_gravity() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with gravity (default config)
        let mut world = World::new().with_physics(PhysicsConfig::default());

        // Add a physics body that will fall
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5);
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Step physics
        world.update(0.1);

        // Entity should have fallen
        let transform = world.ecs().get::<&Transform4D>(entity_handle).unwrap();
        assert!(transform.position.y < 10.0);
    }

    #[test]
    fn test_entity_without_physics_body() {
        // Create a world with physics
        let mut world = World::new().with_physics(PhysicsConfig::default());

        // Add an entity WITHOUT a physics body but with a specific position
        let mut entity = make_test_entity();
        entity.transform.position = rust4d_math::Vec4::new(5.0, 5.0, 5.0, 5.0);
        let entity_handle = world.add_entity(entity);

        // Step physics
        world.update(1.0);

        // Entity position should be unchanged (not linked to physics)
        let transform = world.ecs().get::<&Transform4D>(entity_handle).unwrap();
        assert_eq!(transform.position.x, 5.0);
        assert_eq!(transform.position.y, 5.0);
    }

    #[test]
    fn test_get_by_name() {
        let mut world = World::new();

        // Add a named entity
        let entity = make_test_entity().with_name("tesseract");
        let key = world.add_entity(entity);

        // Should be able to find by name
        let result = world.get_by_name("tesseract");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), key);

        // Verify the entity has the Name component
        let name = world.ecs().get::<&Name>(key).unwrap();
        assert_eq!(name.0, "tesseract");

        // Non-existent name should return None
        assert!(world.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_get_by_tag() {
        let mut world = World::new();

        // Add entities with different tags
        let dynamic1 = make_test_entity().with_tag("dynamic").with_name("dyn1");
        let dynamic2 = make_test_entity().with_tag("dynamic").with_name("dyn2");
        let static1 = make_test_entity().with_tag("static").with_name("stat1");
        world.add_entity(dynamic1);
        world.add_entity(dynamic2);
        world.add_entity(static1);

        // Should find 2 dynamic entities
        let dynamic_entities = world.get_by_tag("dynamic");
        assert_eq!(dynamic_entities.len(), 2);

        // Should find 1 static entity
        let static_entities = world.get_by_tag("static");
        assert_eq!(static_entities.len(), 1);

        // Non-existent tag should return empty
        let none_entities = world.get_by_tag("nonexistent");
        assert!(none_entities.is_empty());
    }

    #[test]
    fn test_name_index_cleanup_on_remove() {
        let mut world = World::new();

        // Add a named entity
        let entity = make_test_entity().with_name("tesseract");
        let key = world.add_entity(entity);

        // Should be able to find by name
        assert!(world.get_by_name("tesseract").is_some());

        // Remove the entity
        world.remove_entity(key);

        // Name should no longer be in the index
        assert!(world.get_by_name("tesseract").is_none());
    }

    #[test]
    fn test_name_index_cleanup_on_clear() {
        let mut world = World::new();

        // Add named entities
        world.add_entity(make_test_entity().with_name("entity1"));
        world.add_entity(make_test_entity().with_name("entity2"));

        // Should be able to find by name
        assert!(world.get_by_name("entity1").is_some());
        assert!(world.get_by_name("entity2").is_some());

        // Clear the world
        world.clear();

        // Names should no longer be in the index
        assert!(world.get_by_name("entity1").is_none());
        assert!(world.get_by_name("entity2").is_none());
    }

    #[test]
    fn test_entity_without_name() {
        let mut world = World::new();

        // Add an unnamed entity
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Entity should exist but not be findable by any name
        assert!(world.contains(key));
        assert!(world.get_by_name("").is_none());
    }

    // --- Dirty tracking tests ---

    #[test]
    fn test_new_entities_are_dirty() {
        let mut world = World::new();
        let key = world.add_entity(make_test_entity());

        // New entities should be dirty (DirtyFlags::ALL)
        let dirty = world.ecs().get::<&DirtyFlags>(key).unwrap();
        assert!(!dirty.is_empty());
        assert!(world.has_dirty_entities());
    }

    #[test]
    fn test_clear_all_dirty() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        // Both should be dirty initially
        assert!(world.has_dirty_entities());

        // Clear all dirty flags
        world.clear_all_dirty();

        // None should be dirty now
        assert!(!world.has_dirty_entities());
    }

    #[test]
    fn test_dirty_entities_tracking() {
        let mut world = World::new();
        let key1 = world.add_entity(make_test_entity());
        let _key2 = world.add_entity(make_test_entity());

        // Clear dirty flags
        world.clear_all_dirty();

        // Manually mark one as dirty
        {
            let mut dirty = world.ecs_mut().get::<&mut DirtyFlags>(key1).unwrap();
            *dirty |= DirtyFlags::TRANSFORM;
        }

        // Should have dirty entities now
        assert!(world.has_dirty_entities());

        // Count dirty entities via query
        let dirty_count = world.ecs().query::<&DirtyFlags>().iter()
            .filter(|(_, d)| !d.is_empty())
            .count();
        assert_eq!(dirty_count, 1);
    }

    #[test]
    fn test_physics_sync_marks_dirty() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics enabled (no gravity for predictable test)
        let config = PhysicsConfig::new(0.0);
        let mut world = World::new().with_physics(config);

        // Add a physics body with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Clear dirty flags
        world.clear_all_dirty();
        assert!(!world.has_dirty_entities());

        // Step physics - entity should move and become dirty
        world.update(1.0);

        // Entity should now be dirty
        let dirty = world.ecs().get::<&DirtyFlags>(entity_handle).unwrap();
        assert!(!dirty.is_empty());
        assert!(dirty.contains(DirtyFlags::TRANSFORM));
    }

    #[test]
    fn test_physics_sync_no_change_not_dirty() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics (no gravity, no velocity = no movement)
        let config = PhysicsConfig::new(0.0);
        let mut world = World::new().with_physics(config);

        // Add a stationary physics body
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5);
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Clear dirty flags
        world.clear_all_dirty();

        // Step physics - no movement should occur
        world.update(1.0);

        // Entity should NOT be dirty (position didn't change)
        let dirty = world.ecs().get::<&DirtyFlags>(entity_handle).unwrap();
        assert!(dirty.is_empty());
    }

    // --- Hierarchy tests ---

    fn make_positioned_entity(x: f32, y: f32, z: f32, w: f32) -> Entity {
        let tesseract = Tesseract4D::new(2.0);
        Entity::with_transform(
            ShapeRef::shared(tesseract),
            crate::Transform4D::from_position(rust4d_math::Vec4::new(x, y, z, w)),
            Material::default(),
        )
    }

    #[test]
    fn test_add_child() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        assert!(world.add_child(parent, child).is_ok());

        // Verify parent/child relationship
        assert_eq!(world.parent_of(child), Some(parent));
        assert_eq!(world.children_of(parent), vec![child]);
        assert!(world.has_children(parent));
        assert!(world.has_parent(child));
        assert!(!world.has_parent(parent));
        assert!(!world.has_children(child));
    }

    #[test]
    fn test_add_child_invalid_entity() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());

        // Create an invalid entity by adding and removing
        let temp = world.add_entity(make_test_entity());
        world.remove_entity(temp);

        // Both invalid child and invalid parent should fail
        assert_eq!(
            world.add_child(parent, temp),
            Err(HierarchyError::InvalidEntity)
        );
        assert_eq!(
            world.add_child(temp, parent),
            Err(HierarchyError::InvalidEntity)
        );
    }

    #[test]
    fn test_cycle_detection() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());

        // A -> B
        assert!(world.add_child(a, b).is_ok());

        // B -> A would create a cycle
        assert_eq!(
            world.add_child(b, a),
            Err(HierarchyError::CyclicHierarchy)
        );

        // Self-parenting should also be rejected
        assert_eq!(
            world.add_child(a, a),
            Err(HierarchyError::CyclicHierarchy)
        );
    }

    #[test]
    fn test_deep_cycle_detection() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());
        let c = world.add_entity(make_test_entity());

        // A -> B -> C
        assert!(world.add_child(a, b).is_ok());
        assert!(world.add_child(b, c).is_ok());

        // C -> A would create a cycle (A is ancestor of C)
        assert_eq!(
            world.add_child(c, a),
            Err(HierarchyError::CyclicHierarchy)
        );
    }

    #[test]
    fn test_already_child() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        assert!(world.add_child(parent, child).is_ok());

        // Adding the same child again should return AlreadyChild
        assert_eq!(
            world.add_child(parent, child),
            Err(HierarchyError::AlreadyChild)
        );
    }

    #[test]
    fn test_remove_from_parent() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        assert!(world.has_parent(child));
        assert!(world.has_children(parent));

        world.remove_from_parent(child);

        assert!(!world.has_parent(child));
        assert!(!world.has_children(parent));
        assert_eq!(world.parent_of(child), None);
        assert!(world.children_of(parent).is_empty());
    }

    #[test]
    fn test_world_transform_no_parent() {
        let mut world = World::new();
        let entity = make_positioned_entity(1.0, 2.0, 3.0, 4.0);
        let key = world.add_entity(entity);

        let wt = world.world_transform(key).unwrap();
        assert!((wt.position.x - 1.0).abs() < 0.001);
        assert!((wt.position.y - 2.0).abs() < 0.001);
        assert!((wt.position.z - 3.0).abs() < 0.001);
        assert!((wt.position.w - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_world_transform_with_parent() {
        let mut world = World::new();

        // Parent at (10, 0, 0, 0)
        let parent = world.add_entity(make_positioned_entity(10.0, 0.0, 0.0, 0.0));
        // Child at (1, 2, 0, 0) in local space
        let child = world.add_entity(make_positioned_entity(1.0, 2.0, 0.0, 0.0));

        world.add_child(parent, child).unwrap();

        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 11.0).abs() < 0.001,
            "Expected x=11.0, got {}", wt.position.x);
        assert!((wt.position.y - 2.0).abs() < 0.001,
            "Expected y=2.0, got {}", wt.position.y);
    }

    #[test]
    fn test_world_transform_with_scale() {
        let mut world = World::new();

        // Parent with scale 2 at origin
        let mut parent_entity = make_positioned_entity(0.0, 0.0, 0.0, 0.0);
        parent_entity.transform.scale = 2.0;
        let parent = world.add_entity(parent_entity);

        // Child at (1, 0, 0, 0) in local space
        let child = world.add_entity(make_positioned_entity(1.0, 0.0, 0.0, 0.0));

        world.add_child(parent, child).unwrap();

        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 2.0).abs() < 0.001,
            "Expected x=2.0, got {}", wt.position.x);
    }

    #[test]
    fn test_delete_recursive() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity().with_name("root"));
        let child1 = world.add_entity(make_test_entity().with_name("child1"));
        let child2 = world.add_entity(make_test_entity().with_name("child2"));
        let grandchild = world.add_entity(make_test_entity().with_name("grandchild"));

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        assert_eq!(world.entity_count(), 4);

        let removed = world.delete_recursive(root);
        assert_eq!(removed.len(), 4);
        assert_eq!(world.entity_count(), 0);

        // All should be gone
        assert!(!world.contains(root));
        assert!(!world.contains(child1));
        assert!(!world.contains(grandchild));

        // Name index should be cleaned up
        assert!(world.get_by_name("root").is_none());
        assert!(world.get_by_name("child1").is_none());
    }

    #[test]
    fn test_delete_recursive_subtree() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity());
        let child1 = world.add_entity(make_test_entity());
        let child2 = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        // Delete just child1 subtree (child1 + grandchild)
        let removed = world.delete_recursive(child1);
        assert_eq!(removed.len(), 2);
        assert_eq!(world.entity_count(), 2);

        // root and child2 should still exist
        assert!(world.contains(root));
        assert!(world.contains(child2));

        // child1 should be removed from root's children
        assert_eq!(world.children_of(root), vec![child2]);
    }

    #[test]
    fn test_descendants() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity());
        let child1 = world.add_entity(make_test_entity());
        let child2 = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        let desc = world.descendants(root);
        assert_eq!(desc.len(), 3);
        assert!(desc.contains(&child1));
        assert!(desc.contains(&child2));
        assert!(desc.contains(&grandchild));

        // child1's descendants should be just grandchild
        let desc1 = world.descendants(child1);
        assert_eq!(desc1, vec![grandchild]);

        // Leaf entity has no descendants
        assert!(world.descendants(grandchild).is_empty());
    }

    #[test]
    fn test_root_entities() {
        let mut world = World::new();
        let root1 = world.add_entity(make_test_entity());
        let root2 = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(root1, child).unwrap();

        let roots = world.root_entities();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&root1));
        assert!(roots.contains(&root2));
        assert!(!roots.contains(&child));
    }

    #[test]
    fn test_is_ancestor() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());
        let c = world.add_entity(make_test_entity());
        let d = world.add_entity(make_test_entity());

        // A -> B -> C
        world.add_child(a, b).unwrap();
        world.add_child(b, c).unwrap();

        assert!(world.is_ancestor(a, b));  // A is ancestor of B
        assert!(world.is_ancestor(a, c));  // A is ancestor of C (transitive)
        assert!(world.is_ancestor(b, c));  // B is ancestor of C
        assert!(!world.is_ancestor(c, a)); // C is NOT ancestor of A
        assert!(!world.is_ancestor(a, a)); // Not ancestor of self
        assert!(!world.is_ancestor(a, d)); // D is unrelated
    }

    #[test]
    fn test_remove_entity_cleans_hierarchy() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        world.add_child(child, grandchild).unwrap();

        // Remove child (middle of hierarchy)
        world.remove_entity(child);

        // Parent should have no children (child was removed)
        assert!(!world.has_children(parent));

        // Grandchild should be orphaned (root entity)
        assert!(!world.has_parent(grandchild));

        // Grandchild should still exist
        assert!(world.contains(grandchild));
    }

    #[test]
    fn test_reparent() {
        let mut world = World::new();
        let parent1 = world.add_entity(make_test_entity());
        let parent2 = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        // First parent
        world.add_child(parent1, child).unwrap();
        assert_eq!(world.parent_of(child), Some(parent1));
        assert_eq!(world.children_of(parent1), vec![child]);

        // Reparent to parent2
        world.add_child(parent2, child).unwrap();
        assert_eq!(world.parent_of(child), Some(parent2));
        assert_eq!(world.children_of(parent2), vec![child]);

        // Old parent should have no children
        assert!(!world.has_children(parent1));
    }

    #[test]
    fn test_hierarchy_error_display() {
        assert_eq!(
            format!("{}", HierarchyError::InvalidEntity),
            "one or both entities do not exist"
        );
        assert_eq!(
            format!("{}", HierarchyError::CyclicHierarchy),
            "adding this child would create a cycle"
        );
        assert_eq!(
            format!("{}", HierarchyError::AlreadyChild),
            "entity is already a child of this parent"
        );
    }

    #[test]
    fn test_clear_cleans_hierarchy() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        world.clear();

        assert!(world.is_empty());
    }

    #[test]
    fn test_world_transform_deep_hierarchy() {
        let mut world = World::new();

        // Grandparent at (10, 0, 0, 0)
        let grandparent = world.add_entity(make_positioned_entity(10.0, 0.0, 0.0, 0.0));
        // Parent at (5, 0, 0, 0) local
        let parent = world.add_entity(make_positioned_entity(5.0, 0.0, 0.0, 0.0));
        // Child at (1, 0, 0, 0) local
        let child = world.add_entity(make_positioned_entity(1.0, 0.0, 0.0, 0.0));

        world.add_child(grandparent, parent).unwrap();
        world.add_child(parent, child).unwrap();

        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 16.0).abs() < 0.001,
            "Expected x=16.0, got {}", wt.position.x);
    }

    #[test]
    fn test_world_transform_nonexistent() {
        let mut world = World::new();
        let key = world.add_entity(make_test_entity());
        world.remove_entity(key);

        // Non-existent entity returns None
        assert!(world.world_transform(key).is_none());
    }
}
