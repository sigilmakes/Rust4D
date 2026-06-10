//! Semantic input actions and key bindings.
//!
//! `CameraController` should not know that "forward" means the W key or that
//! "ana" means Q. Those are bindings, not behavior. [`ActionMap`] keeps the
//! default Rust4D controls while allowing examples, config files, and future
//! UI editors to rebind movement without touching camera math.

use winit::keyboard::KeyCode;

/// High-level camera/controller actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraAction {
    /// Move forward in the current slice plane
    MoveForward,
    /// Move backward in the current slice plane
    MoveBackward,
    /// Strafe left in the current slice plane
    MoveLeft,
    /// Strafe right in the current slice plane
    MoveRight,
    /// Move upward along world/camera Y
    MoveUp,
    /// Move downward along world/camera Y
    MoveDown,
    /// Move ana (+W in camera-local 4D space)
    MoveAna,
    /// Move kata (-W in camera-local 4D space)
    MoveKata,
}

/// Key binding table for camera actions.
///
/// Multiple keys may trigger the same action, and one key may intentionally
/// trigger multiple actions (e.g. Space currently means both upward movement
/// and a one-shot jump in physics mode, handled by `CameraController`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionMap {
    bindings: Vec<(KeyCode, CameraAction)>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self {
            bindings: vec![
                (KeyCode::KeyW, CameraAction::MoveForward),
                (KeyCode::KeyS, CameraAction::MoveBackward),
                (KeyCode::KeyA, CameraAction::MoveLeft),
                (KeyCode::KeyD, CameraAction::MoveRight),
                (KeyCode::Space, CameraAction::MoveUp),
                (KeyCode::ShiftLeft, CameraAction::MoveDown),
                (KeyCode::ShiftRight, CameraAction::MoveDown),
                (KeyCode::KeyQ, CameraAction::MoveAna),
                (KeyCode::KeyE, CameraAction::MoveKata),
            ],
        }
    }
}

impl ActionMap {
    /// Create the default Rust4D camera bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty action map.
    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Bind `key` to `action`, keeping any existing bindings.
    pub fn bind(&mut self, key: KeyCode, action: CameraAction) {
        if !self.bindings.contains(&(key, action)) {
            self.bindings.push((key, action));
        }
    }

    /// Remove every action bound to `key`.
    pub fn unbind_key(&mut self, key: KeyCode) {
        self.bindings.retain(|(bound_key, _)| *bound_key != key);
    }

    /// Remove this exact key/action binding.
    pub fn unbind(&mut self, key: KeyCode, action: CameraAction) {
        self.bindings
            .retain(|(bound_key, bound_action)| *bound_key != key || *bound_action != action);
    }

    /// Return the actions bound to `key` in insertion order.
    pub fn actions_for_key(&self, key: KeyCode) -> impl Iterator<Item = CameraAction> + '_ {
        self.bindings
            .iter()
            .filter_map(move |(bound_key, action)| (*bound_key == key).then_some(*action))
    }

    /// True if `key` is bound to any action.
    pub fn handles_key(&self, key: KeyCode) -> bool {
        self.bindings.iter().any(|(bound_key, _)| *bound_key == key)
    }

    /// Read-only binding list, useful for UI/debug display.
    pub fn bindings(&self) -> &[(KeyCode, CameraAction)] {
        &self.bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_match_legacy_controls() {
        let map = ActionMap::default();
        assert_eq!(
            map.actions_for_key(KeyCode::KeyW).collect::<Vec<_>>(),
            vec![CameraAction::MoveForward]
        );
        assert_eq!(
            map.actions_for_key(KeyCode::KeyQ).collect::<Vec<_>>(),
            vec![CameraAction::MoveAna]
        );
        assert_eq!(
            map.actions_for_key(KeyCode::ShiftRight).collect::<Vec<_>>(),
            vec![CameraAction::MoveDown]
        );
    }

    #[test]
    fn bind_and_unbind_key() {
        let mut map = ActionMap::empty();
        map.bind(KeyCode::ArrowUp, CameraAction::MoveForward);
        map.bind(KeyCode::ArrowUp, CameraAction::MoveForward);
        assert_eq!(map.bindings().len(), 1, "duplicate bindings are ignored");
        assert!(map.handles_key(KeyCode::ArrowUp));
        map.unbind_key(KeyCode::ArrowUp);
        assert!(!map.handles_key(KeyCode::ArrowUp));
    }

    #[test]
    fn one_key_can_drive_multiple_actions() {
        let mut map = ActionMap::empty();
        map.bind(KeyCode::KeyR, CameraAction::MoveForward);
        map.bind(KeyCode::KeyR, CameraAction::MoveAna);
        let actions: Vec<_> = map.actions_for_key(KeyCode::KeyR).collect();
        assert_eq!(
            actions,
            vec![CameraAction::MoveForward, CameraAction::MoveAna]
        );
    }
}
