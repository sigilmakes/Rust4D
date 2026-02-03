//! World container for entities (hecs ECS backend)
//!
//! The World manages all entities in the simulation using hecs for storage.
//! It provides side-tables for name lookups, tag indexing, and physics integration,
//! and stores hierarchy as Parent/Children components.

use std::collections::{HashMap, HashSet, VecDeque};
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
/// - Tag index (HashMap<String, HashSet<hecs::Entity>>)
/// - Root entity tracking (HashSet<hecs::Entity>)
/// - Dirty entity count
/// - Physics simulation (optional PhysicsWorld)
///
/// Hierarchy is stored as ECS components (Parent, Children) rather
/// than separate HashMaps.
pub struct World {
    /// The hecs ECS world
    ecs: hecs::World,
    /// Index from entity names to hecs entities (for fast name lookup)
    name_index: HashMap<String, hecs::Entity>,
    /// Index from tags to entities (for fast tag queries)
    tag_index: HashMap<String, HashSet<hecs::Entity>>,
    /// Set of root entities (no Parent component)
    roots: HashSet<hecs::Entity>,
    /// Count of entities with non-empty DirtyFlags
    dirty_count: usize,
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
            tag_index: HashMap::new(),
            roots: HashSet::new(),
            dirty_count: 0,
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
    /// After spawning, checks for Name and Tags components and updates indices.
    /// If a Name component is present and the name already exists in the index,
    /// a warning is logged and the index is updated to point to the new entity
    /// (the old entity becomes unreachable by name).
    pub fn spawn(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
        let entity = self.ecs.spawn(components);

        // Check if spawned entity has a Name component and index it
        if let Ok(name) = self.ecs.get::<&Name>(entity) {
            let name_str = name.0.clone();
            drop(name);
            if let Some(_old) = self.name_index.insert(name_str.clone(), entity) {
                log::warn!("Name '{}' already exists in world; overwriting index entry", name_str);
            }
        }

        // Check if spawned entity has Tags and index them
        if let Ok(tags) = self.ecs.get::<&Tags>(entity) {
            let tag_set: Vec<String> = tags.0.iter().cloned().collect();
            drop(tags);
            for tag in tag_set {
                self.tag_index.entry(tag).or_default().insert(entity);
            }
        }

        // Track as root (new entities have no parent)
        self.roots.insert(entity);

        // Track dirty count
        if let Ok(dirty) = self.ecs.get::<&DirtyFlags>(entity) {
            if !dirty.is_empty() {
                self.dirty_count += 1;
            }
        }

        entity
    }

    // === Entity Removal ===

    /// Remove an entity from the world
    ///
    /// Cleans up:
    /// - Name index (if entity had Name component)
    /// - Tag index (if entity had Tags component)
    /// - Root tracking
    /// - Dirty count
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

        // Clean up tag index
        if let Ok(tags) = self.ecs.get::<&Tags>(entity) {
            let tag_set: Vec<String> = tags.0.iter().cloned().collect();
            drop(tags);
            for tag in tag_set {
                if let Some(set) = self.tag_index.get_mut(&tag) {
                    set.remove(&entity);
                    // Don't remove empty sets here for performance
                }
            }
        }

        // Clean up dirty count
        if let Ok(dirty) = self.ecs.get::<&DirtyFlags>(entity) {
            if !dirty.is_empty() && self.dirty_count > 0 {
                self.dirty_count -= 1;
            }
        }

        // Clean up root tracking
        self.roots.remove(&entity);

        // Clean up physics body
        if let Ok(body) = self.ecs.get::<&PhysicsBody>(entity) {
            let body_key = body.0;
            drop(body);
            if let Some(ref mut physics) = self.physics_world {
                let result = physics.remove_body(body_key);
                if result.is_none() {
                    log::warn!("Physics body {:?} not found during entity despawn cleanup", body_key);
                }
            }
        }

        // Clean up hierarchy: remove from parent's Children list
        let parent_entity = self.ecs.get::<&Parent>(entity).ok().map(|p| p.0);
        if let Some(parent_entity) = parent_entity {
            if let Ok(mut children) = self.ecs.get::<&mut Children>(parent_entity) {
                children.remove(entity);
            }
        }

