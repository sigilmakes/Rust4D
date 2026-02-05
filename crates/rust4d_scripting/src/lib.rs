//! Lua scripting runtime for the Rust4D engine
//!
//! This crate provides a sandboxed Lua 5.4 VM for game scripting. Scripts can:
//! - Define lifecycle callbacks (on_init, on_update, on_fixed_update, on_shutdown)
//! - Access the ECS to spawn entities and manipulate components
//! - Use safe Lua standard library functions (math, string, table, etc.)
//!
//! Dangerous operations (file I/O, OS access, debug library) are removed for security.
//!
//! # Quick Start
//!
//! ```no_run
//! use rust4d_scripting::{ScriptEngine, ScriptConfig};
//!
//! let config = ScriptConfig::with_scripts_dir("scripts");
//! let mut engine = ScriptEngine::new(config).expect("Failed to create script engine");
//!
//! // Load main.lua and any required modules
//! engine.load_game().expect("Failed to load scripts");
//!
//! // Call lifecycle callbacks
//! engine.call_init().ok();
//!
//! // In game loop:
//! // engine.call_update(dt).ok();
//! // engine.call_fixed_update(fixed_dt).ok();
//!
//! // On shutdown:
//! // engine.call_shutdown().ok();
//! ```
//!
//! # Script Structure
//!
//! Game scripts should have a `main.lua` entry point that defines callbacks:
//!
//! ```lua
//! -- main.lua
//! function on_init()
//!     print("Game starting!")
//! end
//!
//! function on_update(dt)
//!     -- Called every frame
//! end
//!
//! function on_fixed_update(dt)
//!     -- Called at fixed intervals (physics)
//! end
//!
//! function on_shutdown()
//!     print("Game ending!")
//! end
//! ```

mod error;
mod vm;
mod loader;
mod lifecycle;
pub mod bindings;

pub use error::ScriptError;
pub use vm::ScriptConfig;
pub use lifecycle::callbacks;

use mlua::Lua;

/// The main script engine that manages the Lua VM and game scripts
pub struct ScriptEngine {
    /// The Lua VM instance
    lua: Lua,
    /// Configuration for the engine
    config: ScriptConfig,
    /// Last error that occurred (for non-fatal error reporting)
    error_state: Option<ScriptError>,
}

impl ScriptEngine {
    /// Create a new script engine with the given configuration
    ///
    /// This creates and sandboxes the Lua VM but does not load any scripts.
    /// Call `load_game()` to load the game's main.lua.
    pub fn new(config: ScriptConfig) -> Result<Self, ScriptError> {
        let lua = vm::create_lua_vm(&config)?;

        // Register engine bindings
        bindings::register_all(&lua).map_err(|e| ScriptError::LuaError {
            message: format!("Failed to register bindings: {}", e),
            source: Some(e),
        })?;

        Ok(Self {
            lua,
            config,
            error_state: None,
        })
    }

    /// Load the game's main script (main.lua)
    ///
    /// This loads and executes main.lua from the configured scripts directory.
    /// The script should define global callback functions.
    pub fn load_game(&mut self) -> Result<(), ScriptError> {
        self.error_state = None;

        match loader::load_game_scripts(&self.lua, &self.config.scripts_dir) {
            Ok(()) => {
                log::info!("Game scripts loaded from {}", self.config.scripts_dir.display());
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to load game scripts: {}", e);
                self.error_state = Some(ScriptError::LuaError {
                    message: e.to_string(),
                    source: None,
                });
                Err(e)
            }
        }
    }

