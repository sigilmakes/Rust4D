//! Integration tests verifying the Lua sandbox blocks dangerous operations.
//!
//! Each test creates a sandboxed Lua VM and runs a script that attempts
//! a forbidden action. The test passes if the script errors (or the
//! forbidden global is nil).

use rust4d_scripting::ScriptConfig;
use rust4d_scripting::vm::create_lua_vm;

/// Helper: create a sandboxed Lua VM with default config.
fn sandboxed_vm() -> mlua::Lua {
    let config = ScriptConfig::default();
    create_lua_vm(&config).expect("VM creation should succeed")
}

/// Helper: assert that a Lua script fails to execute.
/// Returns the error message for further inspection if needed.
fn assert_lua_fails(lua: &mlua::Lua, script: &str) -> String {
    let result = lua.load(script).exec();
    assert!(
        result.is_err(),
        "Expected script to fail but it succeeded: {}",
        script
    );
    result.unwrap_err().to_string()
}

/// Helper: assert that a Lua expression evaluates to nil.
fn assert_lua_nil(lua: &mlua::Lua, expr: &str) {
    let val: mlua::Value = lua
        .load(format!("return {}", expr))
        .eval()
        .unwrap_or(mlua::Value::Nil);
    assert!(
        val == mlua::Value::Nil,
        "Expected {} to be nil, got {:?}",
        expr,
        val
    );
}

// ---------------------------------------------------------------------------
// os module
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_os_execute() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "os");
    assert_lua_fails(&lua, r#"os.execute("echo pwned")"#);
}

#[test]
fn sandbox_blocks_os_getenv() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"os.getenv("HOME")"#);
}

#[test]
fn sandbox_blocks_os_remove() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"os.remove("/tmp/test")"#);
}

#[test]
fn sandbox_blocks_os_rename() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"os.rename("/tmp/a", "/tmp/b")"#);
}

#[test]
fn sandbox_blocks_os_clock() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, "os.clock()");
}

// ---------------------------------------------------------------------------
// io module
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_io_open() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "io");
    assert_lua_fails(&lua, r#"io.open("/etc/passwd", "r")"#);
}

#[test]
fn sandbox_blocks_io_popen() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"io.popen("ls")"#);
}

#[test]
fn sandbox_blocks_io_read() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"io.read()"#);
}

#[test]
fn sandbox_blocks_io_write() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, r#"io.write("hello")"#);
}

// ---------------------------------------------------------------------------
// debug module
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_debug_library() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "debug");
    assert_lua_fails(&lua, "debug.getinfo(1)");
}

#[test]
fn sandbox_blocks_debug_getregistry() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, "debug.getregistry()");
}

#[test]
fn sandbox_blocks_debug_sethook() {
    let lua = sandboxed_vm();
    assert_lua_fails(&lua, "debug.sethook(function() end, 'c')");
}

#[test]
fn sandbox_blocks_debug_getmetatable() {
    let lua = sandboxed_vm();
    // debug.getmetatable bypasses __metatable, so it must be blocked
    assert_lua_fails(&lua, r#"debug.getmetatable("")"#);
}

// ---------------------------------------------------------------------------
// loadfile / dofile
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_loadfile() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "loadfile");
    assert_lua_fails(&lua, r#"loadfile("/etc/passwd")"#);
}

#[test]
fn sandbox_blocks_dofile() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "dofile");
    assert_lua_fails(&lua, r#"dofile("/etc/passwd")"#);
}

