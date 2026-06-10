//! Hot-reload support for Lua scripts
//!
//! Watches the scripts directory for changes and reloads modified modules.
//! This module is only fully functional when the `hot-reload` feature is enabled.
//!
//! # Usage
//!
//! ```no_run
//! use rust4d_scripting::{ScriptEngine, ScriptConfig};
//!
//! let config = ScriptConfig {
//!     scripts_dir: "my_game/scripts".to_string(),
//!     hot_reload: true,
//!     ..Default::default()
//! };
//! let mut engine = ScriptEngine::new(config).unwrap();
//!
//! // In the game loop:
//! if engine.check_hot_reload() {
//!     println!("Scripts were reloaded!");
//! }
//! ```

#[cfg(feature = "hot-reload")]
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
#[cfg(feature = "hot-reload")]
use std::collections::HashSet;
use std::path::{Path, PathBuf};
#[cfg(feature = "hot-reload")]
use std::sync::mpsc::{self, Receiver};

use crate::error::ScriptError;
use mlua::Lua;

/// Watches a directory for Lua file changes.
///
/// When the `hot-reload` feature is enabled, this uses the `notify` crate
/// to watch for file system events. When disabled, all operations are no-ops.
pub struct ScriptWatcher {
    #[cfg(feature = "hot-reload")]
    _watcher: RecommendedWatcher,
    #[cfg(feature = "hot-reload")]
    rx: Receiver<notify::Result<Event>>,
    scripts_dir: PathBuf,
}

impl ScriptWatcher {
    /// Create a new watcher for the scripts directory.
    ///
    /// # Errors
    ///
    /// Returns `ScriptError::WatcherError` if the file watcher cannot be created
    /// or if it fails to watch the scripts directory.
    #[cfg(feature = "hot-reload")]
    pub fn new(scripts_dir: impl AsRef<Path>) -> Result<Self, ScriptError> {
        let (tx, rx) = mpsc::channel();
        let scripts_path = scripts_dir.as_ref().to_path_buf();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(|e| ScriptError::WatcherError(e.to_string()))?;

        watcher
            .watch(&scripts_path, RecursiveMode::Recursive)
            .map_err(|e| ScriptError::WatcherError(e.to_string()))?;

        log::info!(
            "[scripting] Hot-reload enabled, watching: {}",
            scripts_path.display()
        );

        Ok(Self {
            _watcher: watcher,
            rx,
            scripts_dir: scripts_path,
        })
    }

    /// Stub implementation when hot-reload feature is disabled.
    #[cfg(not(feature = "hot-reload"))]
    pub fn new(scripts_dir: impl AsRef<Path>) -> Result<Self, ScriptError> {
        Ok(Self {
            scripts_dir: scripts_dir.as_ref().to_path_buf(),
        })
    }

    /// Poll for changed .lua files.
    ///
    /// Returns a list of paths to Lua files that have been modified or created
    /// since the last poll. Duplicate events are deduplicated.
    #[cfg(feature = "hot-reload")]
    pub fn poll_changes(&self) -> Vec<PathBuf> {
        let mut changed = HashSet::new();

        while let Ok(Ok(event)) = self.rx.try_recv() {
            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    for path in event.paths {
                        if path.extension().is_some_and(|ext| ext == "lua") {
                            changed.insert(path);
                        }
                    }
                }
                _ => {}
            }
        }

        changed.into_iter().collect()
    }

    /// Stub implementation when hot-reload feature is disabled.
    #[cfg(not(feature = "hot-reload"))]
    pub fn poll_changes(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Get the scripts directory being watched.
    pub fn scripts_dir(&self) -> &Path {
        &self.scripts_dir
    }
}