    /// Call the on_init() callback if it exists
    ///
    /// This should be called once after loading scripts, before the game loop starts.
    pub fn call_init(&self) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, callbacks::ON_INIT)
    }

    /// Call the on_update(dt) callback if it exists
    ///
    /// This should be called every frame with the delta time in seconds.
    pub fn call_update(&self, dt: f32) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle_with_args(&self.lua, callbacks::ON_UPDATE, dt)
    }

    /// Call the on_fixed_update(dt) callback if it exists
    ///
    /// This should be called at fixed intervals for physics updates.
    pub fn call_fixed_update(&self, dt: f32) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle_with_args(&self.lua, callbacks::ON_FIXED_UPDATE, dt)
    }

    /// Call the on_shutdown() callback if it exists
    ///
    /// This should be called when the game is shutting down.
    pub fn call_shutdown(&self) -> Result<(), ScriptError> {
        lifecycle::call_lifecycle(&self.lua, callbacks::ON_SHUTDOWN)
    }

    /// Get the last error that occurred
    ///
    /// This is useful for non-fatal error reporting (e.g., displaying in a debug console).
    pub fn last_error(&self) -> Option<&ScriptError> {
        self.error_state.as_ref()
    }

    /// Clear the error state
    pub fn clear_error(&mut self) {
        self.error_state = None;
    }

    /// Check if a callback exists without calling it
    pub fn has_callback(&self, name: &str) -> bool {
        lifecycle::has_callback(&self.lua, name)
    }

    /// Get a reference to the Lua VM (for advanced use cases)
    ///
    /// This is primarily for the bindings module to register additional functions.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get the current configuration
    pub fn config(&self) -> &ScriptConfig {
        &self.config
    }

    /// Execute arbitrary Lua code (for debugging/console)
    ///
    /// Returns the result as a string representation.
    /// This should only be used for debugging, not in production code.
    pub fn eval(&self, code: &str) -> Result<String, ScriptError> {
        let result: mlua::Value = self.lua.load(code).eval().map_err(|e| {
            ScriptError::LuaError {
                message: e.to_string(),
                source: Some(e),
            }
        })?;

        Ok(format_lua_value(&result))
    }
}

/// Format a Lua value for display
fn format_lua_value(value: &mlua::Value) -> String {
    match value {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => s.to_str().map(|s| format!("\"{}\"", s)).unwrap_or_else(|_| "<invalid string>".to_string()),
        mlua::Value::Table(_) => "table".to_string(),
        mlua::Value::Function(_) => "function".to_string(),
        mlua::Value::Thread(_) => "thread".to_string(),
        mlua::Value::UserData(_) => "userdata".to_string(),
        mlua::Value::LightUserData(_) => "lightuserdata".to_string(),
        mlua::Value::Error(e) => format!("error: {}", e),
        _ => "unknown".to_string(),
    }
}

