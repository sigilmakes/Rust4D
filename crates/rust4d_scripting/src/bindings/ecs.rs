//! ECS bindings for Lua
//!
//! Provides Lua access to the Entity-Component-System via:
//! - `world.spawn()` - Create entities
//! - `world.query()` - Query entities by component
//! - `world.find_by_name()` - Find entity by name
//! - `world.despawn()` - Remove entities
//! - `entity:get()` / `entity:set()` - Access components
//! - `entity:id()` / `entity:to_bits()` - Entity metadata
//!
//! ## Design Note
//!
//! The ECS `World` lives in the engine, not in Lua. This implementation provides
//! the binding API structure with stub operations that log what would happen.
//! The full integration with the engine's `hecs::World` happens when the engine
//! binary wires up the scripting system.
//!
//! This module is owned by Scripting-ECS-Agent.

use mlua::prelude::*;
use rust4d_core::hecs::Entity;

/// Lua-side entity handle wrapping a hecs::Entity
///
/// Provides methods for entity introspection and component access.
/// Note: Component get/set operations are currently stubs pending
/// engine integration with the actual hecs::World.
#[derive(Clone, Copy, Debug)]
pub struct LuaEntity(pub Entity);

impl LuaUserData for LuaEntity {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // entity:id() -> u64
        // Returns the entity's unique ID within its generation
        methods.add_method("id", |_, this, ()| Ok(this.0.id() as u64));

        // entity:to_bits() -> u64
        // Returns a unique 64-bit identifier that encodes both ID and generation.
        // Useful for storing entity references externally or serialization.
        methods.add_method("to_bits", |_, this, ()| Ok(this.0.to_bits().get()));

        // entity:get(component_name) -> table or nil
        // Stub: Returns nil. Real implementation needs World access.
        methods.add_method("get", |_, this, component: String| {
            log::debug!("[ecs] entity:get({}) for {:?}", component, this.0);
            Ok(Option::<LuaTable>::None)
        });

        // entity:set(component_name, value)
        // Stub: No-op. Real implementation needs World access.
        methods.add_method("set", |_, this, (component, _value): (String, LuaValue)| {
            log::debug!("[ecs] entity:set({}) for {:?}", component, this.0);
            Ok(())
        });

        // entity:is_alive() -> bool
        // Stub: Always returns true. Real implementation needs World access.
        methods.add_method("is_alive", |_, this, ()| {
            log::debug!("[ecs] entity:is_alive() for {:?}", this.0);
            Ok(true)
        });
    }
}

