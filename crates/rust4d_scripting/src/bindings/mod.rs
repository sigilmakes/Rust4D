//! Lua bindings for engine subsystems
//!
//! This module provides the bridge between Lua scripts and the Rust engine.
//! ECS bindings are provided by the `ecs` submodule.

pub mod ecs;

use mlua::{Lua, Result as LuaResult};

/// Register all engine bindings with the Lua VM
///
/// This should be called after creating the VM but before loading game scripts.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    ecs::register(lua)?;
    Ok(())
}
