//! 4D Input Handling
//!
//! This crate provides input handling for 4D camera control,
//! replicating 4D Golf-style controls.

mod action_map;
mod camera_controller;

pub use action_map::{ActionMap, CameraAction};
pub use camera_controller::{CameraControl, CameraController};