/// Register ECS bindings with the Lua VM
///
/// Creates a global `world` table with the following functions:
/// - `world.spawn(components)` - Spawn an entity with components
/// - `world.query(component_name)` - Query entities by component
/// - `world.find_by_name(name)` - Find entity by name
/// - `world.despawn(entity)` - Despawn an entity
///
/// # Example (Lua)
///
/// ```lua
/// local entity = world.spawn({ transform = { x = 0, y = 1, z = 0, w = 0 } })
/// print(entity:id())
///
/// for e in world.query("transform") do
///     local t = e:get("transform")
/// end
///
/// world.despawn(entity)
/// ```
pub fn register(lua: &Lua) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    // world.spawn(components) -> LuaEntity
    // Creates a new entity with the given components.
    // Stub: Returns a DANGLING entity and logs the spawn request.
    world_table.set(
        "spawn",
        lua.create_function(|_, components: LuaTable| {
            let count = components.len().unwrap_or(0);
            log::debug!("[ecs] spawn called with {} components", count);

            // Log component names for debugging
            for pair in components.pairs::<String, LuaValue>() {
                if let Ok((key, _)) = pair {
                    log::debug!("[ecs]   component: {}", key);
                }
            }

            // Return a DANGLING entity for now
            // Real implementation would create entity in hecs::World
            Ok(LuaEntity(Entity::DANGLING))
        })?,
    )?;

    // world.query(component_name) -> iterator function
    // Returns an iterator over entities with the given component.
    // Stub: Returns an empty iterator.
    world_table.set(
        "query",
        lua.create_function(|lua, component: String| {
            log::debug!("[ecs] query called for component: {}", component);

            // Return an empty iterator function
            // Real implementation would iterate over hecs::World query results
            let empty_iter = lua.create_function(|_, ()| -> LuaResult<Option<LuaEntity>> {
                Ok(None)
            })?;
            Ok(empty_iter)
        })?,
    )?;

    // world.find_by_name(name) -> LuaEntity or nil
    // Finds an entity by its "name" component.
    // Stub: Always returns nil.
    world_table.set(
        "find_by_name",
        lua.create_function(|_, name: String| {
            log::debug!("[ecs] find_by_name called: {}", name);
            // Real implementation would query World for entity with matching Name component
            Ok(Option::<LuaEntity>::None)
        })?,
    )?;

    // world.despawn(entity)
    // Removes an entity from the world.
    // Stub: No-op, just logs.
    world_table.set(
        "despawn",
        lua.create_function(|_, entity: LuaAnyUserData| {
            if let Ok(lua_entity) = entity.borrow::<LuaEntity>() {
                log::debug!("[ecs] despawn called for entity {:?}", lua_entity.0);
            }
            Ok(())
        })?,
    )?;

    // world.entity_count() -> u64
    // Returns the number of entities in the world.
    // Stub: Always returns 0.
    world_table.set(
        "entity_count",
        lua.create_function(|_, ()| {
            log::debug!("[ecs] entity_count called");
            Ok(0u64)
        })?,
    )?;

    // Register the world table as a global
    lua.globals().set("world", world_table)?;

    log::debug!("[ecs] ECS bindings registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_lua_with_ecs() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register ECS bindings");
        lua
    }

    #[test]
    fn test_world_table_exists() {
        let lua = create_lua_with_ecs();
        let world: LuaTable = lua.globals().get("world").expect("world table should exist");
        assert!(world.contains_key("spawn").unwrap());
        assert!(world.contains_key("query").unwrap());
        assert!(world.contains_key("find_by_name").unwrap());
        assert!(world.contains_key("despawn").unwrap());
        assert!(world.contains_key("entity_count").unwrap());
    }

    #[test]
    fn test_spawn_returns_entity() {
        let lua = create_lua_with_ecs();
        let result: LuaResult<LuaAnyUserData> = lua
            .load(
                r#"
            return world.spawn({ name = "test" })
        "#,
            )
            .eval();
        assert!(result.is_ok(), "spawn should return a LuaEntity userdata");
    }

    #[test]
    fn test_spawn_with_multiple_components() {
        let lua = create_lua_with_ecs();
        let result: LuaResult<LuaAnyUserData> = lua
            .load(
                r#"
            return world.spawn({
                transform = { x = 1, y = 2, z = 3, w = 4 },
                name = "player",
                health = 100
            })
        "#,
            )
            .eval();
        assert!(result.is_ok());
    }

    #[test]
    fn test_entity_has_id_method() {
        let lua = create_lua_with_ecs();
        let id: u64 = lua
            .load(
                r#"
            local e = world.spawn({})
            return e:id()
        "#,
            )
            .eval()
            .expect("id() should return a number");
        // DANGLING entity has id u32::MAX
        assert_eq!(id, u32::MAX as u64);
    }

    #[test]
    fn test_entity_has_to_bits_method() {
        let lua = create_lua_with_ecs();
        // Lua numbers are f64, so large u64 values lose precision.
        // Just verify it returns a number and doesn't error.
        let bits: f64 = lua
            .load(
                r#"
            local e = world.spawn({})
            return e:to_bits()
        "#,
            )
            .eval()
            .expect("to_bits() should return a number");
        // DANGLING entity has specific bits encoding (very large number)
        assert!(bits > 0.0, "to_bits() should return non-zero for DANGLING entity");
    }

    #[test]
    fn test_entity_get_returns_nil() {
        let lua = create_lua_with_ecs();
        let result: LuaValue = lua
            .load(
                r#"
            local e = world.spawn({})
            return e:get("transform")
        "#,
            )
            .eval()
            .expect("get() should not error");
        assert!(result.is_nil(), "stub get() should return nil");
    }

    #[test]
    fn test_entity_set_does_not_error() {
        let lua = create_lua_with_ecs();
        let result: LuaResult<()> = lua
            .load(
                r#"
            local e = world.spawn({})
            e:set("transform", { x = 1, y = 2, z = 3, w = 4 })
        "#,
            )
            .eval();
        assert!(result.is_ok(), "set() should not error");
    }

    #[test]
    fn test_entity_is_alive() {
        let lua = create_lua_with_ecs();
        let alive: bool = lua
            .load(
                r#"
            local e = world.spawn({})
            return e:is_alive()
        "#,
            )
            .eval()
            .expect("is_alive() should return a boolean");
        assert!(alive, "stub is_alive() should return true");
    }

    #[test]
    fn test_query_returns_iterator() {
        let lua = create_lua_with_ecs();
        let count: i32 = lua
            .load(
                r#"
            local count = 0
            for entity in world.query("transform") do
                count = count + 1
            end
            return count
        "#,
            )
            .eval()
            .expect("query iteration should work");
        assert_eq!(count, 0, "empty iterator should return nothing");
    }

    #[test]
    fn test_find_by_name_returns_nil_when_not_found() {
        let lua = create_lua_with_ecs();
        let result: LuaValue = lua
            .load(
                r#"
            return world.find_by_name("nonexistent")
        "#,
            )
            .eval()
            .expect("find_by_name should not error");
        assert!(result.is_nil(), "stub find_by_name should return nil");
    }

    #[test]
    fn test_despawn_does_not_error() {
        let lua = create_lua_with_ecs();
        let result: LuaResult<()> = lua
            .load(
                r#"
            local e = world.spawn({})
            world.despawn(e)
        "#,
            )
            .eval();
        assert!(result.is_ok(), "despawn should not error");
    }

    #[test]
    fn test_entity_count_returns_zero() {
        let lua = create_lua_with_ecs();
        let count: u64 = lua
            .load(
                r#"
            return world.entity_count()
        "#,
            )
            .eval()
            .expect("entity_count should return a number");
        assert_eq!(count, 0, "stub entity_count should return 0");
    }

    #[test]
    fn test_lua_entity_userdata_debug() {
        // Test that LuaEntity can be created directly and has expected properties
        let entity = LuaEntity(Entity::DANGLING);
        // DANGLING entity has id u32::MAX
        assert_eq!(entity.0.id(), u32::MAX);
        // to_bits returns a non-zero value
        assert!(entity.0.to_bits().get() > 0);
    }
}
