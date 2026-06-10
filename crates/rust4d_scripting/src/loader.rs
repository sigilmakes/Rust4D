//! Script loading and require() resolution

use crate::error::ScriptError;
use mlua::prelude::*;

/// Load the game's main.lua entry point and execute it.
///
/// This establishes the global game module by executing `main.lua` from the
/// scripts directory. The script can use `require()` to load other modules,
/// which will be resolved relative to the scripts directory.
pub fn load_game_scripts(lua: &Lua, scripts_dir: &str) -> Result<(), ScriptError> {
    let main_path = format!("{}/main.lua", scripts_dir);

    if !std::path::Path::new(&main_path).exists() {
        return Err(ScriptError::FileNotFound(main_path));
    }

    let source = std::fs::read_to_string(&main_path)
        .map_err(|e| ScriptError::IoError(main_path.clone(), e))?;

    lua.load(source)
        .set_name(main_path)
        .exec()
        .map_err(ScriptError::LuaError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{create_lua_vm, ScriptConfig};
    use std::io::Write;

    #[test]
    fn test_load_main_lua() {
        let dir = tempfile::tempdir().unwrap();
        let main_path = dir.path().join("main.lua");
        std::fs::write(&main_path, "test_global = 42").unwrap();

        let config = ScriptConfig {
            scripts_dir: dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let lua = create_lua_vm(&config).unwrap();
        load_game_scripts(&lua, &config.scripts_dir).unwrap();

        let val: i64 = lua.load("return test_global").eval().unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn test_missing_main_lua() {
        let dir = tempfile::tempdir().unwrap();
        let config = ScriptConfig {
            scripts_dir: dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let lua = create_lua_vm(&config).unwrap();
        let result = load_game_scripts(&lua, &config.scripts_dir);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ScriptError::FileNotFound(_)));
    }

    #[test]
    fn test_syntax_error_in_main() {
        let dir = tempfile::tempdir().unwrap();
        let main_path = dir.path().join("main.lua");
        std::fs::write(&main_path, "this is not valid lua!!!").unwrap();

        let config = ScriptConfig {
            scripts_dir: dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let lua = create_lua_vm(&config).unwrap();
        let result = load_game_scripts(&lua, &config.scripts_dir);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScriptError::LuaError(_)));
    }

    #[test]
    fn test_require_resolves_from_scripts_dir() {
        let dir = tempfile::tempdir().unwrap();

        // Create a module
        let mut module_file = std::fs::File::create(dir.path().join("mymodule.lua")).unwrap();
        writeln!(module_file, "local M = {{}}").unwrap();
        writeln!(module_file, "M.value = 99").unwrap();
        writeln!(module_file, "return M").unwrap();

        // Create main.lua that requires the module
        let main_path = dir.path().join("main.lua");
        std::fs::write(
            &main_path,
            r#"
            local mymod = require("mymodule")
            loaded_value = mymod.value
            "#,
        )
        .unwrap();

        let config = ScriptConfig {
            scripts_dir: dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let lua = create_lua_vm(&config).unwrap();
        load_game_scripts(&lua, &config.scripts_dir).unwrap();

        let val: i64 = lua.load("return loaded_value").eval().unwrap();
        assert_eq!(val, 99);
    }
}
