//! Lua VM initialization and sandboxing
//!
//! Creates a sandboxed Lua 5.4 VM suitable for game scripting.
//! Dangerous operations (os, io, debug, loadfile, dofile) are removed.

use mlua::{Lua, Result as LuaResult};
use std::path::PathBuf;

/// Configuration for the script engine
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Directory containing game scripts (default: "scripts")
    pub scripts_dir: PathBuf,
    /// Enable hot-reload of scripts (default: false)
    pub hot_reload: bool,
    /// Lua memory limit in bytes (0 = unlimited, default: 64MB)
    pub memory_limit: usize,
    /// Lua instruction limit per call (0 = unlimited, default: 0)
    pub instruction_limit: u32,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            scripts_dir: PathBuf::from("scripts"),
            hot_reload: false,
            memory_limit: 64 * 1024 * 1024, // 64MB
            instruction_limit: 0,
        }
    }
}

impl ScriptConfig {
    /// Create config with a specific scripts directory
    pub fn with_scripts_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            scripts_dir: dir.into(),
            ..Default::default()
        }
    }
}

/// Create a new sandboxed Lua VM
///
/// The VM is configured with:
/// - `package.path` set to load from the scripts directory
/// - Dangerous globals removed (os, io, debug, loadfile, dofile)
/// - `print()` redirected to `log::info!`
pub fn create_lua_vm(config: &ScriptConfig) -> LuaResult<Lua> {
    let lua = Lua::new();

    // Configure package path for the scripts directory
    configure_package_path(&lua, &config.scripts_dir)?;

    // Remove dangerous globals for sandboxing
    sandbox_vm(&lua)?;

    // Replace print() with logging
    setup_print_redirect(&lua)?;

    Ok(lua)
}

/// Configure Lua's package.path for the scripts directory
fn configure_package_path(lua: &Lua, scripts_dir: &PathBuf) -> LuaResult<()> {
    let globals = lua.globals();
    let package: mlua::Table = globals.get("package")?;

    // Convert to canonical path string
    let scripts_path = scripts_dir.to_string_lossy();

    // Set package.path to search in scripts directory
    // Pattern: scripts/?.lua, scripts/lib/?.lua
    let path = format!("{}/?.lua;{}/lib/?.lua", scripts_path, scripts_path);
    package.set("path", path)?;

    // Disable C module loading entirely
    package.set("cpath", "")?;
    package.raw_set("loadlib", mlua::Nil)?;

    Ok(())
}

/// Remove dangerous globals for sandboxing
fn sandbox_vm(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    // Remove os library (file system, process control, etc.)
    globals.raw_set("os", mlua::Nil)?;

    // Remove io library (file I/O)
    globals.raw_set("io", mlua::Nil)?;

    // Remove debug library (sandbox escape vector)
    globals.raw_set("debug", mlua::Nil)?;

    // Remove dangerous file loading functions
    globals.raw_set("loadfile", mlua::Nil)?;
    globals.raw_set("dofile", mlua::Nil)?;

    Ok(())
}

/// Replace Lua's print() with log::info!
fn setup_print_redirect(lua: &Lua) -> LuaResult<()> {
    let print_fn = lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
        let parts: Vec<String> = args
            .iter()
            .map(|v| match v {
                mlua::Value::Nil => "nil".to_string(),
                mlua::Value::Boolean(b) => b.to_string(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                mlua::Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_else(|_| "<invalid utf8>".to_string()),
                mlua::Value::Table(_) => "table".to_string(),
                mlua::Value::Function(_) => "function".to_string(),
                mlua::Value::Thread(_) => "thread".to_string(),
                mlua::Value::UserData(_) => "userdata".to_string(),
                mlua::Value::LightUserData(_) => "lightuserdata".to_string(),
                mlua::Value::Error(e) => format!("error: {}", e),
                _ => "unknown".to_string(),
            })
            .collect();

        log::info!("[Lua] {}", parts.join("\t"));
        Ok(())
    })?;

    lua.globals().set("print", print_fn)?;
    Ok(())
}

