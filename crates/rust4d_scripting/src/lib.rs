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

pub mod bindings;
pub mod error;
pub mod hot_reload;
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
    #[cfg(feature = "hot-reload")]
    watcher: Option<hot_reload::ScriptWatcher>,
}

impl ScriptEngine {
    /// Create a new scripting engine with the given configuration.
    ///
    /// This initializes the Lua VM with sandboxed globals and configured
    /// package paths but does not load any scripts.
    ///
    /// If the `hot-reload` feature is enabled and `config.hot_reload` is true,
    /// a file watcher will be created to monitor the scripts directory.
    pub fn new(config: ScriptConfig) -> Result<Self, ScriptError> {
        let lua = vm::create_lua_vm(&config)?;

        #[cfg(feature = "hot-reload")]
        let watcher = if config.hot_reload {
            Some(hot_reload::ScriptWatcher::new(&config.scripts_dir)?)
        } else {
            None
        };

        Ok(Self {
            lua,
            config,
            error_state: None,
            #[cfg(feature = "hot-reload")]
            watcher,
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

    /// Check if a lifecycle callback exists without calling it
    pub fn has_callback(&self, name: &str) -> bool {
        lifecycle::has_callback(&self.lua, name)
    }

    /// Execute arbitrary Lua code and return result as string (for debugging/REPL)
    ///
    /// This evaluates the given Lua code and returns a string representation
    /// of the result. Useful for debug consoles and interactive tools.
    pub fn eval(&self, code: &str) -> Result<String, ScriptError> {
        let result: LuaValue = self.lua.load(code).eval()?;
        Ok(format_lua_value(&result))
    }

    /// Check for changed files and reload them.
    ///
    /// This should be called once per frame in the game loop. It polls the
    /// file watcher for changes and reloads any modified Lua files.
    ///
    /// Returns `true` if any files were successfully reloaded.
    ///
    /// # Error Handling
    ///
    /// If a reload fails (e.g., syntax error in the new code), the error is
    /// logged and stored in the error state, but the old version of the module
    /// continues to run. This allows developers to fix errors without crashing.
    ///
    /// # Feature Gate
    ///
    /// This method only performs work when the `hot-reload` feature is enabled.
    /// Otherwise it always returns `false`.
    pub fn check_hot_reload(&mut self) -> bool {
        #[cfg(feature = "hot-reload")]
        {
            // Collect changes and scripts_dir first to avoid borrow conflicts
            let (changed, scripts_dir) = match self.watcher {
                Some(ref watcher) => (watcher.poll_changes(), watcher.scripts_dir().to_path_buf()),
                None => return false,
            };

            let mut reloaded = false;

            for path in changed {
                if let Some(module_name) = hot_reload::path_to_module_name(&path, &scripts_dir) {
                    match hot_reload::reload_module(&self.lua, &module_name, &path) {
                        Ok(()) => {
                            reloaded = true;
                            self.clear_error();
                        }
                        Err(e) => {
                            log::error!("{}", e);
                            self.set_error(e);
                            // Keep old version running
                        }
                    }
                }
            }
            return reloaded;
        }

        #[cfg(not(feature = "hot-reload"))]
        false
    }

    /// Check if hot-reload is enabled and active.
    ///
    /// Returns `true` if the `hot-reload` feature is enabled and the
    /// configuration has `hot_reload: true`.
    pub fn is_hot_reload_enabled(&self) -> bool {
        #[cfg(feature = "hot-reload")]
        {
            self.watcher.is_some()
        }
        #[cfg(not(feature = "hot-reload"))]
        {
            false
        }
    }
}

/// Format a Lua value for display
fn format_lua_value(value: &LuaValue) -> String {
    match value {
        LuaValue::Nil => "nil".to_string(),
        LuaValue::Boolean(b) => b.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        LuaValue::Number(n) => n.to_string(),
        LuaValue::String(s) => s
            .to_str()
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|_| "<invalid string>".to_string()),
        LuaValue::Table(_) => "table".to_string(),
        LuaValue::Function(_) => "function".to_string(),
        LuaValue::Thread(_) => "thread".to_string(),
        LuaValue::UserData(_) => "userdata".to_string(),
        LuaValue::LightUserData(_) => "lightuserdata".to_string(),
        LuaValue::Error(e) => format!("error: {}", e),
        _ => "unknown".to_string(),
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
        let count: i64 = engine.lua().load("return #lifecycle_log").eval().unwrap();
        assert_eq!(count, 4);

        let first: String = engine.lua().load("return lifecycle_log[1]").eval().unwrap();
        assert_eq!(first, "init");

        let last: String = engine.lua().load("return lifecycle_log[4]").eval().unwrap();
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
        let mut module_file = std::fs::File::create(dir.path().join("utils.lua")).unwrap();
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

    #[test]
    fn test_eval_returns_integer() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        let result = engine.eval("return 42").unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_eval_returns_string() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        let result = engine.eval("return 'hello'").unwrap();
        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn test_eval_returns_nil() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        let result = engine.eval("return nil").unwrap();
        assert_eq!(result, "nil");
    }

    #[test]
    fn test_eval_returns_boolean() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        assert_eq!(engine.eval("return true").unwrap(), "true");
        assert_eq!(engine.eval("return false").unwrap(), "false");
    }

    #[test]
    fn test_eval_returns_table() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        let result = engine.eval("return {1, 2, 3}").unwrap();
        assert_eq!(result, "table");
    }

    #[test]
    fn test_eval_error_on_invalid_code() {
        let (_dir, config) = create_game_dir();
        let engine = ScriptEngine::new(config).unwrap();
        let result = engine.eval("this is not valid lua");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_callback_true_when_exists() {
        let (dir, config) = create_game_dir();
        let main_path = dir.path().join("main.lua");
        std::fs::write(&main_path, "function on_init() end").unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        assert!(engine.has_callback("on_init"));
    }

    #[test]
    fn test_has_callback_false_when_missing() {
        let (dir, config) = create_game_dir();
        let main_path = dir.path().join("main.lua");
        std::fs::write(&main_path, "-- no callbacks").unwrap();

        let mut engine = ScriptEngine::new(config).unwrap();
        engine.load_game().unwrap();
        assert!(!engine.has_callback("on_init"));
    }
}
