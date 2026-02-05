//! HUD drawing context
//!
//! Provides a simplified API for common HUD operations on top of egui.

use egui::{
    Align2, Area, Color32, Context, CornerRadius, FontId, Order, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Vec2,
};

/// Simplified HUD drawing context
///
/// Wraps egui primitives for common HUD operations like drawing text,
/// rectangles, progress bars, and screen flashes.
///
/// # Example
///
/// ```ignore
/// let hud = HudContext::new(ctx);
///
/// // Draw health bar
/// hud.text([10.0, 10.0], "Health", 16.0, [1.0, 1.0, 1.0, 1.0]);
/// hud.progress_bar(
///     [10.0, 30.0],
///     [200.0, 20.0],
///     0.75,
///     [0.2, 0.2, 0.2, 1.0],
///     [0.0, 1.0, 0.0, 1.0],
/// );
///
/// // Flash red when taking damage
/// hud.flash([1.0, 0.0, 0.0, 0.3]);
/// ```
pub struct HudContext<'a> {
    ctx: &'a Context,
}

impl<'a> HudContext<'a> {
    /// Create a new HUD drawing context
    ///
    /// # Arguments
    ///
    /// * `ctx` - egui Context to draw on
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Draw text at screen position
    ///
    /// # Arguments
    ///
    /// * `pos` - Position in screen coordinates [x, y]
    /// * `text` - Text to display
    /// * `size` - Font size in points
    /// * `color` - RGBA color [r, g, b, a] in 0.0-1.0 range
    pub fn text(&self, pos: [f32; 2], text: &str, size: f32, color: [f32; 4]) {
        let pos = Pos2::new(pos[0], pos[1]);
        let color = rgba_to_color32(color);

        Area::new(egui::Id::new(("hud_text", pos.x as i32, pos.y as i32, text)))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(self.ctx, |ui| {
                ui.label(RichText::new(text).font(FontId::proportional(size)).color(color));
            });
    }

    /// Draw text centered at screen position
    ///
    /// # Arguments
    ///
    /// * `pos` - Center position in screen coordinates [x, y]
    /// * `text` - Text to display
    /// * `size` - Font size in points
    /// * `color` - RGBA color [r, g, b, a] in 0.0-1.0 range
    pub fn text_centered(&self, pos: [f32; 2], text: &str, size: f32, color: [f32; 4]) {
        let pos = Pos2::new(pos[0], pos[1]);
        let color = rgba_to_color32(color);

        Area::new(egui::Id::new(("hud_text_centered", pos.x as i32, pos.y as i32, text)))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_pos(pos)
            .show(self.ctx, |ui| {
                ui.label(RichText::new(text).font(FontId::proportional(size)).color(color));
            });
    }

