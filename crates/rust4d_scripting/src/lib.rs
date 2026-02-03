//! Lua scripting engine for Rust4D
//!
//! This crate provides a Lua 5.4 scripting runtime for the Rust4D engine,
//! powered by mlua. It handles:
//!
//! - Lua VM initialization with sandboxed globals
//! - Script loading from a game directory
//! - Game loop lifecycle callbacks (on_init, on_update, on_fixed_update, on_shutdown)
//! - Error handling and reporting
//!
//! ## Usage
//!
//! ```no_run
//! use rust4d_scripting::{ScriptEngine, ScriptConfig};
//!
//! let config = ScriptConfig {
//!     scripts_dir: "my_game/scripts".to_string(),
//!     ..Default::default()
//! };
//! let mut engine = ScriptEngine::new(config).unwrap();
//! engine.load_game().unwrap();
//! engine.call_init().unwrap();
//!
//! // In the game loop:
//! engine.call_update(0.016).unwrap();
//! engine.call_fixed_update(1.0 / 60.0).unwrap();
//!
//! // On shutdown:
//! engine.call_shutdown().unwrap();
//! ```

pub mod error;
pub mod lifecycle;
pub mod loader;
pub mod vm;

pub use error::ScriptError;
pub use vm::ScriptConfig;

use mlua::prelude::*;

/// The main scripting engine handle.
///
/// Owns a Lua VM and manages script loading and lifecycle callbacks.
pub struct ScriptEngine {
    lua: Lua,
    config: ScriptConfig,
    error_state: Option<ScriptError>,
}

impl ScriptEngine {
    /// Create a new scripting engine with the given configuration.
    ///
    /// This initializes the Lua VM with sandboxed globals and configured
    /// package paths but does not load any scripts.
    pub fn new(config: ScriptConfig) -> Result<Self, ScriptError> {
        let lua = vm::create_lua_vm(&config)?;
        Ok(Self {
            lua,
            config,
            error_state: None,
        })
    }

    /// Load the game's main.lua and all required modules.
    ///
    /// This executes `<scripts_dir>/main.lua`, which can use `require()`
    /// to load additional modules. After this call, all lifecycle callbacks
    /// (on_init, on_update, etc.) should be defined as globals.
    pub fn load_game(&mut self) -> Result<(), ScriptError> {
        loader::load_game_scripts(&self.lua, &self.config.scripts_dir)
    }

