//! Sprite/Billboard rendering system for 4D
//!
//! This module provides a sprite rendering system for displaying 2D billboards
//! in 4D space. Sprites always face the camera in XYZ space (classic billboard
//! behavior) while being positioned in 4D with W-fade effects.
//!
//! # Key Features
//!
//! - **4D Positioning**: Sprites have full 4D positions (x, y, z, w)
//! - **W-Fade**: Sprites fade based on their W distance from the camera
//! - **Billboard Behavior**: Sprites always face the camera in XYZ space
//! - **Sprite Sheets**: Support for animated sprites via atlas textures
//! - **Depth Sorting**: Automatic back-to-front sorting for correct transparency
//!
//! # Example
//!
//! ```ignore
//! use rust4d_render::sprite::{SpriteBatch, Sprite, SpriteSheet};
//! use rust4d_math::Vec4;
//!
//! // Create a sprite batch
//! let mut batch = SpriteBatch::new();
//!
//! // Register a sprite sheet
//! let sheet = SpriteSheet::new("enemies", 32, 32, 4, 4);
//! batch.register_sheet(sheet);
//!
//! // Add sprites
//! batch.add(
//!     Sprite::new(Vec4::new(0.0, 1.0, 0.0, 0.0), "enemies")
//!         .with_frame(0)
//!         .with_size(1.0, 1.0)
//!         .with_w_fade_range(2.0)
//! );
//!
//! // Get sorted sprites for rendering
//! let camera_pos = Vec4::new(0.0, 1.0, 5.0, 0.0);
//! let sorted = batch.get_sorted(camera_pos);
//!
//! // Render sprites back-to-front...
//!
//! // Clear for next frame
//! batch.clear();
//! ```
//!
//! # W-Fade System
//!
//! Sprites fade based on their distance from the camera in the W dimension:
//!
//! - At W distance 0: Fully opaque (alpha = 1.0)
//! - At W distance = w_fade_range: Fully transparent (alpha = 0.0)
//! - In between: Linear interpolation
//!
//! This creates a smooth transition effect as sprites move through 4D space.
//!
//! # Billboard Behavior
//!
//! Sprites are "billboards" - they always face the camera:
//!
//! - Position in 4D determines where they appear
//! - They face the camera (no 3D rotation)
//! - Size determines world-space dimensions
//!
//! # Depth Sorting
//!
//! For correct transparency rendering, sprites should be rendered back-to-front.
//! Use `get_sorted()` to get sprites in the correct order based on their 3D
//! distance from the camera (W dimension is ignored for depth sorting since
//! billboards are rendered at their XYZ projection).

pub mod batch;
pub mod types;

// Re-export main types for convenience
pub use batch::SpriteBatch;
pub use types::{Sprite, SpriteSheet, WFadeConfig};