/// Convert a file path to a Lua module name.
///
/// Given a file path and the scripts directory, returns the module name
/// that would be used with `require()` in Lua.
///
/// # Examples
///
/// ```text
/// "/path/to/scripts/enemies/rusher.lua" with scripts_dir="/path/to/scripts"
/// returns Some("enemies.rusher")
/// ```
pub fn path_to_module_name(path: &Path, scripts_dir: &Path) -> Option<String> {
    let relative = path.strip_prefix(scripts_dir).ok()?;
    let stem = relative.with_extension("");
    let parts: Vec<&str> = stem
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// Reload a changed module in the Lua VM.
///
/// This performs the following steps:
/// 1. Reads the new source from disk
/// 2. Clears the module from `package.loaded`
/// 3. Re-executes the module using `eval()` to capture its return value
/// 4. Stores the return value in `package.loaded` (so `require()` returns the new version)
/// 5. Calls `on_reload()` if it exists (for scripts to handle state migration)
///
/// # Module Return Values
///
/// Lua modules typically return a table of exports. When hot-reloading, we must
/// preserve this pattern: the module is evaluated, and its return value is stored
/// in `package.loaded[module_name]`. This ensures that existing code holding
/// references to the old module table will still work (though they'll see old values),
/// while new `require()` calls will get the fresh version.
///
/// # Error Recovery
///
/// If the reload fails (e.g., syntax error), the error is returned but the
/// old version of the module continues to run. This allows developers to
/// fix errors without crashing the game.
///
/// # Errors
///
/// Returns `ScriptError::IoError` if the file cannot be read, or
/// `ScriptError::ModuleReloadError` if the Lua execution fails.
pub fn reload_module(lua: &Lua, module_name: &str, file_path: &Path) -> Result<(), ScriptError> {
    // Read new source
    let source = std::fs::read_to_string(file_path)
        .map_err(|e| ScriptError::IoError(file_path.display().to_string(), e))?;

    // Clear from package.loaded first (so require() won't return cached version)
    let clear_code = format!(r#"package.loaded["{}"] = nil"#, module_name);
    lua.load(&clear_code)
        .exec()
        .map_err(ScriptError::LuaError)?;

    // Re-execute the module using eval() to capture its return value.
    // This is important: Lua modules typically return a table of exports,
    // and we need to store that return value in package.loaded so that
    // subsequent require() calls get the updated module.
    let result: mlua::Value = lua
        .load(&source)
        .set_name(file_path.to_string_lossy())
        .eval()
        .map_err(|e| ScriptError::ModuleReloadError {
            path: file_path.display().to_string(),
            error: e,
        })?;

    // Store the result in package.loaded so require() returns the new version.
    // If the module returned nil/nothing, we store true (Lua convention for "loaded").
    let package: mlua::Table = lua
        .globals()
        .get("package")
        .map_err(ScriptError::LuaError)?;
    let loaded: mlua::Table = package.get("loaded").map_err(ScriptError::LuaError)?;

    let value_to_store = if result.is_nil() {
        mlua::Value::Boolean(true)
    } else {
        result
    };
    loaded
        .set(module_name, value_to_store)
        .map_err(ScriptError::LuaError)?;

    // Call on_reload() if it exists (silently ignore if not)
    let _ = lua.load("if on_reload then on_reload() end").exec();

    log::info!("[scripting] Reloaded: {}", file_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_module_name_simple() {
        let scripts_dir = Path::new("/game/scripts");

        assert_eq!(
            path_to_module_name(Path::new("/game/scripts/main.lua"), scripts_dir),
            Some("main".to_string())
        );
    }

    #[test]
    fn test_path_to_module_name_nested() {
        let scripts_dir = Path::new("/game/scripts");

        assert_eq!(
            path_to_module_name(Path::new("/game/scripts/enemies/rusher.lua"), scripts_dir),
            Some("enemies.rusher".to_string())
        );
    }

    #[test]
    fn test_path_to_module_name_deeply_nested() {
        let scripts_dir = Path::new("/game/scripts");

        assert_eq!(
            path_to_module_name(
                Path::new("/game/scripts/systems/ai/behaviors/patrol.lua"),
                scripts_dir
            ),
            Some("systems.ai.behaviors.patrol".to_string())
        );
    }

    #[test]
    fn test_path_to_module_name_outside_scripts_dir() {
        let scripts_dir = Path::new("/game/scripts");

        // Path not under scripts_dir
        assert_eq!(
            path_to_module_name(Path::new("/other/path/module.lua"), scripts_dir),
            None
        );
    }

    #[test]
    fn test_reload_module_clears_package_loaded() {
        let lua = mlua::Lua::new();

        // Create a temp file
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lua");
        std::fs::write(&path, "test_value = 1").unwrap();

        // Load it initially
        lua.load(std::fs::read_to_string(&path).unwrap())
            .exec()
            .unwrap();
        let val: i64 = lua.load("return test_value").eval().unwrap();
        assert_eq!(val, 1);

        // Change it
        std::fs::write(&path, "test_value = 2").unwrap();

        // Reload
        reload_module(&lua, "test", &path).unwrap();
        let val: i64 = lua.load("return test_value").eval().unwrap();
        assert_eq!(val, 2);
    }

    #[test]
    fn test_reload_module_calls_on_reload() {
        let lua = mlua::Lua::new();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lua");
        std::fs::write(
            &path,
            r#"
            reload_called = false
            function on_reload()
                reload_called = true
            end
            "#,
        )
        .unwrap();

        reload_module(&lua, "test", &path).unwrap();

        let called: bool = lua.load("return reload_called").eval().unwrap();
        assert!(called);
    }

    #[test]
    fn test_reload_module_handles_missing_on_reload() {
        let lua = mlua::Lua::new();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lua");
        std::fs::write(&path, "-- no on_reload function").unwrap();

        // Should not error even without on_reload
        let result = reload_module(&lua, "test", &path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reload_module_returns_error_on_syntax_error() {
        let lua = mlua::Lua::new();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.lua");
        std::fs::write(&path, "this is not valid lua syntax!!!").unwrap();

        let result = reload_module(&lua, "bad", &path);
        assert!(result.is_err());
        assert!(matches!(result, Err(ScriptError::ModuleReloadError { .. })));
    }

    #[test]
    fn test_reload_module_returns_error_on_missing_file() {
        let lua = mlua::Lua::new();

        let path = Path::new("/nonexistent/path/module.lua");
        let result = reload_module(&lua, "module", path);
        assert!(result.is_err());
        assert!(matches!(result, Err(ScriptError::IoError(_, _))));
    }

    #[cfg(feature = "hot-reload")]
    #[test]
    fn test_watcher_creation() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = ScriptWatcher::new(dir.path());
        assert!(watcher.is_ok());
    }

    #[cfg(feature = "hot-reload")]
    #[test]
    fn test_watcher_scripts_dir() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = ScriptWatcher::new(dir.path()).unwrap();
        assert_eq!(watcher.scripts_dir(), dir.path());
    }

    #[cfg(not(feature = "hot-reload"))]
    #[test]
    fn test_watcher_stub_poll_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = ScriptWatcher::new(dir.path()).unwrap();
        assert!(watcher.poll_changes().is_empty());
    }
}
