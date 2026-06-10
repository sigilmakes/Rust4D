//! Input mapping from raw events to semantic actions
//!
//! Maps keyboard and mouse input to high-level actions like ToggleCursor, Exit, etc.
//! Movement keys (WASD, Space) are NOT mapped here - they go directly to CameraController.

use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

/// Actions triggered by special input (not movement)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Toggle cursor capture (Escape when captured, click when released)
    ToggleCursor,
    /// Exit application (Escape when not captured)
    Exit,
    /// Reset camera to starting position (R key)
    ResetCamera,
    /// Toggle fullscreen mode (F key)
    ToggleFullscreen,
    /// Toggle input smoothing (G key)
    ToggleSmoothing,
}

/// Maps raw input events to semantic actions
///
/// Movement keys (WASD, Space, RF) are NOT mapped here - they go directly
/// to the CameraController. This mapper handles "special" keys only.
pub struct InputMapper;

impl InputMapper {
    /// Map keyboard input to an action
    ///
    /// Returns `Some(action)` for special keys, `None` for movement keys
    pub fn map_keyboard(
        key: KeyCode,
        state: ElementState,
        cursor_captured: bool,
    ) -> Option<InputAction> {
        // Only handle key presses, not releases
        if state != ElementState::Pressed {
            return None;
        }

        match key {
            KeyCode::Escape => {
                if cursor_captured {
                    Some(InputAction::ToggleCursor)
                } else {
                    Some(InputAction::Exit)
                }
            }
            KeyCode::KeyR => Some(InputAction::ResetCamera),
            KeyCode::KeyF => Some(InputAction::ToggleFullscreen),
            KeyCode::KeyG => Some(InputAction::ToggleSmoothing),
            _ => None, // Movement keys handled by controller
        }
    }

    /// Map mouse button to an action
    ///
    /// Returns `Some(ToggleCursor)` for left click when cursor not captured
    pub fn map_mouse_button(
        button: MouseButton,
        state: ElementState,
        cursor_captured: bool,
    ) -> Option<InputAction> {
        if button == MouseButton::Left && state == ElementState::Pressed && !cursor_captured {
            Some(InputAction::ToggleCursor)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_when_captured_releases() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            true, // cursor captured
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }

    #[test]
    fn test_escape_when_released_exits() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            false, // cursor not captured
        );
        assert_eq!(action, Some(InputAction::Exit));
    }

    #[test]
    fn test_movement_keys_not_mapped() {
        // WASD should return None (handled by controller)
        for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
            let action = InputMapper::map_keyboard(key, ElementState::Pressed, true);
            assert_eq!(action, None, "Key {:?} should not be mapped", key);
        }
    }

    #[test]
    fn test_key_release_ignored() {
        let action = InputMapper::map_keyboard(KeyCode::Escape, ElementState::Released, true);
        assert_eq!(action, None);
    }

    #[test]
    fn test_click_to_capture() {
        let action = InputMapper::map_mouse_button(
            MouseButton::Left,
            ElementState::Pressed,
            false, // cursor not captured
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }

    #[test]
    fn test_click_when_captured_no_action() {
        let action = InputMapper::map_mouse_button(
            MouseButton::Left,
            ElementState::Pressed,
            true, // cursor already captured
        );
        assert_eq!(action, None);
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyR, ElementState::Pressed, true),
            Some(InputAction::ResetCamera)
        );
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyF, ElementState::Pressed, true),
            Some(InputAction::ToggleFullscreen)
        );
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyG, ElementState::Pressed, true),
            Some(InputAction::ToggleSmoothing)
        );
    }
}