    /// Draw a filled rectangle
    ///
    /// # Arguments
    ///
    /// * `pos` - Top-left position in screen coordinates [x, y]
    /// * `size` - Size of the rectangle [width, height]
    /// * `color` - RGBA fill color [r, g, b, a] in 0.0-1.0 range
    pub fn rect(&self, pos: [f32; 2], size: [f32; 2], color: [f32; 4]) {
        let color = rgba_to_color32(color);

        Area::new(egui::Id::new(("hud_rect", pos[0] as i32, pos[1] as i32)))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(pos[0], pos[1]))
            .show(self.ctx, |ui| {
                let (response, painter) =
                    ui.allocate_painter(Vec2::new(size[0], size[1]), Sense::hover());
                painter.rect_filled(response.rect, CornerRadius::ZERO, color);
            });
    }

    /// Draw a rectangle outline
    ///
    /// # Arguments
    ///
    /// * `pos` - Top-left position in screen coordinates [x, y]
    /// * `size` - Size of the rectangle [width, height]
    /// * `color` - RGBA stroke color [r, g, b, a] in 0.0-1.0 range
    /// * `stroke_width` - Width of the outline stroke in pixels
    pub fn rect_outline(&self, pos: [f32; 2], size: [f32; 2], color: [f32; 4], stroke_width: f32) {
        let color = rgba_to_color32(color);

        Area::new(egui::Id::new(("hud_rect_outline", pos[0] as i32, pos[1] as i32)))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(pos[0], pos[1]))
            .show(self.ctx, |ui| {
                let (response, painter) =
                    ui.allocate_painter(Vec2::new(size[0], size[1]), Sense::hover());
                painter.rect_stroke(response.rect, CornerRadius::ZERO, Stroke::new(stroke_width, color), StrokeKind::Outside);
            });
    }

    /// Draw a progress bar
    ///
    /// # Arguments
    ///
    /// * `pos` - Top-left position in screen coordinates [x, y]
    /// * `size` - Size of the progress bar [width, height]
    /// * `progress` - Progress value from 0.0 to 1.0
    /// * `bg_color` - RGBA background color [r, g, b, a] in 0.0-1.0 range
    /// * `fill_color` - RGBA fill color [r, g, b, a] in 0.0-1.0 range
    pub fn progress_bar(
        &self,
        pos: [f32; 2],
        size: [f32; 2],
        progress: f32,
        bg_color: [f32; 4],
        fill_color: [f32; 4],
    ) {
        let progress = progress.clamp(0.0, 1.0);
        let bg_color = rgba_to_color32(bg_color);
        let fill_color = rgba_to_color32(fill_color);

        Area::new(egui::Id::new(("hud_progress", pos[0] as i32, pos[1] as i32)))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(pos[0], pos[1]))
            .show(self.ctx, |ui| {
                let (response, painter) =
                    ui.allocate_painter(Vec2::new(size[0], size[1]), Sense::hover());
                let rect = response.rect;

                // Draw background
                painter.rect_filled(rect, CornerRadius::ZERO, bg_color);

                // Draw fill
                let fill_width = rect.width() * progress;
                let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
                painter.rect_filled(fill_rect, CornerRadius::ZERO, fill_color);
            });
    }

    /// Flash the screen (for damage, pickups, etc)
    ///
    /// Creates a full-screen overlay with the given color.
    ///
    /// # Arguments
    ///
    /// * `color` - RGBA flash color [r, g, b, a] in 0.0-1.0 range
    pub fn flash(&self, color: [f32; 4]) {
        let color = rgba_to_color32(color);
        let screen = self.ctx.screen_rect();

        Area::new(egui::Id::new("hud_flash"))
            .order(Order::Background) // Behind other HUD elements but over scene
            .fixed_pos(Pos2::ZERO)
            .show(self.ctx, |ui| {
                let (_response, painter) =
                    ui.allocate_painter(screen.size(), Sense::hover());
                painter.rect_filled(screen, CornerRadius::ZERO, color);
            });
    }

    /// Get screen dimensions
    ///
    /// Returns the screen size in logical pixels [width, height].
    pub fn screen_size(&self) -> [f32; 2] {
        let rect = self.ctx.screen_rect();
        [rect.width(), rect.height()]
    }

    /// Get the underlying egui Context
    ///
    /// For advanced use cases that need direct egui access.
    pub fn egui_context(&self) -> &Context {
        self.ctx
    }
}

/// Convert RGBA float array [0.0-1.0] to egui Color32
pub fn rgba_to_color32(color: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    )
}

/// Convert egui Color32 to RGBA float array [0.0-1.0]
///
/// Uses sRGBA unmultiplied format to match the input format of `rgba_to_color32`.
pub fn color32_to_rgba(color: Color32) -> [f32; 4] {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_color32() {
        // Pure white
        let white = rgba_to_color32([1.0, 1.0, 1.0, 1.0]);
        assert_eq!(white, Color32::WHITE);

        // Pure black
        let black = rgba_to_color32([0.0, 0.0, 0.0, 1.0]);
        assert_eq!(black, Color32::BLACK);

        // Red
        let red = rgba_to_color32([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(red, Color32::from_rgb(255, 0, 0));

        // Semi-transparent green
        let green = rgba_to_color32([0.0, 1.0, 0.0, 0.5]);
        assert_eq!(green, Color32::from_rgba_unmultiplied(0, 255, 0, 127));
    }

    #[test]
    fn test_color32_to_rgba() {
        // White
        let rgba = color32_to_rgba(Color32::WHITE);
        assert!((rgba[0] - 1.0).abs() < 0.01);
        assert!((rgba[1] - 1.0).abs() < 0.01);
        assert!((rgba[2] - 1.0).abs() < 0.01);
        assert!((rgba[3] - 1.0).abs() < 0.01);

        // Black
        let rgba = color32_to_rgba(Color32::BLACK);
        assert!((rgba[0] - 0.0).abs() < 0.01);
        assert!((rgba[1] - 0.0).abs() < 0.01);
        assert!((rgba[2] - 0.0).abs() < 0.01);
        assert!((rgba[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_roundtrip() {
        let original = [0.5, 0.25, 0.75, 0.8];
        let color32 = rgba_to_color32(original);
        let back = color32_to_rgba(color32);

        // Allow for rounding errors due to 8-bit color (max error ~0.004 per channel)
        for i in 0..4 {
            assert!(
                (original[i] - back[i]).abs() < 0.02,
                "Channel {}: original={}, back={}",
                i, original[i], back[i]
            );
        }
    }
}
