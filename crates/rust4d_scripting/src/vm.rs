//! Lua VM initialization and configuration

use mlua::prelude::*;
use crate::error::ScriptError;

/// Configuration for the scripting engine
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Root directory for game scripts
    pub scripts_dir: String,
    /// Whether to enable hot-reload file watching
    pub hot_reload: bool,
    /// Memory limit for the Lua VM in bytes (0 = unlimited).
    pub memory_limit: usize,
    /// Instruction count limit per call (0 = unlimited, for sandboxing).
    pub instruction_limit: u32,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            scripts_dir: "scripts".to_string(),
            hot_reload: cfg!(debug_assertions),
            memory_limit: 64 * 1024 * 1024, // 64MB
            instruction_limit: 0,
        }
    }
}

impl ScriptConfig {
    /// Create config with a specific scripts directory
    pub fn with_scripts_dir(dir: impl Into<String>) -> Self {
        Self {
            scripts_dir: dir.into(),
            ..Default::default()
        }
    }
}

/// Initialize a Lua VM with engine configuration.
///
/// The VM is sandboxed: dangerous globals and libraries are removed.
/// Specifically: `os`, `io`, `debug`, `loadfile`, and `dofile` are removed.
/// `package.cpath` is cleared and `package.loadlib` is removed to prevent
/// loading native C modules. `print()` is replaced with a version that
/// routes to `log::info!`. `package.path` is configured to resolve
/// `require()` from the scripts directory only.
pub fn create_lua_vm(config: &ScriptConfig) -> Result<Lua, ScriptError> {
    let lua = Lua::new();

    // Configure package paths for require() resolution.
    // Use the table API to avoid Lua code injection from special characters in the path.
    {
        let globals = lua.globals();
        let package: LuaTable = globals.get("package").map_err(ScriptError::LuaError)?;
        let scripts_dir = config.scripts_dir.replace('\\', "/");
        let path = format!("{0}/?.lua;{0}/?/init.lua", scripts_dir);
        package.set("path", path).map_err(ScriptError::LuaError)?;
        // Clear cpath to prevent loading native C modules from system paths
        package.set("cpath", "").map_err(ScriptError::LuaError)?;
        // Remove loadlib which can load arbitrary shared libraries
        package.set("loadlib", LuaNil).map_err(ScriptError::LuaError)?;
    }

    // Remove dangerous standard library modules for sandboxing
    let globals = lua.globals();
    globals.set("os", LuaNil).map_err(ScriptError::LuaError)?;
    globals.set("io", LuaNil).map_err(ScriptError::LuaError)?;
    globals.set("debug", LuaNil).map_err(ScriptError::LuaError)?;
    globals
        .set("loadfile", LuaNil)
        .map_err(ScriptError::LuaError)?;
    globals
        .set("dofile", LuaNil)
        .map_err(ScriptError::LuaError)?;

    // Replace print() with engine-aware version that routes to log::info
    let print_fn = lua
        .create_function(|_, args: LuaMultiValue| {
            let parts: Vec<String> = args
                .iter()
                .map(|v| match v {
                    LuaValue::Nil => "nil".to_string(),
                    LuaValue::Boolean(b) => b.to_string(),
                    LuaValue::Integer(n) => n.to_string(),
                    LuaValue::Number(n) => n.to_string(),
                    LuaValue::String(s) => {
                        match s.to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "<invalid utf8>".to_string(),
                        }
                    }
                    other => format!("{:?}", other),
                })
                .collect();
            log::info!("[lua] {}", parts.join("\t"));
            Ok(())
        })
        .map_err(ScriptError::LuaError)?;
    globals
        .set("print", print_fn)
        .map_err(ScriptError::LuaError)?;

    // Wire up memory limit if configured
    if config.memory_limit > 0 {
        lua.set_memory_limit(config.memory_limit)
            .map_err(ScriptError::LuaError)?;
    }

    // Wire up instruction counting hook if configured
    if config.instruction_limit > 0 {
        let limit = config.instruction_limit;
        let count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = count.clone();
        lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(1000),
            move |_lua, _debug| {
                let prev = count_clone.fetch_add(1000, std::sync::atomic::Ordering::Relaxed);
                if prev + 1000 > limit {
                    return Err(mlua::Error::RuntimeError(
                        format!("instruction limit exceeded ({})", limit),
                    ));
                }
                Ok(mlua::VmState::Continue)
            },
        );
        // Store the counter on the Lua VM so callers can reset it between calls
        lua.set_app_data(count);
    }

    Ok(lua)
}

/// Check if a global exists in the Lua VM (test utility)
#[cfg(test)]
pub fn has_global(lua: &Lua, name: &str) -> LuaResult<bool> {
    let globals = lua.globals();
    let value: LuaValue = globals.get(name)?;
    Ok(!matches!(value, LuaNil))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_scripts_dir() {
        let config = ScriptConfig::with_scripts_dir("/custom/path");
        assert_eq!(config.scripts_dir, "/custom/path");
        // Other fields should be defaults
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
    }

    #[test]
    fn test_vm_creates_with_default_config() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).expect("VM should create");
        // Verify we can execute basic Lua
        lua.load("local x = 1 + 1").exec().unwrap();
    }

    #[test]
    fn test_sandboxed_globals_removed() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();
        let globals = lua.globals();

        // These should be nil (sandboxed)
        assert!(globals.get::<LuaValue>("os").unwrap() == LuaNil);
        assert!(globals.get::<LuaValue>("io").unwrap() == LuaNil);
        assert!(globals.get::<LuaValue>("debug").unwrap() == LuaNil);
        assert!(globals.get::<LuaValue>("loadfile").unwrap() == LuaNil);
        assert!(globals.get::<LuaValue>("dofile").unwrap() == LuaNil);

        // package.cpath should be empty, package.loadlib should be nil
        let package: LuaTable = globals.get("package").unwrap();
        let cpath: String = package.get("cpath").unwrap();
        assert!(cpath.is_empty(), "package.cpath should be empty");
        assert!(package.get::<LuaValue>("loadlib").unwrap() == LuaNil);

        // These should still exist
        assert!(globals.get::<LuaValue>("math").unwrap() != LuaNil);
        assert!(globals.get::<LuaValue>("string").unwrap() != LuaNil);
        assert!(globals.get::<LuaValue>("table").unwrap() != LuaNil);
        assert!(globals.get::<LuaValue>("coroutine").unwrap() != LuaNil);
    }

    #[test]
    fn test_print_does_not_panic() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();
        // Should not panic even though log isn't fully initialized
        lua.load(r#"print("hello", 42, true, nil)"#)
            .exec()
            .unwrap();
    }

    #[test]
    fn test_package_path_configured() {
        let config = ScriptConfig {
            scripts_dir: "/tmp/test_scripts".to_string(),
            ..Default::default()
        };
        let lua = create_lua_vm(&config).unwrap();
        let path: String = lua
            .load("return package.path")
            .eval()
            .unwrap();
        assert!(path.contains("/tmp/test_scripts/?.lua"));
    }

    #[test]
    fn test_has_global_true_for_existing() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();
        // math should exist
        assert!(has_global(&lua, "math").unwrap());
    }

    #[test]
    fn test_has_global_false_for_sandboxed() {
        let config = ScriptConfig::default();
        let lua = create_lua_vm(&config).unwrap();
        // os should be sandboxed (nil)
        assert!(!has_global(&lua, "os").unwrap());
    }
}
