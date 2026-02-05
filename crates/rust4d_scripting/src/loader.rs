//! Script loading from the filesystem
//!
//! Handles loading and executing Lua scripts from the configured scripts directory.

use crate::error::ScriptError;
use mlua::Lua;
use std::path::Path;

/// Load and execute the main game script (main.lua)
///
/// This is the entry point for game scripts. The main.lua file should
/// define global callbacks like `on_init()`, `on_update(dt)`, etc.
pub fn load_game_scripts(lua: &Lua, scripts_dir: &Path) -> Result<(), ScriptError> {
    let main_path = scripts_dir.join("main.lua");

    load_script(lua, &main_path)
}

/// Load and execute a script from a path
pub fn load_script(lua: &Lua, path: &Path) -> Result<(), ScriptError> {
    // Check if file exists
    if !path.exists() {
        return Err(ScriptError::file_not_found(path));
    }

    // Read the script content
    let content = std::fs::read_to_string(path).map_err(|e| ScriptError::io_error(path, e))?;

    // Set the script name for error messages
    let script_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Load and execute the script
    lua.load(&content)
        .set_name(script_name)
        .exec()
        .map_err(|e| ScriptError::LuaError {
            message: format!("Error in {}: {}", path.display(), e),
            source: Some(e),
        })?;

    log::debug!("Loaded script: {}", path.display());
    Ok(())
}

/// Load a script from a string (useful for testing)
pub fn load_script_string(lua: &Lua, source: &str, name: &str) -> Result<(), ScriptError> {
    lua.load(source)
        .set_name(name)
        .exec()
        .map_err(|e| ScriptError::LuaError {
            message: format!("Error in {}: {}", name, e),
            source: Some(e),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{create_lua_vm, ScriptConfig};
    use tempfile::TempDir;
    use std::fs;

    fn setup_test_vm() -> (Lua, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ScriptConfig::with_scripts_dir(temp_dir.path());
        let lua = create_lua_vm(&config).unwrap();
        (lua, temp_dir)
    }

    #[test]
    fn test_load_main_lua() {
        let (lua, temp_dir) = setup_test_vm();

        // Create main.lua
        fs::write(temp_dir.path().join("main.lua"), r#"
            function on_init()
                -- Game initialization
            end
        "#).unwrap();

        let result = load_game_scripts(&lua, temp_dir.path());
        assert!(result.is_ok(), "Should load main.lua successfully");
    }

    #[test]
    fn test_main_lua_not_found() {
        let (lua, temp_dir) = setup_test_vm();

        let result = load_game_scripts(&lua, temp_dir.path());
        assert!(matches!(result, Err(ScriptError::FileNotFound { .. })));
    }

    #[test]
    fn test_load_script_creates_globals() {
        let (lua, temp_dir) = setup_test_vm();

        fs::write(temp_dir.path().join("main.lua"), r#"
            game_name = "Test Game"
            function on_init()
                return 42
            end
        "#).unwrap();

        load_game_scripts(&lua, temp_dir.path()).unwrap();

        // Verify global was created
        let name: String = lua.globals().get("game_name").unwrap();
        assert_eq!(name, "Test Game");

        // Verify function was created
        let on_init: mlua::Function = lua.globals().get("on_init").unwrap();
        let result: i32 = on_init.call(()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_syntax_error_produces_lua_error() {
        let (lua, temp_dir) = setup_test_vm();

        // Create main.lua with syntax error
        fs::write(temp_dir.path().join("main.lua"), r#"
            function on_init(
                -- Missing closing parenthesis
            end
        "#).unwrap();

        let result = load_game_scripts(&lua, temp_dir.path());
        assert!(matches!(result, Err(ScriptError::LuaError { .. })));
    }

    #[test]
    fn test_load_script_string() {
        let (lua, _temp_dir) = setup_test_vm();

        let result = load_script_string(&lua, "test_value = 123", "test.lua");
        assert!(result.is_ok());

        let value: i32 = lua.globals().get("test_value").unwrap();
        assert_eq!(value, 123);
    }

    #[test]
    fn test_load_script_string_syntax_error() {
        let (lua, _temp_dir) = setup_test_vm();

        let result = load_script_string(&lua, "this is not valid lua {{{", "bad.lua");
        assert!(matches!(result, Err(ScriptError::LuaError { .. })));
    }

    #[test]
    fn test_require_resolves_from_scripts_dir() {
        let (lua, temp_dir) = setup_test_vm();

        // Create a module in the scripts directory
        fs::write(temp_dir.path().join("mymodule.lua"), r#"
            local M = {}
            M.value = 999
            return M
        "#).unwrap();

        // Create main.lua that requires the module
        fs::write(temp_dir.path().join("main.lua"), r#"
            local mymodule = require("mymodule")
            loaded_value = mymodule.value
        "#).unwrap();

        load_game_scripts(&lua, temp_dir.path()).unwrap();

        let loaded: i32 = lua.globals().get("loaded_value").unwrap();
        assert_eq!(loaded, 999);
    }

    #[test]
    fn test_require_resolves_from_lib_subdir() {
        let (lua, temp_dir) = setup_test_vm();

        // Create lib subdirectory
        let lib_dir = temp_dir.path().join("lib");
        fs::create_dir(&lib_dir).unwrap();

        // Create a module in lib/
        fs::write(lib_dir.join("utils.lua"), r#"
            local M = {}
            M.helper = function() return "helped" end
            return M
        "#).unwrap();

        // Create main.lua that requires from lib
        fs::write(temp_dir.path().join("main.lua"), r#"
            local utils = require("utils")
            result = utils.helper()
        "#).unwrap();

        load_game_scripts(&lua, temp_dir.path()).unwrap();

        let result: String = lua.globals().get("result").unwrap();
        assert_eq!(result, "helped");
    }

    #[test]
    fn test_load_multiple_scripts() {
        let (lua, temp_dir) = setup_test_vm();

        // Create first script
        let script1_path = temp_dir.path().join("script1.lua");
        fs::write(&script1_path, "value1 = 1").unwrap();

        // Create second script
        let script2_path = temp_dir.path().join("script2.lua");
        fs::write(&script2_path, "value2 = 2").unwrap();

        load_script(&lua, &script1_path).unwrap();
        load_script(&lua, &script2_path).unwrap();

        let v1: i32 = lua.globals().get("value1").unwrap();
        let v2: i32 = lua.globals().get("value2").unwrap();
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
    }
}
