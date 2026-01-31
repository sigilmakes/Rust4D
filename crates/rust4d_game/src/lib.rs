//! Game logic layer for Rust4D
//!
//! This crate bridges the engine and game-specific code, providing:
//!
//! - [`CharacterController4D`] - First-person character controller for 4D space
//! - [`EventBus`] - Typed event bus for game events
//! - [`StateMachine`] - Generic finite state machine
//! - Scene helpers for physics setup from entity tags

pub mod character_controller;
pub mod events;
pub mod fsm;
pub mod scene_helpers;

pub use character_controller::{CharacterController4D, CharacterConfig};
pub use events::{EventBus, HandlerId};
pub use fsm::StateMachine;
