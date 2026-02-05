//! Lifecycle callback dispatch
//!
//! Calls game lifecycle functions (on_init, on_update, etc.) if they exist.
//! Missing callbacks are silently ignored - it's not an error if a script
//! doesn't define a particular callback.

use crate::error::ScriptError;
use mlua::{Lua, Value};

/// Lifecycle callback names
pub mod callbacks {
    /// Called once when the game starts
    pub const ON_INIT: &str = "on_init";
    /// Called every frame with delta time
    pub const ON_UPDATE: &str = "on_update";
    /// Called at fixed intervals for physics
    pub const ON_FIXED_UPDATE: &str = "on_fixed_update";
    /// Called when the game is shutting down
    pub const ON_SHUTDOWN: &str = "on_shutdown";
}

/// Call a lifecycle callback if it exists
///
/// If the callback is not defined (nil), this returns Ok(()).
/// If the callback exists but fails, returns an error with context.
pub fn call_lifecycle(lua: &Lua, callback_name: &str) -> Result<(), ScriptError> {
    call_lifecycle_with_args(lua, callback_name, ())
}

/// Call a lifecycle callback with arguments if it exists
///
/// If the callback is not defined (nil), this returns Ok(()).
/// If the callback exists but fails, returns an error with context.
pub fn call_lifecycle_with_args<A>(
    lua: &Lua,
    callback_name: &str,
    args: A,
) -> Result<(), ScriptError>
where
    A: mlua::IntoLuaMulti,
{
    let globals = lua.globals();
    let callback: Value = globals.get(callback_name).map_err(|e| {
        ScriptError::LuaError {
            message: format!("Error accessing '{}': {}", callback_name, e),
            source: Some(e),
        }
    })?;

    // If callback doesn't exist, silently succeed
    if matches!(callback, Value::Nil) {
        return Ok(());
    }

    // Ensure it's a function
    let func = match callback {
        Value::Function(f) => f,
        _ => {
            return Err(ScriptError::RuntimeError {
                callback: callback_name.to_string(),
                message: format!(
                    "'{}' exists but is not a function (got {})",
                    callback_name,
                    type_name(&callback)
                ),
                source: None,
            });
        }
    };

    // Call the function
    func.call::<()>(args)
        .map_err(|e| ScriptError::runtime(callback_name, e))?;

    Ok(())
}

/// Get a human-readable type name for a Lua value
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Nil => "nil",
        Value::Boolean(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Table(_) => "table",
        Value::Function(_) => "function",
        Value::Thread(_) => "thread",
        Value::UserData(_) => "userdata",
        Value::LightUserData(_) => "lightuserdata",
        Value::Error(_) => "error",
        _ => "unknown",
    }
}

