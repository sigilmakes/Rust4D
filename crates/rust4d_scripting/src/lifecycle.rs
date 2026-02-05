//! Game loop lifecycle callback dispatch

use mlua::prelude::*;
use crate::error::ScriptError;

/// Check if a callback function exists without calling it
pub fn has_callback(lua: &Lua, callback_name: &str) -> bool {
    lua.globals()
        .get::<LuaValue>(callback_name)
        .map(|v| matches!(v, LuaValue::Function(_)))
        .unwrap_or(false)
}

/// Call a global Lua lifecycle function if it exists, silently ignoring missing functions.
///
/// If the function exists, it is called with the provided arguments.
/// If the function doesn't exist (global is nil), this returns Ok(()).
/// If the function exists but errors during execution, this returns a RuntimeError.
pub fn call_lifecycle(lua: &Lua, name: &str, args: impl IntoLuaMulti) -> Result<(), ScriptError> {
    let globals = lua.globals();

    match globals.get::<LuaValue>(name) {
        Ok(LuaValue::Function(func)) => {
            func.call::<()>(args).map_err(|e| ScriptError::RuntimeError {
                callback: name.to_string(),
                error: e,
            })?;
            Ok(())
        }
        Ok(LuaValue::Nil) => {
            // Function doesn't exist — that's fine, it's optional
            Ok(())
        }
        Ok(_) => {
            // Global exists but isn't a function — silently ignore
            Ok(())
        }
        Err(e) => Err(ScriptError::LuaError(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{create_lua_vm, ScriptConfig};

    fn test_lua() -> Lua {
        create_lua_vm(&ScriptConfig::default()).unwrap()
    }

    #[test]
    fn test_call_existing_callback() {
        let lua = test_lua();
        lua.load("function on_init() called_init = true end")
            .exec()
            .unwrap();

        call_lifecycle(&lua, "on_init", ()).unwrap();

        let called: bool = lua.load("return called_init").eval().unwrap();
        assert!(called);
    }

    #[test]
    fn test_missing_callback_is_ok() {
        let lua = test_lua();
        // on_init is not defined
        let result = call_lifecycle(&lua, "on_init", ());
        assert!(result.is_ok());
    }

    #[test]
    fn test_callback_receives_args() {
        let lua = test_lua();
        lua.load("function on_update(dt) received_dt = dt end")
            .exec()
            .unwrap();

        call_lifecycle(&lua, "on_update", 0.016_f32).unwrap();

        let dt: f32 = lua.load("return received_dt").eval().unwrap();
        assert!((dt - 0.016).abs() < 0.0001);
    }

    #[test]
    fn test_callback_runtime_error() {
        let lua = test_lua();
        lua.load("function on_update(dt) error('boom') end")
            .exec()
            .unwrap();

        let result = call_lifecycle(&lua, "on_update", 0.016_f32);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ScriptError::RuntimeError { .. }));
        // Check the callback name is captured
        if let ScriptError::RuntimeError { callback, .. } = &err {
            assert_eq!(callback, "on_update");
        }
    }

    #[test]
    fn test_non_function_global_is_ok() {
        let lua = test_lua();
        lua.load("on_init = 42").exec().unwrap(); // Not a function

        let result = call_lifecycle(&lua, "on_init", ());
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_callback_true_when_exists() {
        let lua = test_lua();
        lua.load("function on_init() end").exec().unwrap();

        assert!(has_callback(&lua, "on_init"));
    }

    #[test]
    fn test_has_callback_false_when_missing() {
        let lua = test_lua();
        assert!(!has_callback(&lua, "on_init"));
    }

    #[test]
    fn test_has_callback_false_for_non_function() {
        let lua = test_lua();
        lua.load("on_init = 42").exec().unwrap();

        assert!(!has_callback(&lua, "on_init"));
    }
}