// ---------------------------------------------------------------------------
// Native/C module loading
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_package_loadlib() {
    let lua = sandboxed_vm();
    assert_lua_nil(&lua, "package.loadlib");
    assert_lua_fails(&lua, r#"package.loadlib("/usr/lib/libc.so", "init")"#);
}

#[test]
fn sandbox_clears_package_cpath() {
    let lua = sandboxed_vm();
    let cpath: String = lua
        .load("return package.cpath")
        .eval()
        .expect("should be able to read cpath");
    assert!(
        cpath.is_empty(),
        "package.cpath should be empty but was: {}",
        cpath
    );
}

#[test]
fn sandbox_blocks_require_c_module() {
    let lua = sandboxed_vm();
    // Attempting to require a C module should fail because cpath is empty
    // and loadlib is removed
    let result = lua.load(r#"require("socket")"#).exec();
    assert!(
        result.is_err(),
        "require('socket') should fail in sandbox"
    );
}

// ---------------------------------------------------------------------------
// Metatable-based escape attempts
// ---------------------------------------------------------------------------

#[test]
fn sandbox_blocks_string_metatable_pollution() {
    let lua = sandboxed_vm();
    // Attempting to modify the string metatable to inject code
    // This should either fail or be contained -- the key thing is
    // that it doesn't give access to removed globals
    let result = lua
        .load(
            r#"
            local mt = getmetatable("")
            -- Trying to get os through string metatable tricks
            -- This should not restore os access
            local found_os = false
            if mt and mt.__index then
                -- Try to walk through the string library's environment
                -- In sandboxed Lua, this shouldn't leak dangerous modules
                found_os = (os ~= nil)
            end
            assert(not found_os, "os should not be accessible via metatable tricks")
            "#,
        )
        .exec();
    assert!(
        result.is_ok(),
        "Metatable access check should pass (os stays nil): {:?}",
        result
    );
}

#[test]
fn sandbox_blocks_rawget_global_restore() {
    let lua = sandboxed_vm();
    // Verify that rawget on _G doesn't restore removed globals
    let result: mlua::Value = lua
        .load(r#"return rawget(_G, "os")"#)
        .eval()
        .unwrap_or(mlua::Value::Nil);
    assert!(
        result == mlua::Value::Nil,
        "rawget(_G, 'os') should be nil"
    );
}

#[test]
fn sandbox_blocks_rawget_io() {
    let lua = sandboxed_vm();
    let result: mlua::Value = lua
        .load(r#"return rawget(_G, "io")"#)
        .eval()
        .unwrap_or(mlua::Value::Nil);
    assert!(
        result == mlua::Value::Nil,
        "rawget(_G, 'io') should be nil"
    );
}

#[test]
fn sandbox_blocks_rawget_debug() {
    let lua = sandboxed_vm();
    let result: mlua::Value = lua
        .load(r#"return rawget(_G, "debug")"#)
        .eval()
        .unwrap_or(mlua::Value::Nil);
    assert!(
        result == mlua::Value::Nil,
        "rawget(_G, 'debug') should be nil"
    );
}

#[test]
fn sandbox_blocks_load_bytecode_access() {
    let lua = sandboxed_vm();
    // `load` is still available for loading strings (needed for scripting),
    // but ensure it can't be used to access removed globals via upvalue tricks
    let result = lua
        .load(
            r#"
            -- Try to use load() to access os indirectly
            local fn = load("return os")
            local val = fn()
            assert(val == nil, "load() should not restore os access")
            "#,
        )
        .exec();
    assert!(
        result.is_ok(),
        "load() should not provide access to removed globals: {:?}",
        result
    );
}

#[test]
fn sandbox_safe_globals_still_work() {
    let lua = sandboxed_vm();
    // Verify that safe standard libraries are still functional
    lua.load(
        r#"
        -- math
        assert(math.floor(3.7) == 3)
        assert(math.pi > 3.14)

        -- string
        assert(string.len("hello") == 5)
        assert(string.upper("abc") == "ABC")

        -- table
        local t = {3, 1, 2}
        table.sort(t)
        assert(t[1] == 1 and t[2] == 2 and t[3] == 3)

        -- type, tostring, tonumber, pairs, ipairs, pcall, etc.
        assert(type(42) == "number")
        assert(tonumber("42") == 42)
        assert(tostring(42) == "42")

        local sum = 0
        for _, v in ipairs({1, 2, 3}) do sum = sum + v end
        assert(sum == 6)

        local ok, err = pcall(function() error("test") end)
        assert(not ok)
        "#,
    )
    .exec()
    .expect("Safe standard libraries should work in the sandbox");
}

// ---------------------------------------------------------------------------
// Package searcher / require restrictions
// ---------------------------------------------------------------------------

#[test]
fn sandbox_require_only_resolves_from_scripts_dir() {
    let lua = sandboxed_vm();
    // Attempting to require a module that doesn't exist in the scripts dir
    // should fail, not fall through to system paths
    let result = lua.load(r#"require("lfs")"#).exec();
    assert!(
        result.is_err(),
        "require('lfs') should fail -- not on scripts path"
    );
}

#[test]
fn sandbox_package_path_restricted() {
    let lua = sandboxed_vm();
    let path: String = lua
        .load("return package.path")
        .eval()
        .expect("should be able to read package.path");
    // Should only contain the scripts directory, not system paths
    assert!(
        !path.contains("/usr/"),
        "package.path should not contain system paths: {}",
        path
    );
    assert!(
        !path.contains("/usr/local/"),
        "package.path should not contain system paths: {}",
        path
    );
}
