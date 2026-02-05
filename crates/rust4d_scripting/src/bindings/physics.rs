//! Physics bindings for Lua
//!
//! Provides Lua access to physics queries:
//! - `physics.raycast(origin, direction, max_distance, layers)` - Cast a ray
//! - `physics.raycast_nearest(...)` - Get nearest hit only
//! - `physics.query_sphere(center, radius, layers)` - Find entities in a sphere
//! - `physics.line_of_sight(from, to, blocking_layers)` - Check visibility
//! - `physics.query_area_effect(...)` - Find entities with falloff calculation
//!
//! ## Usage (Lua)
//!
//! ```lua
//! -- Raycast from player position downward
//! local origin = player_position
//! local direction = Vec4.new(0, -1, 0, 0)
//! local hits = physics.raycast(origin, direction, 100, {"world", "enemies"})
//!
//! for _, hit in ipairs(hits) do
//!     print("Hit at distance:", hit.distance)
//!     print("Hit point:", hit.point)
//!     print("Hit normal:", hit.normal)
//! end
//!
//! -- Or get just the nearest hit
//! local hit = physics.raycast_nearest(origin, direction, 100, {"world"})
//! if hit then
//!     print("Nearest hit:", hit.distance)
//! end
//!
//! -- Check line of sight
//! if physics.line_of_sight(player_pos, enemy_pos, {"world"}) then
//!     print("Can see enemy!")
//! end
//!
//! -- Find all entities in a radius
//! local nearby = physics.query_sphere(explosion_center, 10, {"enemies", "props"})
//! for _, entry in ipairs(nearby) do
//!     print("Entity at distance:", entry.distance)
//! end
//! ```
//!
//! ## Design Note
//!
//! The physics world lives in the engine, not in Lua. This implementation provides
//! the binding API structure with stub operations that log what would happen.
//! Full integration with the engine's PhysicsWorld happens when the engine binary
//! wires up the scripting system via `lua.set_app_data()`.
//!
//! This module is owned by Agent D2 (Math/Physics Bindings).

use mlua::prelude::*;

use super::math::LuaVec4;

/// Lua representation of a ray hit result
///
/// Contains distance, hit point, surface normal, and optionally the entity hit.
#[derive(Clone, Debug)]
pub struct LuaRayHit {
    /// Distance from ray origin to hit point
    pub distance: f32,
    /// The point where the ray hit
    pub point: LuaVec4,
    /// Surface normal at the hit point
    pub normal: LuaVec4,
    /// Entity ID if we hit a body (as u64 bits), None for static geometry
    pub entity_bits: Option<u64>,
}

impl LuaUserData for LuaRayHit {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("distance", |_, this| Ok(this.distance));
        fields.add_field_method_get("point", |_, this| Ok(this.point));
        fields.add_field_method_get("normal", |_, this| Ok(this.normal));
        fields.add_field_method_get("entity", |_, this| Ok(this.entity_bits));
    }
}

/// Lua representation of a sphere query result
#[derive(Clone, Debug)]
pub struct LuaSphereQueryResult {
    /// Entity ID as bits
    pub entity_bits: u64,
    /// Position of the entity
    pub position: LuaVec4,
    /// Distance from query center to entity
    pub distance: f32,
}

impl LuaUserData for LuaSphereQueryResult {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("entity", |_, this| Ok(this.entity_bits));
        fields.add_field_method_get("position", |_, this| Ok(this.position));
        fields.add_field_method_get("distance", |_, this| Ok(this.distance));
    }
}

/// Lua representation of an area effect query result
#[derive(Clone, Debug)]
pub struct LuaAreaEffectResult {
    /// Entity ID as bits
    pub entity_bits: u64,
    /// Position of the entity
    pub position: LuaVec4,
    /// Distance from query center to entity
    pub distance: f32,
    /// Falloff multiplier (1.0 at center, 0.0 at radius edge)
    pub falloff: f32,
    /// Direction from center to entity (normalized)
    pub direction: LuaVec4,
}

impl LuaUserData for LuaAreaEffectResult {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("entity", |_, this| Ok(this.entity_bits));
        fields.add_field_method_get("position", |_, this| Ok(this.position));
        fields.add_field_method_get("distance", |_, this| Ok(this.distance));
        fields.add_field_method_get("falloff", |_, this| Ok(this.falloff));
        fields.add_field_method_get("direction", |_, this| Ok(this.direction));
    }
}