impl std::fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("config", &self.config)
            .field("error_state", &self.error_state)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn setup_test_engine() -> (ScriptEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ScriptConfig::with_scripts_dir(temp_dir.path());
        let engine = ScriptEngine::new(config).unwrap();
        (engine, temp_dir)
    }

    #[test]
    fn test_engine_creation() {
        let config = ScriptConfig::default();
        let engine = ScriptEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_load_game_file_not_found() {
        let (mut engine, _temp_dir) = setup_test_engine();

        let result = engine.load_game();
        assert!(result.is_err());
        assert!(engine.last_error().is_some());
    }

    #[test]
    fn test_load_game_success() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(temp_dir.path().join("main.lua"), "-- empty script").unwrap();

        let result = engine.load_game();
        assert!(result.is_ok());
        assert!(engine.last_error().is_none());
    }

    #[test]
    fn test_call_init() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            init_called = false
            function on_init()
                init_called = true
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        engine.call_init().unwrap();

        let called: bool = engine.lua().globals().get("init_called").unwrap();
        assert!(called);
    }

    #[test]
    fn test_call_update() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            update_dt = 0
            function on_update(dt)
                update_dt = dt
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        engine.call_update(0.016).unwrap();

        let dt: f32 = engine.lua().globals().get("update_dt").unwrap();
        assert!((dt - 0.016).abs() < 0.0001);
    }

    #[test]
    fn test_call_fixed_update() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            fixed_dt = 0
            function on_fixed_update(dt)
                fixed_dt = dt
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        engine.call_fixed_update(1.0 / 60.0).unwrap();

        let dt: f32 = engine.lua().globals().get("fixed_dt").unwrap();
        assert!((dt - 1.0 / 60.0).abs() < 0.0001);
    }

    #[test]
    fn test_call_shutdown() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            shutdown_called = false
            function on_shutdown()
                shutdown_called = true
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        engine.call_shutdown().unwrap();

        let called: bool = engine.lua().globals().get("shutdown_called").unwrap();
        assert!(called);
    }

    #[test]
    fn test_missing_callbacks_ok() {
        let (mut engine, temp_dir) = setup_test_engine();

        // Script with no callbacks defined
        fs::write(temp_dir.path().join("main.lua"), "-- no callbacks").unwrap();

        engine.load_game().unwrap();

        // All of these should succeed (silently)
        assert!(engine.call_init().is_ok());
        assert!(engine.call_update(0.016).is_ok());
        assert!(engine.call_fixed_update(1.0 / 60.0).is_ok());
        assert!(engine.call_shutdown().is_ok());
    }

    #[test]
    fn test_has_callback() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            function on_init() end
            -- on_update not defined
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();

        assert!(engine.has_callback("on_init"));
        assert!(!engine.has_callback("on_update"));
    }

    #[test]
    fn test_eval() {
        let (engine, _temp_dir) = setup_test_engine();

        let result = engine.eval("return 2 + 2").unwrap();
        assert_eq!(result, "4");

        let result = engine.eval("return 'hello'").unwrap();
        assert_eq!(result, "\"hello\"");

        let result = engine.eval("return true").unwrap();
        assert_eq!(result, "true");

        let result = engine.eval("return nil").unwrap();
        assert_eq!(result, "nil");
    }

    #[test]
    fn test_eval_error() {
        let (engine, _temp_dir) = setup_test_engine();

        let result = engine.eval("this is not valid lua");
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_error() {
        let (mut engine, _temp_dir) = setup_test_engine();

        // Trigger an error
        let _ = engine.load_game();
        assert!(engine.last_error().is_some());

        engine.clear_error();
        assert!(engine.last_error().is_none());
    }

    #[test]
    fn test_config_accessors() {
        let config = ScriptConfig::with_scripts_dir("/test/scripts");
        let engine = ScriptEngine::new(config).unwrap();

        assert_eq!(engine.config().scripts_dir.to_string_lossy(), "/test/scripts");
    }

    #[test]
    fn test_debug_impl() {
        let config = ScriptConfig::default();
        let engine = ScriptEngine::new(config).unwrap();

        let debug_str = format!("{:?}", engine);
        assert!(debug_str.contains("ScriptEngine"));
        assert!(debug_str.contains("config"));
    }

    #[test]
    fn test_runtime_error_in_callback() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            function on_update(dt)
                error("Test error")
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        let result = engine.call_update(0.016);

        assert!(result.is_err());
        if let Err(ScriptError::RuntimeError { callback, message, .. }) = result {
            assert_eq!(callback, "on_update");
            assert!(message.contains("Test error"));
        } else {
            panic!("Expected RuntimeError");
        }
    }

    #[test]
    fn test_syntax_error_on_load() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            function on_init(
                -- syntax error: missing closing paren
            end
        "#,
        )
        .unwrap();

        let result = engine.load_game();
        assert!(result.is_err());
    }

    #[test]
    fn test_full_lifecycle() {
        let (mut engine, temp_dir) = setup_test_engine();

        fs::write(
            temp_dir.path().join("main.lua"),
            r#"
            lifecycle = {}

            function on_init()
                table.insert(lifecycle, "init")
            end

            function on_update(dt)
                table.insert(lifecycle, "update:" .. tostring(dt))
            end

            function on_fixed_update(dt)
                table.insert(lifecycle, "fixed:" .. tostring(dt))
            end

            function on_shutdown()
                table.insert(lifecycle, "shutdown")
            end
        "#,
        )
        .unwrap();

        engine.load_game().unwrap();
        engine.call_init().unwrap();
        engine.call_update(0.016).unwrap();
        engine.call_fixed_update(0.02).unwrap();
        engine.call_shutdown().unwrap();

        let lifecycle: mlua::Table = engine.lua().globals().get("lifecycle").unwrap();
        assert_eq!(lifecycle.len().unwrap(), 4);
        assert_eq!(lifecycle.get::<String>(1).unwrap(), "init");
        assert!(lifecycle.get::<String>(2).unwrap().starts_with("update:"));
        assert!(lifecycle.get::<String>(3).unwrap().starts_with("fixed:"));
        assert_eq!(lifecycle.get::<String>(4).unwrap(), "shutdown");
    }
}
