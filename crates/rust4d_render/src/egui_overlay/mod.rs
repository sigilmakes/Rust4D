//! egui overlay for HUD rendering
//!
//! Provides a 2D immediate-mode GUI layer rendered on top of the 4D scene.
//! Built on top of [egui](https://github.com/emilk/egui) for maximum flexibility
//! and ease of use.
//!
//! # Overview
//!
//! The overlay system consists of two main components:
//!
//! - [`EguiRenderer`] - Low-level integration with wgpu for rendering egui
//! - [`HudContext`] - Simplified API for common HUD operations
//!
//! # Usage
//!
//! ```ignore
//! use rust4d_render::egui_overlay::{EguiRenderer, HudContext};
//! use egui::RawInput;
//! use egui_wgpu::ScreenDescriptor;
//!
//! // Create the renderer (once, during initialization)
//! let mut egui_renderer = EguiRenderer::new(&device, surface_format);
//!
//! // In your game loop:
//! let raw_input = RawInput::default();
//! let ctx = egui_renderer.begin_frame(raw_input);
//!
//! // Use the simplified HUD API
//! let hud = HudContext::new(ctx);
//! hud.text([10.0, 10.0], "Health: 100", 20.0, [1.0, 1.0, 1.0, 1.0]);
//! hud.progress_bar(
//!     [10.0, 40.0],
//!     [200.0, 20.0],
//!     0.75,
//!     [0.2, 0.2, 0.2, 1.0],
//!     [0.0, 1.0, 0.0, 1.0],
//! );
//!
//! // Or use egui directly for complex UI
//! egui::Window::new("Debug Info").show(ctx, |ui| {
//!     ui.label("FPS: 60");
//!     ui.label("Position: (0, 0, 0, 0)");
//! });
//!
//! // Finalize and render
//! let output = egui_renderer.end_frame();
//! let screen_desc = ScreenDescriptor {
//!     size_in_pixels: [width, height],
//!     pixels_per_point: 1.0,
//! };
//! egui_renderer.render(&mut encoder, &view, &screen_desc, &device, &queue, output);
//! ```
//!
//! # HUD Drawing API
//!
//! The [`HudContext`] provides simple methods for common HUD elements:
//!
//! - [`HudContext::text`] / [`HudContext::text_centered`] - Draw text labels
//! - [`HudContext::rect`] / [`HudContext::rect_outline`] - Draw rectangles
//! - [`HudContext::progress_bar`] - Draw progress/health bars
//! - [`HudContext::flash`] - Full-screen flash effect for damage/pickups
//!
//! All color parameters use RGBA float arrays `[f32; 4]` with values in the
//! range 0.0 to 1.0 for consistency with the rest of the engine.
//!
//! # Input Handling
//!
//! For full input support (mouse, keyboard), enable the `egui-winit` feature
//! and use `egui_winit::State` to convert winit events to egui [`RawInput`].
//!
//! ```ignore
//! // With egui-winit feature enabled:
//! use egui_winit::State as EguiWinitState;
//!
//! let mut egui_state = EguiWinitState::new(
//!     egui_renderer.context().clone(),
//!     viewport_id,
//!     &window,
//!     None,
//!     None,
//! );
//!
//! // In event loop:
//! let _ = egui_state.on_window_event(&window, &event);
//!
//! // When rendering:
//! let raw_input = egui_state.take_egui_input(&window);
//! ```

mod context;
mod renderer;

pub use context::{HudContext, color32_to_rgba, rgba_to_color32};
pub use renderer::EguiRenderer;

// Re-export key egui types for convenience
pub use egui::{Context as EguiContext, FullOutput, RawInput};
pub use egui_wgpu::ScreenDescriptor;