        // Orphan all children (remove Parent component, add them to roots)
        let child_list: Vec<hecs::Entity> = self.ecs.get::<&Children>(entity)
            .ok()
            .map(|c| c.0.clone())
            .unwrap_or_default();
        for child in child_list {
            let _ = self.ecs.remove_one::<Parent>(child);
            self.roots.insert(child);
        }

        self.ecs.despawn(entity).is_ok()
    }

    // === Entity Access ===

    /// Check if an entity exists
    pub fn contains(&self, entity: hecs::Entity) -> bool {
        self.ecs.contains(entity)
    }

    /// Direct access to the underlying hecs World (for queries)
    pub fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    /// Direct mutable access to the hecs World. WARNING: bypasses name index,
    /// tag index, root tracking, hierarchy, dirty count, and physics invariants.
    pub fn ecs_mut_unchecked(&mut self) -> &mut hecs::World {
        &mut self.ecs
    }

    // === Name Lookups ===

    /// Get an entity by name
    ///
    /// Returns the Entity handle if found.
    pub fn get_by_name(&self, name: &str) -> Option<hecs::Entity> {
        self.name_index.get(name).copied()
    }

    /// Rename an entity
    ///
    /// Removes the old name from the index, updates the Name component,
    /// and adds the new name to the index.
    ///
    /// Returns `Some(())` on success, `None` if the entity doesn't exist
    /// or doesn't have a Name component.
    pub fn rename_entity(&mut self, entity: hecs::Entity, new_name: impl Into<String>) -> Option<()> {
        // Remove old name from index
        let old_name = self.ecs.get::<&Name>(entity).ok().map(|n| n.0.clone())?;
        self.name_index.remove(&old_name);

        let new_name = new_name.into();

        // Update the component
        if let Ok(mut name) = self.ecs.get::<&mut Name>(entity) {
            name.0 = new_name.clone();
        }

        // Add new name to index
        if let Some(_old) = self.name_index.insert(new_name.clone(), entity) {
            log::warn!("Name '{}' already exists in world; overwriting index entry", new_name);
        }

        Some(())
    }

    /// Rebuild the name index by scanning all entities with Name components
    ///
    /// Useful after bulk operations via `ecs_mut_unchecked()`.
    pub fn rebuild_name_index(&mut self) {
        self.name_index.clear();
        for (entity, name) in self.ecs.query::<&Name>().iter() {
            self.name_index.insert(name.0.clone(), entity);
        }
    }

    // === Tag Queries ===

    /// Get all entities with a specific tag
    ///
    /// Uses the tag index for O(1) lookup instead of scanning.
    pub fn get_by_tag(&self, tag: &str) -> Vec<hecs::Entity> {
        self.tag_index.get(tag)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
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
        self.tag_index.clear();
        self.roots.clear();
        self.dirty_count = 0;
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
                        let was_clean = dirty.is_empty();
                        transform.position = phys_body.position;
                        *dirty |= DirtyFlags::TRANSFORM;
                        if was_clean {
                            self.dirty_count += 1;
                        }
                    }
                }
            }
        }
    }

    // === Dirty Tracking ===

    /// Check if any entity in the world has dirty flags set
    pub fn has_dirty_entities(&self) -> bool {
        self.dirty_count > 0
    }

    /// Clear dirty flags on all entities
    pub fn clear_all_dirty(&mut self) {
        for (_, dirty) in self.ecs.query_mut::<&mut DirtyFlags>() {
            *dirty = DirtyFlags::NONE;
        }
        self.dirty_count = 0;
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

        // Remove child from roots (it now has a parent)
        self.roots.remove(&child);

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

            // Add back to roots
            self.roots.insert(child);
        }
    }

    /// Get the world-space transform of an entity
    ///
    /// For root entities (no parent), this is just their own local transform.
    /// For children, this composes transforms from root to leaf.
    ///
    /// Returns `None` if the entity does not exist.
    /// Includes a depth limit of 64 to guard against cycles.
    pub fn world_transform(&self, entity: hecs::Entity) -> Option<Transform4D> {
        const MAX_DEPTH: usize = 64;

        let local_transform = *self.ecs.get::<&Transform4D>(entity).ok()?;

        // Build the chain of ancestors from leaf to root
        let mut chain = vec![local_transform];
        let mut current = entity;
        let mut depth = 0;
        while let Ok(parent) = self.ecs.get::<&Parent>(current) {
            depth += 1;
            if depth > MAX_DEPTH {
                log::error!("world_transform: depth limit ({}) exceeded for entity, possible cycle", MAX_DEPTH);
                break;
            }
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
        self.roots.iter().copied().collect()
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

    /// Validate the hierarchy for consistency (debug tool)
    ///
    /// Checks that:
    /// - Every Parent's target entity exists
    /// - Every parent lists the child in its Children component
    /// - No cycles exist (depth limit check)
    ///
    /// Returns a Vec of human-readable issue descriptions. Empty means valid.
    pub fn validate_hierarchy(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check all Parent components
        for (entity, parent) in self.ecs.query::<&Parent>().iter() {
            // Check parent exists
            if !self.ecs.contains(parent.0) {
                issues.push(format!(
                    "Entity {:?} has Parent pointing to non-existent entity {:?}",
                    entity, parent.0
                ));
                continue;
            }

            // Check parent has this entity in its Children
            match self.ecs.get::<&Children>(parent.0) {
                Ok(children) => {
                    if !children.contains(entity) {
                        issues.push(format!(
                            "Entity {:?} has Parent {:?}, but parent's Children does not contain it",
                            entity, parent.0
                        ));
                    }
                }
                Err(_) => {
                    issues.push(format!(
                        "Entity {:?} has Parent {:?}, but parent has no Children component",
                        entity, parent.0
                    ));
                }
            }
        }


        // Check all Children components for stale/non-existent references
        for (entity, children) in self.ecs.query::<&Children>().iter() {
            for &child in &children.0 {
                if !self.ecs.contains(child) {
                    issues.push(format!(
                        "Entity {:?} has Children listing non-existent entity {:?}",
                        entity, child
                    ));
                    continue;
                }

                // Check child's Parent points back to this entity
                match self.ecs.get::<&Parent>(child) {
                    Ok(parent) => {
                        if parent.0 != entity {
                            issues.push(format!(
                                "Entity {:?} lists {:?} as child, but child's Parent is {:?}",
                                entity, child, parent.0
                            ));
                        }
                    }
                    Err(_) => {
                        issues.push(format!(
                            "Entity {:?} lists {:?} as child, but child has no Parent component",
                            entity, child
                        ));
                    }
                }
            }
        }

        // Check for cycles by walking up from each entity with a depth limit
        for (entity, _) in self.ecs.query::<&Parent>().iter() {
            let mut current = entity;
            let mut depth = 0;
            while let Ok(p) = self.ecs.get::<&Parent>(current) {
                depth += 1;
                if depth > 64 {
                    issues.push(format!(
                        "Possible cycle detected starting from entity {:?} (depth exceeded 64)",
                        entity
                    ));
                    break;
                }
                current = p.0;
            }
        }

        issues
    }

    /// Validate component schema consistency across entities
    ///
    /// Checks for common misconfigurations:
    /// - Entity with PhysicsBody but no Transform4D
    /// - Entity with Children listing entities that lack a Parent component
    ///   (also covered by validate_hierarchy, but this gives a schema-level view)
    /// - Entity with Parent but no Transform4D (unusual for hierarchy nodes)
    ///
    /// Returns a Vec of human-readable warning strings. Empty means no issues found.
    pub fn validate_component_schemas(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // PhysicsBody requires Transform4D
        for (entity, _body) in self.ecs.query::<&PhysicsBody>().iter() {
            if self.ecs.get::<&Transform4D>(entity).is_err() {
                let name = self.ecs.get::<&Name>(entity)
                    .map(|n| format!(" (\"{}\")", n.0))
                    .unwrap_or_default();
                warnings.push(format!(
                    "Entity {:?}{} has PhysicsBody but no Transform4D",
                    entity, name
                ));
            }
        }

        // Children with entries that lack Parent component
        for (entity, children) in self.ecs.query::<&Children>().iter() {
            for &child in &children.0 {
                if self.ecs.contains(child) && self.ecs.get::<&Parent>(child).is_err() {
                    let name = self.ecs.get::<&Name>(entity)
                        .map(|n| format!(" (\"{}\")", n.0))
                        .unwrap_or_default();
                    warnings.push(format!(
                        "Entity {:?}{} has child {:?} that lacks a Parent component",
                        entity, name, child
                    ));
                }
            }
        }

        // Parent component but no Transform4D (hierarchy nodes usually need transforms)
        for (entity, _parent) in self.ecs.query::<&Parent>().iter() {
            if self.ecs.get::<&Transform4D>(entity).is_err() {
                let name = self.ecs.get::<&Name>(entity)
                    .map(|n| format!(" (\"{}\")", n.0))
                    .unwrap_or_default();
                warnings.push(format!(
                    "Entity {:?}{} has Parent but no Transform4D (hierarchy node without transform)",
                    entity, name
                ));
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, ShapeRef};
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

    fn spawn_positioned_entity(world: &mut World, x: f32, y: f32, z: f32, w: f32) -> hecs::Entity {
        let tesseract = Tesseract4D::new(2.0);
        world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::from_position(rust4d_math::Vec4::new(x, y, z, w)),
            Material::default(),
            DirtyFlags::ALL,
        ))
    }

    fn spawn_named_entity(world: &mut World, name: &str) -> hecs::Entity {
        let tesseract = Tesseract4D::new(2.0);
        world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            Name(name.to_string()),
        ))
    }

    fn spawn_tagged_entity(world: &mut World, name: &str, tags: &[&str]) -> hecs::Entity {
        let tesseract = Tesseract4D::new(2.0);
        let tag_set: std::collections::HashSet<String> = tags.iter().map(|t| t.to_string()).collect();
        world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            Name(name.to_string()),
            Tags(tag_set),
        ))
    }

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert!(world.is_empty());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_world_spawn_entity() {
        let mut world = World::new();
        let key = spawn_test_entity(&mut world);

        // Key should be valid
        assert!(world.contains(key));
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_world_get_entity() {
        let mut world = World::new();
        let handle = spawn_test_entity(&mut world);

        let shape = world.ecs().get::<&ShapeRef>(handle);
        assert!(shape.is_ok());
        assert_eq!(shape.unwrap().as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_world_get_entity_mut() {
        let mut world = World::new();
        let handle = spawn_test_entity(&mut world);

        {
            let mut material = world.ecs_mut_unchecked().get::<&mut Material>(handle).unwrap();
            *material = Material::RED;
        }

        let material = world.ecs().get::<&Material>(handle).unwrap();
        assert_eq!(material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_world_entity_count() {
        let mut world = World::new();
        spawn_test_entity(&mut world);
        spawn_test_entity(&mut world);

        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn test_world_clear() {
        let mut world = World::new();
        spawn_test_entity(&mut world);
        spawn_test_entity(&mut world);

        world.clear();
        assert!(world.is_empty());
    }

    #[test]
    fn test_world_ecs_query() {
        let mut world = World::new();
        spawn_test_entity(&mut world);
        spawn_test_entity(&mut world);

        let count = world.ecs().query::<&Transform4D>().iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_world_update() {
        let mut world = World::new();
        spawn_test_entity(&mut world);

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
        let key = spawn_test_entity(&mut world);

        // Key is valid initially
        assert!(world.contains(key));

        // Remove the entity
        let removed = world.despawn(key);
        assert!(removed);

        // Key is now stale
        assert!(!world.contains(key));

        // Add a new entity
        let new_key = spawn_test_entity(&mut world);

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
        let tesseract = Tesseract4D::new(2.0);
        let entity_handle = world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            PhysicsBody(body_handle),
        ));

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
        let tesseract = Tesseract4D::new(2.0);
        let entity_handle = world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            PhysicsBody(body_handle),
        ));

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
        let tesseract = Tesseract4D::new(2.0);
        let entity_handle = world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::from_position(rust4d_math::Vec4::new(5.0, 5.0, 5.0, 5.0)),
            Material::default(),
            DirtyFlags::ALL,
        ));

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
        let key = spawn_named_entity(&mut world, "tesseract");

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
        spawn_tagged_entity(&mut world, "dyn1", &["dynamic"]);
        spawn_tagged_entity(&mut world, "dyn2", &["dynamic"]);
        spawn_tagged_entity(&mut world, "stat1", &["static"]);

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
        let key = spawn_named_entity(&mut world, "tesseract");

        // Should be able to find by name
        assert!(world.get_by_name("tesseract").is_some());

        // Remove the entity
        world.despawn(key);

        // Name should no longer be in the index
        assert!(world.get_by_name("tesseract").is_none());
    }

    #[test]
    fn test_name_index_cleanup_on_clear() {
        let mut world = World::new();

        // Add named entities
        spawn_named_entity(&mut world, "entity1");
        spawn_named_entity(&mut world, "entity2");

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
        let key = spawn_test_entity(&mut world);

        // Entity should exist but not be findable by any name
        assert!(world.contains(key));
        assert!(world.get_by_name("").is_none());
    }

    // --- Dirty tracking tests ---

    #[test]
    fn test_new_entities_are_dirty() {
        let mut world = World::new();
        let key = spawn_test_entity(&mut world);

        // New entities should be dirty (DirtyFlags::ALL)
        let dirty = world.ecs().get::<&DirtyFlags>(key).unwrap();
        assert!(!dirty.is_empty());
        assert!(world.has_dirty_entities());
    }

    #[test]
    fn test_clear_all_dirty() {
        let mut world = World::new();
        spawn_test_entity(&mut world);
        spawn_test_entity(&mut world);

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
        let key1 = spawn_test_entity(&mut world);
        let _key2 = spawn_test_entity(&mut world);

        // Clear dirty flags
        world.clear_all_dirty();

        // Manually mark one as dirty
        {
            let mut dirty = world.ecs_mut_unchecked().get::<&mut DirtyFlags>(key1).unwrap();
            *dirty |= DirtyFlags::TRANSFORM;
        }
        // Note: dirty_count won't track this since we used ecs_mut_unchecked

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
        let tesseract = Tesseract4D::new(2.0);
        let entity_handle = world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            PhysicsBody(body_handle),
        ));

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
        let tesseract = Tesseract4D::new(2.0);
        let entity_handle = world.spawn((
            ShapeRef::shared(tesseract),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            PhysicsBody(body_handle),
        ));

        // Clear dirty flags
        world.clear_all_dirty();

        // Step physics - no movement should occur
        world.update(1.0);

        // Entity should NOT be dirty (position didn't change)
        let dirty = world.ecs().get::<&DirtyFlags>(entity_handle).unwrap();
        assert!(dirty.is_empty());
    }

    // --- Hierarchy tests ---

    #[test]
    fn test_add_child() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

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
        let parent = spawn_test_entity(&mut world);

        // Create an invalid entity by adding and removing
        let temp = spawn_test_entity(&mut world);
        world.despawn(temp);

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
        let a = spawn_test_entity(&mut world);
        let b = spawn_test_entity(&mut world);

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
        let a = spawn_test_entity(&mut world);
        let b = spawn_test_entity(&mut world);
        let c = spawn_test_entity(&mut world);

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
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

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
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

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
        let key = spawn_positioned_entity(&mut world, 1.0, 2.0, 3.0, 4.0);

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
        let parent = spawn_positioned_entity(&mut world, 10.0, 0.0, 0.0, 0.0);
        // Child at (1, 2, 0, 0) in local space
        let child = spawn_positioned_entity(&mut world, 1.0, 2.0, 0.0, 0.0);

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
        let tesseract = Tesseract4D::new(2.0);
        let mut parent_transform = Transform4D::from_position(rust4d_math::Vec4::new(0.0, 0.0, 0.0, 0.0));
        parent_transform.scale = 2.0;
        let parent = world.spawn((
            ShapeRef::shared(tesseract),
            parent_transform,
            Material::default(),
            DirtyFlags::ALL,
        ));

        // Child at (1, 0, 0, 0) in local space
        let child = spawn_positioned_entity(&mut world, 1.0, 0.0, 0.0, 0.0);

        world.add_child(parent, child).unwrap();

        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 2.0).abs() < 0.001,
            "Expected x=2.0, got {}", wt.position.x);
    }

    #[test]
    fn test_delete_recursive() {
        let mut world = World::new();
        let root = spawn_named_entity(&mut world, "root");
        let child1 = spawn_named_entity(&mut world, "child1");
        let child2 = spawn_named_entity(&mut world, "child2");
        let grandchild = spawn_named_entity(&mut world, "grandchild");

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
        let root = spawn_test_entity(&mut world);
        let child1 = spawn_test_entity(&mut world);
        let child2 = spawn_test_entity(&mut world);
        let grandchild = spawn_test_entity(&mut world);

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
        let root = spawn_test_entity(&mut world);
        let child1 = spawn_test_entity(&mut world);
        let child2 = spawn_test_entity(&mut world);
        let grandchild = spawn_test_entity(&mut world);

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
        let root1 = spawn_test_entity(&mut world);
        let root2 = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

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
        let a = spawn_test_entity(&mut world);
        let b = spawn_test_entity(&mut world);
        let c = spawn_test_entity(&mut world);
        let d = spawn_test_entity(&mut world);

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
    fn test_despawn_cleans_hierarchy() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);
        let grandchild = spawn_test_entity(&mut world);

        world.add_child(parent, child).unwrap();
        world.add_child(child, grandchild).unwrap();

        // Remove child (middle of hierarchy)
        world.despawn(child);

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
        let parent1 = spawn_test_entity(&mut world);
        let parent2 = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

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
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

        world.add_child(parent, child).unwrap();
        world.clear();

        assert!(world.is_empty());
    }

    #[test]
    fn test_world_transform_deep_hierarchy() {
        let mut world = World::new();

        // Grandparent at (10, 0, 0, 0)
        let grandparent = spawn_positioned_entity(&mut world, 10.0, 0.0, 0.0, 0.0);
        // Parent at (5, 0, 0, 0) local
        let parent = spawn_positioned_entity(&mut world, 5.0, 0.0, 0.0, 0.0);
        // Child at (1, 0, 0, 0) local
        let child = spawn_positioned_entity(&mut world, 1.0, 0.0, 0.0, 0.0);

        world.add_child(grandparent, parent).unwrap();
        world.add_child(parent, child).unwrap();

        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 16.0).abs() < 0.001,
            "Expected x=16.0, got {}", wt.position.x);
    }

    #[test]
    fn test_world_transform_nonexistent() {
        let mut world = World::new();
        let key = spawn_test_entity(&mut world);
        world.despawn(key);

        // Non-existent entity returns None
        assert!(world.world_transform(key).is_none());
    }

    // --- Rename tests ---

    #[test]
    fn test_rename_entity() {
        let mut world = World::new();
        let key = spawn_named_entity(&mut world, "old_name");

        assert!(world.get_by_name("old_name").is_some());
        assert!(world.get_by_name("new_name").is_none());

        let result = world.rename_entity(key, "new_name");
        assert!(result.is_some());

        assert!(world.get_by_name("old_name").is_none());
        assert!(world.get_by_name("new_name").is_some());
        assert_eq!(world.get_by_name("new_name").unwrap(), key);
    }

    #[test]
    fn test_rename_entity_no_name() {
        let mut world = World::new();
        let key = spawn_test_entity(&mut world);

        // Should return None for entity without Name
        assert!(world.rename_entity(key, "new_name").is_none());
    }

    // --- Rebuild index tests ---

    #[test]
    fn test_rebuild_name_index() {
        let mut world = World::new();
        let key = spawn_named_entity(&mut world, "test_entity");

        // Manually corrupt the index
        world.name_index.clear();
        assert!(world.get_by_name("test_entity").is_none());

        // Rebuild should fix it
        world.rebuild_name_index();
        assert!(world.get_by_name("test_entity").is_some());
        assert_eq!(world.get_by_name("test_entity").unwrap(), key);
    }

    // --- Validate hierarchy tests ---

    #[test]
    fn test_validate_hierarchy_clean() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);
        world.add_child(parent, child).unwrap();

        let issues = world.validate_hierarchy();
        assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
    }

    // --- Tag index tests ---

    #[test]
    fn test_tag_index_cleanup_on_despawn() {
        let mut world = World::new();
        let key = spawn_tagged_entity(&mut world, "tagged", &["enemy", "dynamic"]);

        assert_eq!(world.get_by_tag("enemy").len(), 1);
        assert_eq!(world.get_by_tag("dynamic").len(), 1);

        world.despawn(key);

        assert_eq!(world.get_by_tag("enemy").len(), 0);
        assert_eq!(world.get_by_tag("dynamic").len(), 0);
    }

    // --- Root tracking tests ---

    #[test]
    fn test_roots_tracking() {
        let mut world = World::new();
        let a = spawn_test_entity(&mut world);
        let b = spawn_test_entity(&mut world);
        let c = spawn_test_entity(&mut world);

        // All three should be roots
        assert_eq!(world.root_entities().len(), 3);

        // Make c a child of a
        world.add_child(a, c).unwrap();

        // Now only a and b should be roots
        let roots = world.root_entities();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&a));
        assert!(roots.contains(&b));
        assert!(!roots.contains(&c));

        // Remove c from parent -> becomes root again
        world.remove_from_parent(c);
        let roots = world.root_entities();
        assert_eq!(roots.len(), 3);
        assert!(roots.contains(&c));
    }

    // --- Dirty count tests ---

    #[test]
    fn test_dirty_count() {
        let mut world = World::new();
        spawn_test_entity(&mut world);
        spawn_test_entity(&mut world);

        // Both spawned with DirtyFlags::ALL
        assert!(world.has_dirty_entities());

        world.clear_all_dirty();
        assert!(!world.has_dirty_entities());
    }

    // --- Priority 4: Name collision tests ---

    #[test]
    fn test_name_collision_overwrites_index() {
        let mut world = World::new();
        let e1 = spawn_named_entity(&mut world, "cube");
        let e2 = spawn_named_entity(&mut world, "cube");

        // Second entity should win in the name index
        assert_eq!(world.get_by_name("cube"), Some(e2));
        // e1 still exists in the world, just unreachable by name
        assert!(world.ecs().contains(e1));
        assert!(world.ecs().contains(e2));
    }

    #[test]
    fn test_name_collision_first_entity_still_valid() {
        let mut world = World::new();
        let e1 = spawn_named_entity(&mut world, "cube");
        let _e2 = spawn_named_entity(&mut world, "cube");

        // e1 still has its Name component
        let name = world.ecs().get::<&Name>(e1).unwrap();
        assert_eq!(name.0, "cube");
    }

    // --- ecs_mut_unchecked breaking name index ---

    #[test]
    fn test_ecs_mut_unchecked_breaks_name_index() {
        let mut world = World::new();
        let e1 = spawn_named_entity(&mut world, "original");

        // Mutate via raw access -- bypasses index
        world.ecs_mut_unchecked().get::<&mut Name>(e1).unwrap().0 = "changed".to_string();

        // Index is now stale
        assert_eq!(world.get_by_name("original"), Some(e1)); // stale entry
        assert_eq!(world.get_by_name("changed"), None); // new name not indexed

        // rebuild_name_index fixes it
        world.rebuild_name_index();
        assert_eq!(world.get_by_name("original"), None);
        assert_eq!(world.get_by_name("changed"), Some(e1));
    }

    // --- world_transform on entity without Transform4D ---

    #[test]
    fn test_world_transform_without_transform4d() {
        let mut world = World::new();
        // Spawn entity with only DirtyFlags, no Transform4D
        let e = world.spawn((DirtyFlags::ALL,));
        assert!(world.world_transform(e).is_none());
    }

    // --- world_transform cycle guard ---

    #[test]
    fn test_world_transform_cycle_guard() {
        let mut world = World::new();
        let e1 = spawn_test_entity(&mut world);
        let e2 = spawn_test_entity(&mut world);

        // Create cycle by directly setting Parent components (bypassing add_child)
        let _ = world.ecs_mut_unchecked().insert_one(e1, Parent(e2));
        let _ = world.ecs_mut_unchecked().insert_one(e2, Parent(e1));

        // world_transform should not infinite-loop; it has a depth limit
        let result = world.world_transform(e1);
        // It returns Some because both have Transform4D, just hits depth limit
        assert!(result.is_some());
    }

    // --- Physics cleanup verification ---

    #[test]
    fn test_despawn_cleans_up_physics_body() {
        use rust4d_physics::{PhysicsMaterial, BodyType};

        let config = crate::PhysicsConfig::new(-9.81);
        let mut world = World::new().with_physics(config);

        // Add a physics body
        let body_key = {
            let physics = world.physics_mut().unwrap();
            let body = crate::RigidBody4D::new_aabb(
                rust4d_math::Vec4::new(0.0, 5.0, 0.0, 0.0),
                rust4d_math::Vec4::new(0.5, 0.5, 0.5, 0.5),
            ).with_body_type(BodyType::Dynamic)
             .with_mass(1.0)
             .with_material(PhysicsMaterial::RUBBER);
            physics.add_body(body)
        };

        // Spawn entity with PhysicsBody component
        let e = world.spawn((
            ShapeRef::shared(Tesseract4D::new(1.0)),
            Transform4D::identity(),
            Material::default(),
            DirtyFlags::ALL,
            PhysicsBody(body_key),
        ));

        // Verify physics body exists
        assert!(world.physics().unwrap().get_body(body_key).is_some());

        // Despawn should clean up the physics body
        world.despawn(e);
        assert!(world.physics().unwrap().get_body(body_key).is_none());
    }

    // --- Validate hierarchy with corrupted state ---

    #[test]
    fn test_validate_hierarchy_detects_orphaned_parent() {
        let mut world = World::new();
        let child = spawn_test_entity(&mut world);

        // Manually set Parent to nonexistent entity (bypassing add_child)
        let fake_parent = hecs::Entity::DANGLING;
        let _ = world.ecs_mut_unchecked().insert_one(child, Parent(fake_parent));

        let issues = world.validate_hierarchy();
        assert!(!issues.is_empty(), "Should detect parent pointing to nonexistent entity");
    }

    #[test]
    fn test_validate_hierarchy_detects_missing_child_in_parent() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

        // Set Parent on child but don't add to parent's Children
        let _ = world.ecs_mut_unchecked().insert_one(child, Parent(parent));

        let issues = world.validate_hierarchy();
        assert!(!issues.is_empty(), "Should detect child not listed in parent's Children");
    }

    #[test]
    fn test_validate_hierarchy_detects_stale_child() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);

        // Create a child, add to parent's Children, then despawn the child
        let child = spawn_test_entity(&mut world);
        world.add_child(parent, child).unwrap();
        // Directly remove child from ECS to simulate stale reference
        // We need to bypass normal despawn to leave the Children entry intact
        world.ecs_mut_unchecked().despawn(child).unwrap();

        let issues = world.validate_hierarchy();
        assert!(
            issues.iter().any(|i| i.contains("non-existent")),
            "Should detect stale child reference; got: {:?}", issues
        );
    }

    #[test]
    fn test_validate_hierarchy_child_parent_mismatch() {
        let mut world = World::new();
        let parent_a = spawn_test_entity(&mut world);
        let parent_b = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

        // Set up child under parent_a properly
        world.add_child(parent_a, child).unwrap();

        // Now manually add child to parent_b's Children without updating child's Parent
        let _ = world.ecs_mut_unchecked().insert_one(parent_b, Children(vec![child]));

        let issues = world.validate_hierarchy();
        assert!(
            issues.iter().any(|i| i.contains("child's Parent")),
            "Should detect child-parent mismatch; got: {:?}", issues
        );
    }

    #[test]
    fn test_validate_component_schemas_physics_no_transform() {
        let mut world = World::new();
        // Spawn entity with only PhysicsBody, no Transform4D
        let _entity = world.ecs_mut_unchecked().spawn((
            PhysicsBody(crate::BodyKey::default()),
        ));

        let warnings = world.validate_component_schemas();
        assert!(
            warnings.iter().any(|w| w.contains("PhysicsBody but no Transform4D")),
            "Should detect PhysicsBody without Transform4D; got: {:?}", warnings
        );
    }

    #[test]
    fn test_validate_component_schemas_child_missing_parent() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);

        // Manually add child to parent's Children without setting Parent on child
        let _ = world.ecs_mut_unchecked().insert_one(parent, Children(vec![child]));

        let warnings = world.validate_component_schemas();
        assert!(
            warnings.iter().any(|w| w.contains("lacks a Parent component")),
            "Should detect child without Parent; got: {:?}", warnings
        );
    }

    #[test]
    fn test_validate_component_schemas_clean() {
        let mut world = World::new();
        let parent = spawn_test_entity(&mut world);
        let child = spawn_test_entity(&mut world);
        world.add_child(parent, child).unwrap();

        let warnings = world.validate_component_schemas();
        assert!(
            warnings.is_empty(),
            "Valid world should have no schema warnings; got: {:?}", warnings
        );
    }

}
