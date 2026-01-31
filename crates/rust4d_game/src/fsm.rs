//! Generic finite state machine
//!
//! A minimal state machine that tracks the current state and supports transitions.
//! Game-specific state logic (enter/exit callbacks, transition conditions) is left
//! to the consumer -- this just provides clean state tracking.

/// A generic finite state machine
///
/// Tracks the current state and provides transition methods. The state type `S`
/// must implement `Eq`, `Clone`, and `Debug`.
///
/// # Example
///
/// ```
/// use rust4d_game::StateMachine;
///
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// enum PlayerState {
///     Idle,
///     Walking,
///     Jumping,
///     Falling,
/// }
///
/// let mut fsm = StateMachine::new(PlayerState::Idle);
/// assert!(fsm.is_in(&PlayerState::Idle));
///
/// // Transition to walking
/// let changed = fsm.transition(PlayerState::Walking);
/// assert!(changed);
/// assert!(fsm.is_in(&PlayerState::Walking));
///
/// // Previous state is tracked
/// assert_eq!(fsm.previous(), Some(&PlayerState::Idle));
///
/// // Transition to same state returns false
/// let changed = fsm.transition(PlayerState::Walking);
/// assert!(!changed);
/// ```
pub struct StateMachine<S: Eq + Clone + std::fmt::Debug> {
    current: S,
    previous: Option<S>,
}

impl<S: Eq + Clone + std::fmt::Debug> StateMachine<S> {
    /// Create a new state machine with the given initial state
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            previous: None,
        }
    }

    /// Get a reference to the current state
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Get a reference to the previous state (before the last transition)
    ///
    /// Returns `None` if no transition has occurred yet.
    pub fn previous(&self) -> Option<&S> {
        self.previous.as_ref()
    }

    /// Transition to a new state
    ///
    /// Returns `true` if the state changed, `false` if already in that state.
    /// When the state changes, the old state is saved as `previous`.
    pub fn transition(&mut self, new_state: S) -> bool {
        if self.current != new_state {
            log::debug!("FSM transition: {:?} -> {:?}", self.current, new_state);
            let old = std::mem::replace(&mut self.current, new_state);
            self.previous = Some(old);
            true
        } else {
            false
        }
    }

    /// Check if the state machine is in the given state
    pub fn is_in(&self, state: &S) -> bool {
        &self.current == state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestState {
        Idle,
        Walking,
        Jumping,
        Falling,
    }

    #[test]
    fn test_new_initial_state() {
        let fsm = StateMachine::new(TestState::Idle);
        assert_eq!(fsm.current(), &TestState::Idle);
    }

    #[test]
    fn test_is_in() {
        let fsm = StateMachine::new(TestState::Idle);
        assert!(fsm.is_in(&TestState::Idle));
        assert!(!fsm.is_in(&TestState::Walking));
        assert!(!fsm.is_in(&TestState::Jumping));
    }

    #[test]
    fn test_transition_changes_state() {
        let mut fsm = StateMachine::new(TestState::Idle);
        let changed = fsm.transition(TestState::Walking);
        assert!(changed);
        assert_eq!(fsm.current(), &TestState::Walking);
        assert!(fsm.is_in(&TestState::Walking));
        assert!(!fsm.is_in(&TestState::Idle));
    }

    #[test]
    fn test_transition_same_state_returns_false() {
        let mut fsm = StateMachine::new(TestState::Idle);
        let changed = fsm.transition(TestState::Idle);
        assert!(!changed);
        assert!(fsm.is_in(&TestState::Idle));
    }

    #[test]
    fn test_multiple_transitions() {
        let mut fsm = StateMachine::new(TestState::Idle);

        assert!(fsm.transition(TestState::Walking));
        assert_eq!(fsm.current(), &TestState::Walking);

        assert!(fsm.transition(TestState::Jumping));
        assert_eq!(fsm.current(), &TestState::Jumping);

        assert!(fsm.transition(TestState::Falling));
        assert_eq!(fsm.current(), &TestState::Falling);

        assert!(fsm.transition(TestState::Idle));
        assert_eq!(fsm.current(), &TestState::Idle);
    }

    #[test]
    fn test_works_with_integer_states() {
        // StateMachine should work with any Eq + Clone + Debug type
        let mut fsm = StateMachine::new(0u32);
        assert!(fsm.is_in(&0));
        assert!(fsm.transition(1));
        assert!(fsm.is_in(&1));
        assert!(!fsm.transition(1));
    }

    #[test]
    fn test_works_with_string_states() {
        let mut fsm = StateMachine::new("idle".to_string());
        assert!(fsm.is_in(&"idle".to_string()));
        assert!(fsm.transition("running".to_string()));
        assert!(fsm.is_in(&"running".to_string()));
    }

    #[test]
    fn test_previous_is_none_initially() {
        let fsm = StateMachine::new(TestState::Idle);
        assert!(fsm.previous().is_none());
    }

    #[test]
    fn test_previous_set_on_transition() {
        let mut fsm = StateMachine::new(TestState::Idle);
        fsm.transition(TestState::Walking);

        assert_eq!(fsm.previous(), Some(&TestState::Idle));
    }

    #[test]
    fn test_previous_tracks_last_transition() {
        let mut fsm = StateMachine::new(TestState::Idle);
        fsm.transition(TestState::Walking);
        fsm.transition(TestState::Jumping);

        // Previous should be Walking (the state before Jumping), not Idle
        assert_eq!(fsm.previous(), Some(&TestState::Walking));
    }

    #[test]
    fn test_previous_unchanged_on_same_state_transition() {
        let mut fsm = StateMachine::new(TestState::Idle);
        fsm.transition(TestState::Walking);
        fsm.transition(TestState::Walking); // No-op

        // Previous should still be Idle (last successful transition)
        assert_eq!(fsm.previous(), Some(&TestState::Idle));
    }
}
