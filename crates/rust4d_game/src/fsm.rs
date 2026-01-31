//! Generic finite state machine
//!
//! A minimal state machine that tracks the current state and supports transitions.
//! Game-specific state logic (enter/exit callbacks, transition conditions) is left
//! to the consumer -- this just provides clean state tracking.

use std::hash::Hash;

/// A generic finite state machine
///
/// Tracks the current state and provides transition methods. The state type `S`
/// must implement `Eq`, `Hash`, `Clone`, and `Debug`.
///
/// # Example
///
/// ```
/// use rust4d_game::StateMachine;
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
/// // Transition to same state returns false
/// let changed = fsm.transition(PlayerState::Walking);
/// assert!(!changed);
/// ```
pub struct StateMachine<S: Eq + Hash + Clone + std::fmt::Debug> {
    current: S,
}

impl<S: Eq + Hash + Clone + std::fmt::Debug> StateMachine<S> {
    /// Create a new state machine with the given initial state
    pub fn new(initial: S) -> Self {
        Self { current: initial }
    }

    /// Get a reference to the current state
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Transition to a new state
    ///
    /// Returns `true` if the state changed, `false` if already in that state.
    pub fn transition(&mut self, new_state: S) -> bool {
        if self.current != new_state {
            log::debug!("FSM transition: {:?} -> {:?}", self.current, new_state);
            self.current = new_state;
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

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        // StateMachine should work with any Eq + Hash + Clone + Debug type
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
}
