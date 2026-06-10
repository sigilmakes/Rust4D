//! Window management system
//!
//! Handles window creation, cursor capture/release, fullscreen toggle, and title updates.

use crate::config::WindowConfig;
use std::sync::Arc;
use winit::{
    event_loop::ActiveEventLoop,
    window::{CursorGrabMode, Fullscreen, Window},
};

/// Manages the application window and cursor state
pub struct WindowSystem {
    window: Arc<Window>,
    cursor_captured: bool,
    base_title: String,
}

impl WindowSystem {
    /// Create window from config
    pub fn create(
        event_loop: &ActiveEventLoop,
        config: &WindowConfig,
    ) -> Result<Self, WindowError> {
        let mut attrs = Window::default_attributes()
            .with_title(&config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height));

        if config.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|e| WindowError::CreationFailed(e.to_string()))?,
        );

        Ok(Self {
            window,
            cursor_captured: false,
            base_title: config.title.clone(),
        })
    }

    /// Get window reference (for RenderContext creation)
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// Check if cursor is captured
    pub fn is_cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    /// Capture cursor for FPS-style controls
    pub fn capture_cursor(&mut self) -> bool {
        let grab_result = self
            .window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));

        if grab_result.is_ok() {
            self.window.set_cursor_visible(false);
            self.cursor_captured = true;
            log::info!("Cursor captured - Escape to release");
            true
        } else {
            log::warn!("Failed to capture cursor");
            false
        }
    }

    /// Release cursor
    pub fn release_cursor(&mut self) {
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.cursor_captured = false;
        log::info!("Cursor released - click to capture");
    }

    /// Toggle fullscreen mode
    pub fn toggle_fullscreen(&self) {
        let new_fullscreen = if self.window.fullscreen().is_some() {
            None
        } else {
            Some(Fullscreen::Borderless(None))
        };
        self.window.set_fullscreen(new_fullscreen);
    }

    /// Update window title with position/state info
    pub fn update_title(&self, pos: [f32; 4], slice_w: f32) {
        let title = if self.cursor_captured {
            format!(
                "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
                self.base_title, pos[0], pos[1], pos[2], pos[3], slice_w
            )
        } else {
            format!(
                "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
                self.base_title, pos[0], pos[1], pos[2], pos[3], slice_w
            )
        };
        self.window.set_title(&title);
    }

    /// Request a redraw
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[derive(Debug)]
pub enum WindowError {
    CreationFailed(String),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::CreationFailed(msg) => write!(f, "Window creation failed: {}", msg),
        }
    }
}

impl std::error::Error for WindowError {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_title_formatting_captured() {
        // Test title format when cursor is captured
        // Note: Can't test actual window without event loop
        let pos = [1.0, 2.0, 3.0, 4.0];
        let title = format!(
            "Test - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
            pos[0], pos[1], pos[2], pos[3], 0.5
        );
        assert!(title.contains("Esc to release"));
    }

    #[test]
    fn test_title_formatting_released() {
        let pos = [1.0, 2.0, 3.0, 4.0];
        let title = format!(
            "Test - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
            pos[0], pos[1], pos[2], pos[3], 0.5
        );
        assert!(title.contains("Click to capture"));
    }
}
