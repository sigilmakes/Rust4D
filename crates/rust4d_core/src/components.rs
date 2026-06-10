//! ECS component types
//!
//! Each component represents a single aspect of an entity.
//! Components are stored in hecs and queried individually or in combination.
//!
//! Existing types like `Transform4D`, `Material`, `ShapeRef`, and `DirtyFlags`
//! are also used as ECS components directly (they satisfy Send + Sync + 'static).

use rust4d_physics::BodyKey;
use std::collections::HashSet;

// === Name Component ===

/// Entity name for lookup by name
///
/// Only add this component to entities that need to be found by name.
/// The World wrapper maintains a side-table index for O(1) name lookups.
#[derive(Clone, Debug)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// === Tags Component ===

/// Tags for categorization and filtering
///
/// Tags like "dynamic", "static", "enemy" allow querying entities by category.
#[derive(Clone, Debug, Default)]
pub struct Tags(pub HashSet<String>);

impl Tags {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.0.insert(tag.into());
        self
    }

    pub fn has(&self, tag: &str) -> bool {
        self.0.contains(tag)
    }

    pub fn insert(&mut self, tag: impl Into<String>) {
        self.0.insert(tag.into());
    }
}

impl<I, S> From<I> for Tags
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    fn from(iter: I) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

// === PhysicsBody Component ===

/// Links an entity to a rigid body in the PhysicsWorld
///
/// The BodyKey references a body in the PhysicsWorld side-table.
/// When an entity with this component is despawned, the World wrapper
/// must clean up the corresponding physics body.
#[derive(Clone, Copy, Debug)]
pub struct PhysicsBody(pub BodyKey);

// === Hierarchy Components ===

/// Parent relationship -- points to this entity's parent
#[derive(Clone, Copy, Debug)]
pub struct Parent(pub hecs::Entity);

/// Children relationship -- lists this entity's children
#[derive(Clone, Debug, Default)]
pub struct Children(pub Vec<hecs::Entity>);

impl Children {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, child: hecs::Entity) {
        self.0.push(child);
    }

    pub fn remove(&mut self, child: hecs::Entity) {
        if let Some(pos) = self.0.iter().position(|&c| c == child) {
            self.0.swap_remove(pos);
        }
    }

    pub fn contains(&self, child: hecs::Entity) -> bool {
        self.0.contains(&child)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