    /// Call the `on_init()` lifecycle callback.
    ///
    /// This should be called once after `load_game()` and before the game loop starts.
    /// If `on_init` is not defined, this is a no-op.
    pub fn call_init(&self) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, "on_init", ())
    }

    /// Call the `on_update(dt)` lifecycle callback.
    ///
    /// Called each frame with the variable-timestep delta time.
    /// If `on_update` is not defined, this is a no-op.
    pub fn call_update(&self, dt: f32) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, "on_update", dt)
    }

    /// Call the `on_fixed_update(dt)` lifecycle callback.
    ///
    /// Called at a fixed timestep rate for deterministic game logic.
    /// If `on_fixed_update` is not defined, this is a no-op.
    pub fn call_fixed_update(&self, dt: f32) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, "on_fixed_update", dt)
    }

    /// Call the `on_shutdown()` lifecycle callback.
    ///
    /// Called once before the engine exits. If `on_shutdown` is not defined,
    /// this is a no-op.
    pub fn call_shutdown(&self) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, "on_shutdown", ())
    }

    /// Get the last error encountered by the engine.
    ///
    /// Useful for displaying errors on-screen during development.
    pub fn last_error(&self) -> Option<&ScriptError> {
        self.error_state.as_ref()
    }

    /// Set the error state (for internal use when suppressing repeated errors).
    pub fn set_error(&mut self, err: ScriptError) {
        self.error_state = Some(err);
    }

    /// Clear the error state.
    pub fn clear_error(&mut self) {
        self.error_state = None;
    }

    /// Get a reference to the underlying Lua VM.
    ///
    /// Useful for registering additional bindings or for testing.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get the script configuration.
    pub fn config(&self) -> &ScriptConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_game_dir() -> (tempfile::TempDir, ScriptConfig) {
        let dir = tempfile::tempdir().unwrap();
        let config = ScriptConfig {
            scripts_dir: dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        (dir, config)
    }

    #[test]
    fn test_engine_new() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_load_missing_main() {
        let (_dir, config) = create_game_dir();
        let mut engine = ScriptEngine::new(config).unwrap();
        let result = engine.load_game();
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_full_lifecycle() {
        let (dir, config) = create_game_dir();

        // Write main.lua with all lifecycle callbacks
        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            lifecycle_log = {}

            function on_init()
                table.insert(lifecycle_log, "init")
            end

            function on_update(dt)
                table.insert(lifecycle_log, "update:" .. tostring(dt))
            end

            function on_fixed_update(dt)
                table.insert(lifecycle_log, "fixed:" .. tostring(dt))
            end

            function on_shutdown()
                table.insert(lifecycle_log, "shutdown")
            end
            "#,
        )
        .unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        engine.call_init().unwrap();
        engine.call_update(0.016).unwrap();
        engine.call_fixed_update(1.0 / 60.0).unwrap();
        engine.call_shutdown().unwrap();

        // Verify the lifecycle log
        let count: i64 = engine
            .lua()
            .load("return #lifecycle_log")
            .eval()
            .unwrap();
        assert_eq!(count, 4);

        let first: String = engine
            .lua()
            .load("return lifecycle_log[1]")
            .eval()
            .unwrap();
        assert_eq!(first, "init");

        let last: String = engine
            .lua()
            .load("return lifecycle_log[4]")
            .eval()
            .unwrap();
        assert_eq!(last, "shutdown");
    }

    #[test]
    fn test_engine_missing_callbacks_ok() {
        let (dir, config) = create_game_dir();

        // main.lua with no callbacks defined
        let main_path = dir.path().join("main.lua");
        std::fs::write(&main_path, "-- empty script").unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        assert!(engine.call_init().is_ok());
        assert!(engine.call_update(0.016).is_ok());
        assert!(engine.call_fixed_update(1.0 / 60.0).is_ok());
        assert!(engine.call_shutdown().is_ok());
    }

    #[test]
    fn test_engine_runtime_error_in_callback() {
        let (dir, config) = create_game_dir();

        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            function on_update(dt)
                error("something went wrong")
            end
            "#,
        )
        .unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        let result = engine.call_update(0.016);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_require_modules() {
        let (dir, config) = create_game_dir();

        // Create a module
        let mut module_file =
            std::fs::File::create(dir.path().join("utils.lua")).unwrap();
        writeln!(module_file, "local M = {{}}").unwrap();
        writeln!(module_file, "function M.double(x) return x * 2 end").unwrap();
        writeln!(module_file, "return M").unwrap();

        // main.lua that requires and uses the module
        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            local utils = require("utils")
            result = utils.double(21)
            "#,
        )
        .unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();

        let result: i64 = engine.lua().load("return result").eval().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_engine_error_state() {
        let (_dir, config) = create_game_dir();
        let mut engine = ScriptEngine::new(config).unwrap();

        assert!(engine.last_error().is_none());

        engine.set_error(ScriptError::FileNotFound("test.lua".to_string()));
        assert!(engine.last_error().is_some());

        engine.clear_error();
        assert!(engine.last_error().is_none());
    }

    #[test]
    fn test_engine_update_receives_correct_dt() {
        let (dir, config) = create_game_dir();

        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            function on_update(dt)
                received_dt = dt
            end
            "#,
        )
        .unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        engine.call_update(0.033).unwrap();

        let dt: f64 = engine.lua().load("return received_dt").eval().unwrap();
        assert!((dt - 0.033).abs() < 0.001);
    }

    #[test]
    fn test_sandboxing_prevents_os_access() {
        let (dir, config) = create_game_dir();

        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            function on_init()
                if os then
                    error("os should not be available")
                end
            end
            "#,
        )
        .unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        engine.call_init().unwrap(); // Should not error
    }
}