/// Check if a global exists in the Lua VM
pub fn has_global(lua: &Lua, name: &str) -> LuaResult<bool> {
    let globals = lua.globals();
    let value: mlua::Value = globals.get(name)?;
    Ok(!matches!(value, mlua::Value::Nil))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScriptConfig::default();
        assert_eq!(config.scripts_dir, PathBuf::from("scripts"));
        assert!(!config.hot_reload);
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
        assert_eq!(config.instruction_limit, 0);
    }

    #[test]
    fn test_config_with_scripts_dir() {
        let config = ScriptConfig::with_scripts_dir("/game/scripts");
        assert_eq!(config.scripts_dir, PathBuf::from("/game/scripts"));
    }

    #[test]
    fn test_create_vm_succeeds() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).expect("VM creation should succeed");

        // Should be able to run simple Lua code
        lua.load("local x = 1 + 1").exec().expect("Basic Lua should work");
    }

    #[test]
    fn test_sandbox_removes_os() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        assert!(!has_global(&lua, "os").unwrap(), "os should be removed");
    }

    #[test]
    fn test_sandbox_removes_io() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        assert!(!has_global(&lua, "io").unwrap(), "io should be removed");
    }

    #[test]
    fn test_sandbox_removes_debug() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        assert!(!has_global(&lua, "debug").unwrap(), "debug should be removed");
    }

    #[test]
    fn test_sandbox_removes_loadfile() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        assert!(
            !has_global(&lua, "loadfile").unwrap(),
            "loadfile should be removed"
        );
    }

    #[test]
    fn test_sandbox_removes_dofile() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        assert!(
            !has_global(&lua, "dofile").unwrap(),
            "dofile should be removed"
        );
    }

    #[test]
    fn test_sandbox_keeps_safe_globals() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        // These should still exist
        assert!(has_global(&lua, "print").unwrap(), "print should exist");
        assert!(has_global(&lua, "type").unwrap(), "type should exist");
        assert!(has_global(&lua, "pairs").unwrap(), "pairs should exist");
        assert!(has_global(&lua, "ipairs").unwrap(), "ipairs should exist");
        assert!(has_global(&lua, "tonumber").unwrap(), "tonumber should exist");
        assert!(has_global(&lua, "tostring").unwrap(), "tostring should exist");
        assert!(has_global(&lua, "table").unwrap(), "table should exist");
        assert!(has_global(&lua, "string").unwrap(), "string should exist");
        assert!(has_global(&lua, "math").unwrap(), "math should exist");
        assert!(has_global(&lua, "require").unwrap(), "require should exist");
    }

    #[test]
    fn test_print_redirect_doesnt_panic() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        // Should not panic, even if log isn't configured
        lua.load(r#"print("Hello", 42, true, nil)"#)
            .exec()
            .expect("print should work");
    }

    #[test]
    fn test_print_with_table() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        // Tables should convert to "table" string
        lua.load(r#"print({1, 2, 3})"#)
            .exec()
            .expect("print with table should work");
    }

    #[test]
    fn test_package_path_configured() {
        let config = ScriptConfig::with_scripts_dir("/game/scripts");
        let lua = create_lua_vm(&config).unwrap();

        let package: mlua::Table = lua.globals().get("package").unwrap();
        let path: String = package.get("path").unwrap();

        assert!(
            path.contains("/game/scripts/?.lua"),
            "package.path should include scripts dir"
        );
        assert!(
            path.contains("/game/scripts/lib/?.lua"),
            "package.path should include lib subdir"
        );
    }

    #[test]
    fn test_cpath_disabled() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        let package: mlua::Table = lua.globals().get("package").unwrap();
        let cpath: String = package.get("cpath").unwrap();

        assert!(cpath.is_empty(), "cpath should be empty for sandboxing");
    }

    #[test]
    fn test_loadlib_disabled() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        let package: mlua::Table = lua.globals().get("package").unwrap();
        let loadlib: mlua::Value = package.get("loadlib").unwrap();

        assert!(
            matches!(loadlib, mlua::Value::Nil),
            "loadlib should be nil"
        );
    }

    #[test]
    fn test_load_string_still_works() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();

        // load() with strings should still work (for eval)
        let result: i32 = lua
            .load("return 2 + 2")
            .eval()
            .expect("load with string should work");
        assert_eq!(result, 4);
    }
}
