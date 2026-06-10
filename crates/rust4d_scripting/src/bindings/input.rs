//! Input bindings for Lua
//!
//! Provides Lua access to input polling:
//! - `input.is_key_pressed(key_name)` - Check if a key is held
//! - `input.is_key_just_pressed(key_name)` - Check if a key was pressed this frame
//! - `input.is_action_pressed(action_name)` - Check if a mapped action is active
//! - `input.is_action_just_pressed(action_name)` - Check if an action was triggered this frame
//! - `input.get_axis(positive_key, negative_key)` - Get axis value from key pair
//! - `input.mouse_delta()` - Get mouse movement since last frame
//!
//! ## Usage (Lua)
//!
//! ```lua
//! -- Check keys directly
//! if input.is_key_pressed("W") then
//!     player:move_forward()
//! end
//!
//! -- Check for just-pressed (rising edge)
//! if input.is_key_just_pressed("Space") then
//!     player:jump()
//! end
//!
//! -- Use mapped actions (recommended)
//! if input.is_action_just_pressed("attack") then
//!     player:attack()
//! end
//!
//! -- Get axis values for smooth movement
//! local forward = input.get_axis("W", "S")  -- -1 to 1
//! local strafe = input.get_axis("D", "A")   -- -1 to 1
//!
//! -- Get mouse delta for look
//! local dx, dy = input.mouse_delta()
//! camera:rotate(dx * sensitivity, dy * sensitivity)
//! ```
//!
//! ## Design Note
//!
//! The actual InputState lives in the engine, not in Lua. This implementation provides
//! the binding API structure with stub operations that log warnings and return defaults.
//! Full integration with the engine's InputState happens when the engine binary wires up
//! the scripting system via `lua.set_app_data()`.
//!
//! This module is owned by Agent D3 (Input/Audio Bindings).

use std::sync::atomic::{AtomicBool, Ordering};

use mlua::prelude::*;

/// Track whether we've logged the "input not connected" warning.
static INPUT_WARNED: AtomicBool = AtomicBool::new(false);

/// Known valid key names for validation.
/// These correspond to common keyboard keys that would be handled by winit/gilrs.
const VALID_KEY_NAMES: &[&str] = &[
    // Letters
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    // Numbers
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    // Function keys
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    // Modifiers
    "Shift",
    "LShift",
    "RShift",
    "Control",
    "LControl",
    "RControl",
    "Ctrl",
    "LCtrl",
    "RCtrl",
    "Alt",
    "LAlt",
    "RAlt",
    "Super",
    "LSuper",
    "RSuper",
    "Win",
    "LWin",
    "RWin",
    // Special keys
    "Space",
    "Enter",
    "Return",
    "Escape",
    "Esc",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    // Punctuation and symbols
    "Minus",
    "Plus",
    "Equals",
    "LeftBracket",
    "RightBracket",
    "LBracket",
    "RBracket",
    "Backslash",
    "Semicolon",
    "Quote",
    "Apostrophe",
    "Comma",
    "Period",
    "Slash",
    "Grave",
    "Backtick",
    "Tilde",
    // Numpad
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadSubtract",
    "NumpadMultiply",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadDecimal",
    // Other
    "CapsLock",
    "NumLock",
    "ScrollLock",
    "PrintScreen",
    "Pause",
];

/// Validate a key name and return it normalized (uppercase for letters).
/// Returns an error if the key name is not recognized.
fn validate_key_name(key: &str) -> LuaResult<()> {
    // Check if it matches any known key (case-insensitive for letters)
    let key_upper = key.to_uppercase();

    for &valid in VALID_KEY_NAMES {
        if valid.eq_ignore_ascii_case(key) || valid.to_uppercase() == key_upper {
            return Ok(());
        }
    }

    // Not a recognized key - warn but don't error (might be a valid key we don't know about)
    log::warn!(
        "[input] Unknown key name '{}'. This may be a typo. Known keys include: \
         A-Z, 0-9, F1-F12, Space, Enter, Escape, Shift, Control, Alt, Arrow keys, etc.",
        key
    );
    Ok(())
}

/// Known valid mouse button names.
const VALID_MOUSE_BUTTONS: &[&str] = &["left", "right", "middle", "mouse1", "mouse2", "mouse3"];

/// Validate a mouse button name.
fn validate_mouse_button(button: &str) -> LuaResult<()> {
    let lower = button.to_lowercase();
    if !VALID_MOUSE_BUTTONS.contains(&lower.as_str()) {
        return Err(LuaError::RuntimeError(format!(
            "Invalid mouse button '{}'. Valid buttons: left, right, middle",
            button
        )));
    }
    Ok(())
}