/// Check if a callback exists without calling it
pub fn has_callback(lua: &Lua, callback_name: &str) -> bool {
    lua.globals()
        .get::<Value>(callback_name)
        .map(|v| matches!(v, Value::Function(_)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_script_string;
    use crate::vm::{create_lua_vm, ScriptConfig};

    fn setup_test_vm() -> Lua {
        let config = ScriptConfig::default();
        create_lua_vm(&config).unwrap()
    }

    #[test]
    fn test_call_missing_callback_is_ok() {
        let lua = setup_test_vm();

        // No callbacks defined - should succeed silently
        let result = call_lifecycle(&lua, "on_init");
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_existing_callback() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            init_called = false
            function on_init()
                init_called = true
            end
        "#,
            "test.lua",
        )
        .unwrap();

        call_lifecycle(&lua, "on_init").unwrap();

        let called: bool = lua.globals().get("init_called").unwrap();
        assert!(called, "on_init should have been called");
    }

    #[test]
    fn test_call_update_with_dt() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            received_dt = 0
            function on_update(dt)
                received_dt = dt
            end
        "#,
            "test.lua",
        )
        .unwrap();

        call_lifecycle_with_args(&lua, "on_update", 0.016).unwrap();

        let dt: f64 = lua.globals().get("received_dt").unwrap();
        assert!((dt - 0.016).abs() < 0.0001, "dt should be passed correctly");
    }

    #[test]
    fn test_call_fixed_update_with_dt() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            received_dt = 0
            function on_fixed_update(dt)
                received_dt = dt
            end
        "#,
            "test.lua",
        )
        .unwrap();

        // Fixed update typically at 60fps = 1/60 seconds
        let fixed_dt = 1.0 / 60.0;
        call_lifecycle_with_args(&lua, "on_fixed_update", fixed_dt).unwrap();

        let dt: f64 = lua.globals().get("received_dt").unwrap();
        assert!((dt - fixed_dt).abs() < 0.0001);
    }

    #[test]
    fn test_runtime_error_in_callback() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            function on_update(dt)
                error("Something went wrong!")
            end
        "#,
            "test.lua",
        )
        .unwrap();

        let result = call_lifecycle_with_args(&lua, "on_update", 0.016);
        assert!(matches!(result, Err(ScriptError::RuntimeError { .. })));

        if let Err(ScriptError::RuntimeError { callback, message, .. }) = result {
            assert_eq!(callback, "on_update");
            assert!(message.contains("Something went wrong"));
        }
    }

    #[test]
    fn test_callback_is_not_function() {
        let lua = setup_test_vm();

        load_script_string(&lua, "on_init = 42", "test.lua").unwrap();

        let result = call_lifecycle(&lua, "on_init");
        assert!(matches!(result, Err(ScriptError::RuntimeError { .. })));

        if let Err(ScriptError::RuntimeError { message, .. }) = result {
            assert!(message.contains("not a function"));
            assert!(message.contains("integer"));
        }
    }

    #[test]
    fn test_has_callback_true() {
        let lua = setup_test_vm();

        load_script_string(&lua, "function on_init() end", "test.lua").unwrap();

        assert!(has_callback(&lua, "on_init"));
    }

    #[test]
    fn test_has_callback_false_missing() {
        let lua = setup_test_vm();

        assert!(!has_callback(&lua, "on_init"));
    }

    #[test]
    fn test_has_callback_false_not_function() {
        let lua = setup_test_vm();

        load_script_string(&lua, "on_init = 'not a function'", "test.lua").unwrap();

        assert!(!has_callback(&lua, "on_init"));
    }

    #[test]
    fn test_shutdown_callback() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            shutdown_called = false
            function on_shutdown()
                shutdown_called = true
            end
        "#,
            "test.lua",
        )
        .unwrap();

        call_lifecycle(&lua, "on_shutdown").unwrap();

        let called: bool = lua.globals().get("shutdown_called").unwrap();
        assert!(called);
    }

    #[test]
    fn test_callback_can_return_values() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            function on_init()
                return "initialized"
            end
        "#,
            "test.lua",
        )
        .unwrap();

        // Our call_lifecycle discards return values, but shouldn't error
        let result = call_lifecycle(&lua, "on_init");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_callbacks() {
        let lua = setup_test_vm();

        load_script_string(
            &lua,
            r#"
            call_order = {}
            function on_init()
                table.insert(call_order, "init")
            end
            function on_update(dt)
                table.insert(call_order, "update")
            end
            function on_shutdown()
                table.insert(call_order, "shutdown")
            end
        "#,
            "test.lua",
        )
        .unwrap();

        call_lifecycle(&lua, "on_init").unwrap();
        call_lifecycle_with_args(&lua, "on_update", 0.016).unwrap();
        call_lifecycle_with_args(&lua, "on_update", 0.016).unwrap();
        call_lifecycle(&lua, "on_shutdown").unwrap();

        let order: mlua::Table = lua.globals().get("call_order").unwrap();
        assert_eq!(order.len().unwrap(), 4);
        assert_eq!(order.get::<String>(1).unwrap(), "init");
        assert_eq!(order.get::<String>(2).unwrap(), "update");
        assert_eq!(order.get::<String>(3).unwrap(), "update");
        assert_eq!(order.get::<String>(4).unwrap(), "shutdown");
    }

    #[test]
    fn test_callbacks_module_constants() {
        // Verify the callback constants are correct
        assert_eq!(callbacks::ON_INIT, "on_init");
        assert_eq!(callbacks::ON_UPDATE, "on_update");
        assert_eq!(callbacks::ON_FIXED_UPDATE, "on_fixed_update");
        assert_eq!(callbacks::ON_SHUTDOWN, "on_shutdown");
    }
}
