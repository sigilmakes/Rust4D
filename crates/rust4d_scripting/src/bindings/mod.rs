//! Lua bindings for engine subsystems
//!
//! This module provides the bridge between Lua scripts and the Rust engine.
//!
//! ## Available Bindings
//!
//! - `ecs` - Entity-Component-System access (spawn, query, despawn)
//! - `math` - 4D math types (Vec4, Rotor4, Transform4D)
//! - `physics` - Physics queries (raycast, sphere query, line of sight)
//! - `input` - Input polling (keyboard, mouse, actions)
//! - `audio` - 4D spatial audio playback
//! - `hud` - HUD/GUI overlay drawing
//!
//! ## Configuration Types
//!
//! - [`ScreenConfig`] - Set via `lua.set_app_data()` to provide screen dimensions

pub mod audio;
pub mod ecs;
pub mod hud;
pub mod input;
pub mod math;
pub mod physics;

// Re-export configuration types for easy access
pub use hud::ScreenConfig;

use mlua::{Lua, Result as LuaResult};

/// Register all engine bindings with the Lua VM
///
/// This should be called after creating the VM but before loading game scripts.
/// Registers the following globals:
///
/// - `world` - ECS access table
/// - `Vec4`, `Rotor4`, `Transform4D` - Math types
/// - `physics` - Physics queries table
/// - `input` - Input polling table
/// - `audio` - Audio playback table
/// - `hud` - HUD drawing table
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    ecs::register(lua)?;
    math::register(lua)?;
    physics::register(lua)?;
    input::register(lua)?;
    audio::register(lua)?;
    hud::register(lua)?;
    Ok(())
}