/// Register input bindings with the Lua VM
///
/// Creates a global `input` table with the following functions:
/// - `input.is_key_pressed(key_name: string) -> bool` - Check if key is held
/// - `input.is_key_just_pressed(key_name: string) -> bool` - Check if key was just pressed
/// - `input.is_action_pressed(action_name: string) -> bool` - Check mapped action
/// - `input.is_action_just_pressed(action_name: string) -> bool` - Check action rising edge
/// - `input.get_axis(positive_key: string, negative_key: string) -> float` - Get axis value
/// - `input.mouse_delta() -> dx, dy` - Get mouse movement (two return values)
///
/// # Stub Implementation
///
/// These functions are currently stubs that log warnings but return sensible defaults.
/// Full integration requires the engine to provide InputState via `lua.set_app_data()`.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let input_table = lua.create_table()?;

    // input.is_key_pressed(key_name: string) -> bool
    //
    // Check if a key is currently held down.
    //
    // Arguments:
    // - key_name: The name of the key (e.g., "W", "A", "S", "D", "Space", "Shift")
    //
    // Returns:
    // - true if the key is currently held, false otherwise
    input_table.set(
        "is_key_pressed",
        lua.create_function(|_, key: String| {
            // Validate key name (MEDIUM-14)
            validate_key_name(&key)?;

            // Log warning only on first call (LOW-6)
            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero. \
                     This is expected during development but indicates missing engine integration."
                );
            }

            // STUB: Return false
            // Real implementation would:
            // 1. Get InputState from lua.app_data()
            // 2. Parse key name to KeyCode
            // 3. Return input_state.is_key_pressed(key_code)
            log::trace!("[input] is_key_pressed('{}') - stub returning false", key);
            Ok(false)
        })?,
    )?;

    // input.is_key_just_pressed(key_name: string) -> bool
    //
    // Check if a key was pressed this frame (rising edge detection).
    //
    // Arguments:
    // - key_name: The name of the key
    //
    // Returns:
    // - true if the key was just pressed this frame, false otherwise
    input_table.set(
        "is_key_just_pressed",
        lua.create_function(|_, key: String| {
            validate_key_name(&key)?;

            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return false
            log::trace!(
                "[input] is_key_just_pressed('{}') - stub returning false",
                key
            );
            Ok(false)
        })?,
    )?;

    // input.is_action_pressed(action_name: string) -> bool
    //
    // Check if a mapped action is currently active.
    // Actions are defined in the game's input configuration.
    //
    // Arguments:
    // - action_name: The name of the action (e.g., "jump", "attack", "move_forward")
    //
    // Returns:
    // - true if any key/button mapped to this action is pressed, false otherwise
    input_table.set(
        "is_action_pressed",
        lua.create_function(|_, action: String| {
            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return false
            // Action names are user-defined, so we don't validate them
            log::trace!(
                "[input] is_action_pressed('{}') - stub returning false",
                action
            );
            Ok(false)
        })?,
    )?;

    // input.is_action_just_pressed(action_name: string) -> bool
    //
    // Check if a mapped action was triggered this frame (rising edge).
    //
    // Arguments:
    // - action_name: The name of the action
    //
    // Returns:
    // - true if the action was just triggered, false otherwise
    input_table.set(
        "is_action_just_pressed",
        lua.create_function(|_, action: String| {
            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return false
            log::trace!(
                "[input] is_action_just_pressed('{}') - stub returning false",
                action
            );
            Ok(false)
        })?,
    )?;

    // input.get_axis(positive_key: string, negative_key: string) -> float
    //
    // Get an axis value based on two opposing keys.
    // Returns 1.0 if positive key is pressed, -1.0 if negative key is pressed,
    // 0.0 if neither or both are pressed.
    //
    // Arguments:
    // - positive_key: Key that contributes +1.0 (e.g., "W" or "D")
    // - negative_key: Key that contributes -1.0 (e.g., "S" or "A")
    //
    // Returns:
    // - float in range [-1.0, 1.0]
    input_table.set(
        "get_axis",
        lua.create_function(|_, (positive, negative): (String, String)| {
            validate_key_name(&positive)?;
            validate_key_name(&negative)?;

            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return 0.0
            log::trace!(
                "[input] get_axis('{}', '{}') - stub returning 0.0",
                positive,
                negative
            );
            Ok(0.0f32)
        })?,
    )?;

    // input.mouse_delta() -> dx, dy
    //
    // Get mouse movement since the last frame.
    //
    // Returns:
    // - dx: Horizontal mouse movement (positive = right)
    // - dy: Vertical mouse movement (positive = down)
    input_table.set(
        "mouse_delta",
        lua.create_function(|_, ()| {
            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return (0.0, 0.0)
            log::trace!("[input] mouse_delta() - stub returning (0.0, 0.0)");
            Ok((0.0f32, 0.0f32))
        })?,
    )?;

    // input.mouse_position() -> x, y
    //
    // Get current mouse position in window coordinates.
    //
    // Returns:
    // - x: Horizontal position (0 = left edge)
    // - y: Vertical position (0 = top edge)
    input_table.set(
        "mouse_position",
        lua.create_function(|_, ()| {
            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            // STUB: Return (0.0, 0.0)
            log::trace!("[input] mouse_position() - stub returning (0.0, 0.0)");
            Ok((0.0f32, 0.0f32))
        })?,
    )?;

    // input.is_mouse_button_pressed(button: string) -> bool
    //
    // Check if a mouse button is currently held.
    //
    // Arguments:
    // - button: "left", "right", or "middle"
    //
    // Returns:
    // - true if the button is pressed, false otherwise
    input_table.set(
        "is_mouse_button_pressed",
        lua.create_function(|_, button: String| {
            validate_mouse_button(&button)?;

            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            log::trace!(
                "[input] is_mouse_button_pressed('{}') - stub returning false",
                button
            );
            Ok(false)
        })?,
    )?;

    // input.is_mouse_button_just_pressed(button: string) -> bool
    //
    // Check if a mouse button was pressed this frame.
    //
    // Arguments:
    // - button: "left", "right", or "middle"
    //
    // Returns:
    // - true if the button was just pressed, false otherwise
    input_table.set(
        "is_mouse_button_just_pressed",
        lua.create_function(|_, button: String| {
            validate_mouse_button(&button)?;

            if !INPUT_WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "[input] InputState not connected - all input queries will return false/zero."
                );
            }

            log::trace!(
                "[input] is_mouse_button_just_pressed('{}') - stub returning false",
                button
            );
            Ok(false)
        })?,
    )?;

    // Register the input table as a global
    lua.globals().set("input", input_table)?;

    log::debug!("[input] Input bindings registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_lua_with_input() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register input bindings");
        lua
    }

    #[test]
    fn test_input_table_exists() {
        let lua = create_lua_with_input();
        let input: LuaTable = lua
            .globals()
            .get("input")
            .expect("input table should exist");
        assert!(input.contains_key("is_key_pressed").unwrap());
        assert!(input.contains_key("is_key_just_pressed").unwrap());
        assert!(input.contains_key("is_action_pressed").unwrap());
        assert!(input.contains_key("is_action_just_pressed").unwrap());
        assert!(input.contains_key("get_axis").unwrap());
        assert!(input.contains_key("mouse_delta").unwrap());
        assert!(input.contains_key("mouse_position").unwrap());
        assert!(input.contains_key("is_mouse_button_pressed").unwrap());
        assert!(input.contains_key("is_mouse_button_just_pressed").unwrap());
    }

    #[test]
    fn test_is_key_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_key_pressed('W')")
            .eval()
            .expect("is_key_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_is_key_just_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_key_just_pressed('Space')")
            .eval()
            .expect("is_key_just_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_is_action_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_action_pressed('jump')")
            .eval()
            .expect("is_action_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_is_action_just_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_action_just_pressed('attack')")
            .eval()
            .expect("is_action_just_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_get_axis_returns_zero() {
        let lua = create_lua_with_input();
        let result: f32 = lua
            .load("return input.get_axis('W', 'S')")
            .eval()
            .expect("get_axis should work");
        assert_eq!(result, 0.0, "stub should return 0.0");
    }

    #[test]
    fn test_mouse_delta_returns_two_values() {
        let lua = create_lua_with_input();
        lua.load(
            r#"
            local dx, dy = input.mouse_delta()
            assert(type(dx) == 'number', 'dx should be number')
            assert(type(dy) == 'number', 'dy should be number')
            assert(dx == 0.0, 'dx stub should be 0')
            assert(dy == 0.0, 'dy stub should be 0')
        "#,
        )
        .exec()
        .expect("mouse_delta should return two numbers");
    }

    #[test]
    fn test_mouse_position_returns_two_values() {
        let lua = create_lua_with_input();
        lua.load(
            r#"
            local x, y = input.mouse_position()
            assert(type(x) == 'number', 'x should be number')
            assert(type(y) == 'number', 'y should be number')
        "#,
        )
        .exec()
        .expect("mouse_position should return two numbers");
    }

    #[test]
    fn test_is_mouse_button_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_mouse_button_pressed('left')")
            .eval()
            .expect("is_mouse_button_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_is_mouse_button_just_pressed_returns_false() {
        let lua = create_lua_with_input();
        let result: bool = lua
            .load("return input.is_mouse_button_just_pressed('right')")
            .eval()
            .expect("is_mouse_button_just_pressed should work");
        assert!(!result, "stub should return false");
    }

    #[test]
    fn test_input_functions_callable_from_lua() {
        let lua = create_lua_with_input();
        // Test that all functions can be called without error
        lua.load(
            r#"
            -- Key checks
            local pressed = input.is_key_pressed('W')
            local just = input.is_key_just_pressed('Space')

            -- Action checks
            local action = input.is_action_pressed('move_forward')
            local action_just = input.is_action_just_pressed('jump')

            -- Axis
            local forward = input.get_axis('W', 'S')
            local strafe = input.get_axis('D', 'A')

            -- Mouse
            local dx, dy = input.mouse_delta()
            local mx, my = input.mouse_position()
            local lmb = input.is_mouse_button_pressed('left')
            local rmb_just = input.is_mouse_button_just_pressed('right')

            -- All should be their default values
            assert(pressed == false)
            assert(just == false)
            assert(action == false)
            assert(action_just == false)
            assert(forward == 0.0)
            assert(strafe == 0.0)
            assert(dx == 0.0)
            assert(dy == 0.0)
            assert(lmb == false)
            assert(rmb_just == false)
        "#,
        )
        .exec()
        .expect("All input functions should be callable");
    }
}