/// Parse a layer filter table from Lua
///
/// Accepts either a table of layer name strings or nil (all layers).
fn parse_layers(value: LuaValue) -> LuaResult<Vec<String>> {
    match value {
        LuaValue::Nil => Ok(vec![]), // Empty = all layers
        LuaValue::Table(table) => {
            let mut layers = Vec::new();
            for pair in table.sequence_values::<String>() {
                layers.push(pair?);
            }
            Ok(layers)
        }
        _ => Err(LuaError::RuntimeError(
            "layers must be a table of strings or nil".to_string(),
        )),
    }
}

/// Register physics bindings with the Lua VM
///
/// Creates a global `physics` table with the following functions:
/// - `physics.raycast(origin, direction, max_distance, layers)` -> array of hits
/// - `physics.raycast_nearest(origin, direction, max_distance, layers)` -> hit or nil
/// - `physics.query_sphere(center, radius, layers)` -> array of results
/// - `physics.query_area_effect(center, radius, layers, with_falloff)` -> array of results
/// - `physics.line_of_sight(from, to, blocking_layers)` -> boolean
///
/// # Stub Implementation
///
/// These functions are currently stubs that log what would happen but return
/// empty results. Full integration requires the engine to provide a PhysicsWorld
/// via `lua.set_app_data()`.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let physics_table = lua.create_table()?;

    // physics.raycast(origin, direction, max_distance, layers) -> array of hits
    //
    // Cast a ray and return all hits sorted by distance (nearest first).
    //
    // Arguments:
    // - origin: Vec4 - Starting point of the ray
    // - direction: Vec4 - Direction of the ray (will be normalized)
    // - max_distance: number - Maximum distance to check
    // - layers: table or nil - Layer names to check, or nil for all layers
    //
    // Returns:
    // - Array of hit results, each with: distance, point, normal, entity (or nil)
    physics_table.set(
        "raycast",
        lua.create_function(
            |lua, (origin, direction, max_distance, layers): (LuaVec4, LuaVec4, f32, LuaValue)| {
                let layer_names = parse_layers(layers)?;
                log::debug!(
                    "[physics] raycast from ({:.2}, {:.2}, {:.2}, {:.2}) dir ({:.2}, {:.2}, {:.2}, {:.2}) max_dist={:.2} layers={:?}",
                    origin.0.x, origin.0.y, origin.0.z, origin.0.w,
                    direction.0.x, direction.0.y, direction.0.z, direction.0.w,
                    max_distance, layer_names
                );

                // STUB: Return empty array
                // Real implementation would:
                // 1. Get PhysicsWorld from lua.app_data()
                // 2. Create Ray4D from origin and normalized direction
                // 3. Call world.raycast_all(ray, max_distance, layers)
                // 4. Convert hits to LuaRayHit and sort by distance
                let results = lua.create_table()?;
                Ok(results)
            },
        )?,
    )?;

    // physics.raycast_nearest(origin, direction, max_distance, layers) -> hit or nil
    //
    // Cast a ray and return only the nearest hit.
    //
    // Arguments: same as raycast
    //
    // Returns:
    // - Single hit result or nil if nothing was hit
    physics_table.set(
        "raycast_nearest",
        lua.create_function(
            |_, (origin, direction, max_distance, layers): (LuaVec4, LuaVec4, f32, LuaValue)| {
                let layer_names = parse_layers(layers)?;
                log::debug!(
                    "[physics] raycast_nearest from ({:.2}, {:.2}, {:.2}, {:.2}) dir ({:.2}, {:.2}, {:.2}, {:.2}) max_dist={:.2} layers={:?}",
                    origin.0.x, origin.0.y, origin.0.z, origin.0.w,
                    direction.0.x, direction.0.y, direction.0.z, direction.0.w,
                    max_distance, layer_names
                );

                // STUB: Return nil
                // Real implementation would call world.raycast() which returns Option<WorldRayHit>
                Ok(Option::<LuaRayHit>::None)
            },
        )?,
    )?;

    // physics.query_sphere(center, radius, layers) -> array of results
    //
    // Find all entities within a sphere.
    //
    // Arguments:
    // - center: Vec4 - Center of the sphere
    // - radius: number - Radius of the sphere
    // - layers: table or nil - Layer names to check, or nil for all layers
    //
    // Returns:
    // - Array of results, each with: entity, position, distance
    physics_table.set(
        "query_sphere",
        lua.create_function(
            |lua, (center, radius, layers): (LuaVec4, f32, LuaValue)| {
                let layer_names = parse_layers(layers)?;
                log::debug!(
                    "[physics] query_sphere at ({:.2}, {:.2}, {:.2}, {:.2}) radius={:.2} layers={:?}",
                    center.0.x, center.0.y, center.0.z, center.0.w,
                    radius, layer_names
                );

                // STUB: Return empty array
                // Real implementation would:
                // 1. Get PhysicsWorld from lua.app_data()
                // 2. Iterate all bodies and check distance from center
                // 3. Filter by layer
                // 4. Return matches with position and distance
                let results = lua.create_table()?;
                Ok(results)
            },
        )?,
    )?;

    // physics.query_area_effect(center, radius, layers, with_falloff) -> array of results
    //
    // Find all entities within a radius with distance-based falloff calculation.
    // Useful for explosions, area damage, etc.
    //
    // Arguments:
    // - center: Vec4 - Center of the effect
    // - radius: number - Maximum radius of the effect
    // - layers: table or nil - Layer names to check
    // - with_falloff: boolean - If true, calculate falloff (default true)
    //
    // Returns:
    // - Array of results, each with: entity, position, distance, falloff, direction
    physics_table.set(
        "query_area_effect",
        lua.create_function(
            |lua,
             (center, radius, layers, with_falloff): (LuaVec4, f32, LuaValue, Option<bool>)| {
                let layer_names = parse_layers(layers)?;
                let falloff = with_falloff.unwrap_or(true);
                log::debug!(
                    "[physics] query_area_effect at ({:.2}, {:.2}, {:.2}, {:.2}) radius={:.2} falloff={} layers={:?}",
                    center.0.x, center.0.y, center.0.z, center.0.w,
                    radius, falloff, layer_names
                );

                // STUB: Return empty array
                // Real implementation would be similar to query_sphere but include:
                // - falloff = 1.0 - (distance / radius) if with_falloff else 1.0
                // - direction = (entity_pos - center).normalized()
                let results = lua.create_table()?;
                Ok(results)
            },
        )?,
    )?;

    // physics.line_of_sight(from, to, blocking_layers) -> boolean
    //
    // Check if there's a clear line of sight between two points.
    //
    // Arguments:
    // - from: Vec4 - Starting point
    // - to: Vec4 - Target point
    // - blocking_layers: table or nil - Layers that block line of sight
    //
    // Returns:
    // - true if line of sight is clear, false if blocked
    physics_table.set(
        "line_of_sight",
        lua.create_function(|_, (from, to, blocking_layers): (LuaVec4, LuaVec4, LuaValue)| {
            let layer_names = parse_layers(blocking_layers)?;
            log::debug!(
                "[physics] line_of_sight from ({:.2}, {:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2}, {:.2}) blocking={:?}",
                from.0.x, from.0.y, from.0.z, from.0.w,
                to.0.x, to.0.y, to.0.z, to.0.w,
                layer_names
            );

            // STUB: Return true (nothing blocking)
            // Real implementation would:
            // 1. Calculate direction and distance from 'from' to 'to'
            // 2. Cast a ray with that direction and distance
            // 3. Return true if no hits, false if any hit
            Ok(true)
        })?,
    )?;

    // physics.gravity() -> Vec4
    //
    // Get the current gravity vector.
    // Default is (0, -20, 0, 0).
    physics_table.set(
        "gravity",
        lua.create_function(|_, ()| {
            log::debug!("[physics] gravity() called");
            // STUB: Return default gravity
            // Real implementation would get from PhysicsConfig
            Ok(LuaVec4(rust4d_math::Vec4::new(0.0, -20.0, 0.0, 0.0)))
        })?,
    )?;

    // Register the physics table as a global
    lua.globals().set("physics", physics_table)?;

    log::debug!("[physics] Physics bindings registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::math;

    fn create_lua_with_physics() -> Lua {
        let lua = Lua::new();
        math::register(&lua).expect("Failed to register math bindings");
        register(&lua).expect("Failed to register physics bindings");
        lua
    }

    #[test]
    fn test_physics_table_exists() {
        let lua = create_lua_with_physics();
        let physics: LuaTable = lua
            .globals()
            .get("physics")
            .expect("physics table should exist");
        assert!(physics.contains_key("raycast").unwrap());
        assert!(physics.contains_key("raycast_nearest").unwrap());
        assert!(physics.contains_key("query_sphere").unwrap());
        assert!(physics.contains_key("query_area_effect").unwrap());
        assert!(physics.contains_key("line_of_sight").unwrap());
        assert!(physics.contains_key("gravity").unwrap());
    }

    #[test]
    fn test_raycast_returns_table() {
        let lua = create_lua_with_physics();
        lua.load(
            r#"
            local origin = Vec4.new(0, 0, 0, 0)
            local direction = Vec4.new(1, 0, 0, 0)
            local hits = physics.raycast(origin, direction, 100, nil)
            assert(type(hits) == "table", "raycast should return a table")
        "#,
        )
        .exec()
        .expect("raycast should work");
    }

    #[test]
    fn test_raycast_with_layers() {
        let lua = create_lua_with_physics();
        lua.load(
            r#"
            local origin = Vec4.new(0, 0, 0, 0)
            local direction = Vec4.new(0, -1, 0, 0)
            local hits = physics.raycast(origin, direction, 100, {"world", "enemies"})
            assert(type(hits) == "table")
        "#,
        )
        .exec()
        .expect("raycast with layers should work");
    }

    #[test]
    fn test_raycast_nearest_returns_nil() {
        let lua = create_lua_with_physics();
        let result: LuaValue = lua
            .load(
                r#"
            local origin = Vec4.new(0, 0, 0, 0)
            local direction = Vec4.new(1, 0, 0, 0)
            return physics.raycast_nearest(origin, direction, 100, nil)
        "#,
            )
            .eval()
            .expect("raycast_nearest should work");
        assert!(result.is_nil(), "stub should return nil");
    }

    #[test]
    fn test_query_sphere_returns_table() {
        let lua = create_lua_with_physics();
        lua.load(
            r#"
            local center = Vec4.new(0, 0, 0, 0)
            local results = physics.query_sphere(center, 10, nil)
            assert(type(results) == "table")
        "#,
        )
        .exec()
        .expect("query_sphere should work");
    }

    #[test]
    fn test_query_area_effect_returns_table() {
        let lua = create_lua_with_physics();
        lua.load(
            r#"
            local center = Vec4.new(5, 5, 5, 0)
            local results = physics.query_area_effect(center, 15, {"enemies"}, true)
            assert(type(results) == "table")
        "#,
        )
        .exec()
        .expect("query_area_effect should work");
    }

    #[test]
    fn test_query_area_effect_default_falloff() {
        let lua = create_lua_with_physics();
        lua.load(
            r#"
            local center = Vec4.new(0, 0, 0, 0)
            -- with_falloff defaults to true
            local results = physics.query_area_effect(center, 10, nil)
            assert(type(results) == "table")
        "#,
        )
        .exec()
        .expect("query_area_effect should work with default falloff");
    }

    #[test]
    fn test_line_of_sight_returns_bool() {
        let lua = create_lua_with_physics();
        let result: bool = lua
            .load(
                r#"
            local from = Vec4.new(0, 0, 0, 0)
            local to = Vec4.new(10, 0, 0, 0)
            return physics.line_of_sight(from, to, {"world"})
        "#,
            )
            .eval()
            .expect("line_of_sight should work");
        assert!(result, "stub should return true (no blocking)");
    }

    #[test]
    fn test_gravity_returns_vec4() {
        let lua = create_lua_with_physics();
        let y: f32 = lua
            .load(
                r#"
            local g = physics.gravity()
            return g.y
        "#,
            )
            .eval()
            .expect("gravity should work");
        assert!(y < 0.0, "gravity should be negative (downward)");
    }

    #[test]
    fn test_invalid_layers_type_error() {
        let lua = create_lua_with_physics();
        let result: LuaResult<()> = lua
            .load(
                r#"
            local origin = Vec4.new(0, 0, 0, 0)
            local direction = Vec4.new(1, 0, 0, 0)
            physics.raycast(origin, direction, 100, "invalid")
        "#,
            )
            .exec();
        assert!(result.is_err(), "should error on invalid layers type");
    }
}
