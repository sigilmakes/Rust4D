//! HUD/GUI bindings for Lua
//!
//! Provides Lua access to the HUD overlay system:
//! - `hud.text(x, y, text, size, color)` - Draw text at screen position
//! - `hud.text_centered(x, y, text, size, color)` - Draw text centered at position
//! - `hud.rect(x, y, width, height, color)` - Draw filled rectangle
//! - `hud.rect_outline(x, y, width, height, color, stroke)` - Draw rectangle outline
//! - `hud.progress_bar(x, y, width, height, progress, bg_color, fill_color)` - Draw progress bar
//! - `hud.flash(color)` - Flash the entire screen
//! - `hud.screen_size()` - Get screen dimensions
//!
//! ## Usage (Lua)
//!
//! ```lua
//! -- Example HUD script for a health bar game
//! function on_update(dt)
//!     -- Get screen dimensions
//!     local sw, sh = hud.screen_size()
//!
//!     -- Draw health bar in top-left
//!     local health = player:get_health() / player:get_max_health()
//!     hud.text(10, 10, "Health", 14, {1, 1, 1, 1})
//!     hud.progress_bar(10, 30, 200, 20, health,
//!         {0.2, 0.2, 0.2, 0.8},  -- dark gray background
//!         {0.8, 0.1, 0.1, 1.0})  -- red fill
//!
//!     -- Draw ammo count in top-right
//!     local ammo_text = string.format("Ammo: %d / %d", current_ammo, max_ammo)
//!     hud.text(sw - 150, 10, ammo_text, 16, {1, 1, 1, 1})
//!
//!     -- Flash red when taking damage
//!     if just_took_damage then
//!         hud.flash({1, 0, 0, 0.3})
//!     end
//! end
//! ```
//!
//! ## Color Format
//!
//! Colors can be specified as either:
//! - Array format: `{r, g, b, a}` e.g., `{1, 0, 0, 1}` for red
//! - Named format: `{r=1, g=0, b=0, a=1}` for red
//!
//! All color values are in the 0.0-1.0 range. Alpha defaults to 1.0 if omitted.
//!
//! ## Coordinate System
//!
//! All positions are in screen pixels with origin at top-left:
//! - X increases to the right
//! - Y increases downward
//!
//! ## Design Note
//!
//! The actual HudContext lives in rust4d_render, not in Lua. This implementation provides
//! the binding API structure with stub operations that log debug messages.
//! Full integration with the engine's HudContext happens when the engine binary wires up
//! the scripting system via `lua.set_app_data()`.
//!
//! This module is owned by Agent A3 (Lua HUD API Bindings).

use mlua::prelude::*;

/// Register HUD bindings with the Lua VM
///
/// Creates a global `hud` table with the following functions:
/// - `hud.text(x, y, text, size, color)` - Draw text at screen position
/// - `hud.text_centered(x, y, text, size, color)` - Draw text centered at position
/// - `hud.rect(x, y, width, height, color)` - Draw filled rectangle
/// - `hud.rect_outline(x, y, width, height, color, stroke)` - Draw rectangle outline
/// - `hud.progress_bar(x, y, width, height, progress, bg_color, fill_color)` - Draw progress bar
/// - `hud.flash(color)` - Flash the entire screen
/// - `hud.screen_size() -> width, height` - Get screen dimensions
///
/// # Stub Implementation
///
/// These functions are currently stubs that log debug messages but don't render.
/// Full integration requires the engine to provide HudContext via `lua.set_app_data()`.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let hud_table = lua.create_table()?;

    // hud.text(x, y, text, size, color)
    //
    // Draw text at screen position.
    //
    // Arguments:
    // - x: Horizontal position (pixels from left)
    // - y: Vertical position (pixels from top)
    // - text: Text string to display
    // - size: Font size in points
    // - color: RGBA color table {r, g, b, a} or {1, 2, 3, 4}
    hud_table.set(
        "text",
        lua.create_function(|_, (x, y, text, size, color): (f32, f32, String, f32, LuaTable)| {
            let color = table_to_color(&color)?;
            // STUB: Log debug message
            // Real implementation would:
            // 1. Get HudContext from lua.app_data()
            // 2. Call hud.text([x, y], &text, size, color)
            log::debug!(
                "[hud] text at ({}, {}): '{}' size={} color={:?}",
                x,
                y,
                text,
                size,
                color
            );
            Ok(())
        })?,
    )?;

    // hud.text_centered(x, y, text, size, color)
    //
    // Draw text centered at screen position.
    //
    // Arguments:
    // - x: Center horizontal position (pixels from left)
    // - y: Center vertical position (pixels from top)
    // - text: Text string to display
    // - size: Font size in points
    // - color: RGBA color table {r, g, b, a} or {1, 2, 3, 4}
    hud_table.set(
        "text_centered",
        lua.create_function(|_, (x, y, text, size, color): (f32, f32, String, f32, LuaTable)| {
            let color = table_to_color(&color)?;
            log::debug!(
                "[hud] text_centered at ({}, {}): '{}' size={} color={:?}",
                x,
                y,
                text,
                size,
                color
            );
            Ok(())
        })?,
    )?;

    // hud.rect(x, y, width, height, color)
    //
    // Draw a filled rectangle.
    //
    // Arguments:
    // - x: Left edge position (pixels from left)
    // - y: Top edge position (pixels from top)
    // - width: Rectangle width in pixels
    // - height: Rectangle height in pixels
    // - color: RGBA fill color table
    hud_table.set(
        "rect",
        lua.create_function(|_, (x, y, w, h, color): (f32, f32, f32, f32, LuaTable)| {
            let color = table_to_color(&color)?;
            log::debug!(
                "[hud] rect at ({}, {}) size=({}, {}) color={:?}",
                x,
                y,
                w,
                h,
                color
            );
            Ok(())
        })?,
    )?;

    // hud.rect_outline(x, y, width, height, color, stroke)
    //
    // Draw a rectangle outline (border only, no fill).
    //
    // Arguments:
    // - x: Left edge position (pixels from left)
    // - y: Top edge position (pixels from top)
    // - width: Rectangle width in pixels
    // - height: Rectangle height in pixels
    // - color: RGBA stroke color table
    // - stroke: Stroke width in pixels
    hud_table.set(
        "rect_outline",
        lua.create_function(
            |_, (x, y, w, h, color, stroke): (f32, f32, f32, f32, LuaTable, f32)| {
                let color = table_to_color(&color)?;
                log::debug!(
                    "[hud] rect_outline at ({}, {}) size=({}, {}) stroke={} color={:?}",
                    x,
                    y,
                    w,
                    h,
                    stroke,
                    color
                );
                Ok(())
            },
        )?,
    )?;

    // hud.progress_bar(x, y, width, height, progress, bg_color, fill_color)
    //
    // Draw a progress bar with background and fill.
    //
    // Arguments:
    // - x: Left edge position (pixels from left)
    // - y: Top edge position (pixels from top)
    // - width: Bar width in pixels
    // - height: Bar height in pixels
    // - progress: Progress value from 0.0 (empty) to 1.0 (full)
    // - bg_color: RGBA background color table
    // - fill_color: RGBA fill color table
    hud_table.set(
        "progress_bar",
        lua.create_function(
            |_,
             (x, y, w, h, progress, bg_color, fill_color): (
                f32,
                f32,
                f32,
                f32,
                f32,
                LuaTable,
                LuaTable,
            )| {
                let bg = table_to_color(&bg_color)?;
                let fill = table_to_color(&fill_color)?;
                let progress = progress.clamp(0.0, 1.0);
                log::debug!(
                    "[hud] progress_bar at ({}, {}) size=({}, {}) progress={:.0}% bg={:?} fill={:?}",
                    x,
                    y,
                    w,
                    h,
                    progress * 100.0,
                    bg,
                    fill
                );
                Ok(())
            },
        )?,
    )?;

    // hud.flash(color)
    //
    // Flash the entire screen with a color overlay.
    // Useful for damage feedback, pickups, transitions, etc.
    //
    // Arguments:
    // - color: RGBA flash color table (alpha controls intensity)
    hud_table.set(
        "flash",
        lua.create_function(|_, color: LuaTable| {
            let color = table_to_color(&color)?;
            log::debug!("[hud] flash color={:?}", color);
            Ok(())
        })?,
    )?;

    // hud.screen_size() -> width, height
    //
    // Get the current screen dimensions in logical pixels.
    //
    // Returns:
    // - width: Screen width in pixels
    // - height: Screen height in pixels
    hud_table.set(
        "screen_size",
        lua.create_function(|_, ()| {
            // STUB: Return default fallback
            // Real implementation would get from HudContext::screen_size()
            log::debug!("[hud] screen_size() called - HudContext not bound, returning 1920x1080");
            Ok((1920.0f32, 1080.0f32))
        })?,
    )?;

    // Register the hud table as a global
    lua.globals().set("hud", hud_table)?;

    log::debug!("[hud] HUD bindings registered");
    Ok(())
}

/// Convert a Lua table to an RGBA color array
///
/// Accepts two formats:
/// - Array format: `{r, g, b, a}` at indices 1, 2, 3, 4
/// - Named format: `{r=r, g=g, b=b, a=a}`
///
/// Alpha defaults to 1.0 if not specified.
fn table_to_color(table: &LuaTable) -> LuaResult<[f32; 4]> {
    // Try named keys first (r, g, b, a)
    let r: Option<f32> = table.get("r").ok();
    if let Some(r) = r {
        let g: f32 = table.get("g").unwrap_or(0.0);
        let b: f32 = table.get("b").unwrap_or(0.0);
        let a: f32 = table.get("a").unwrap_or(1.0);
        return Ok([r, g, b, a]);
    }

    // Try array indices {1, 2, 3, 4}
    let r: f32 = table.get(1).unwrap_or(0.0);
    let g: f32 = table.get(2).unwrap_or(0.0);
    let b: f32 = table.get(3).unwrap_or(0.0);
    let a: f32 = table.get(4).unwrap_or(1.0);
    Ok([r, g, b, a])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_lua_with_hud() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register HUD bindings");
        lua
    }

    #[test]
    fn test_hud_table_exists() {
        let lua = create_lua_with_hud();
        let hud: LuaTable = lua
            .globals()
            .get("hud")
            .expect("hud table should exist");
        assert!(hud.contains_key("text").unwrap());
        assert!(hud.contains_key("text_centered").unwrap());
        assert!(hud.contains_key("rect").unwrap());
        assert!(hud.contains_key("rect_outline").unwrap());
        assert!(hud.contains_key("progress_bar").unwrap());
        assert!(hud.contains_key("flash").unwrap());
        assert!(hud.contains_key("screen_size").unwrap());
    }

    #[test]
    fn test_hud_text_callable() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text(10, 20, "Hello", 16, {1, 1, 1, 1})"#)
            .exec()
            .expect("hud.text should be callable");
    }

    #[test]
    fn test_hud_text_with_named_colors() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text(10, 20, "Hello", 16, {r=1, g=0.5, b=0, a=1})"#)
            .exec()
            .expect("hud.text should accept named color format");
    }

    #[test]
    fn test_hud_text_centered_callable() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text_centered(400, 300, "Game Over", 32, {1, 0, 0, 1})"#)
            .exec()
            .expect("hud.text_centered should be callable");
    }

    #[test]
    fn test_hud_rect_callable() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.rect(0, 0, 100, 50, {0.2, 0.2, 0.2, 0.8})"#)
            .exec()
            .expect("hud.rect should be callable");
    }

    #[test]
    fn test_hud_rect_outline_callable() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.rect_outline(50, 50, 200, 100, {1, 1, 1, 1}, 2)"#)
            .exec()
            .expect("hud.rect_outline should be callable");
    }

    #[test]
    fn test_hud_progress_bar_callable() {
        let lua = create_lua_with_hud();
        lua.load(
            r#"
            hud.progress_bar(10, 40, 200, 20, 0.75, {0.2, 0.2, 0.2, 1}, {0, 1, 0, 1})
        "#,
        )
        .exec()
        .expect("hud.progress_bar should be callable");
    }

    #[test]
    fn test_hud_progress_bar_clamps_progress() {
        let lua = create_lua_with_hud();
        // Should not error, just clamp values
        lua.load(
            r#"
            hud.progress_bar(10, 40, 200, 20, 2.0, {0.2, 0.2, 0.2, 1}, {0, 1, 0, 1})
            hud.progress_bar(10, 40, 200, 20, -1.0, {0.2, 0.2, 0.2, 1}, {0, 1, 0, 1})
        "#,
        )
        .exec()
        .expect("progress_bar should clamp out-of-range progress values");
    }

    #[test]
    fn test_hud_flash_callable() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.flash({1, 0, 0, 0.3})"#)
            .exec()
            .expect("hud.flash should be callable");
    }

    #[test]
    fn test_hud_screen_size_returns_two_values() {
        let lua = create_lua_with_hud();
        lua.load(
            r#"
            local w, h = hud.screen_size()
            assert(type(w) == 'number', 'width should be number')
            assert(type(h) == 'number', 'height should be number')
            assert(w > 0, 'width should be positive')
            assert(h > 0, 'height should be positive')
        "#,
        )
        .exec()
        .expect("hud.screen_size should return two positive numbers");
    }

    #[test]
    fn test_color_table_array_format() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1, 0.5f32).unwrap();
        table.set(2, 0.6f32).unwrap();
        table.set(3, 0.7f32).unwrap();
        table.set(4, 0.8f32).unwrap();

        let color = table_to_color(&table).unwrap();
        assert!((color[0] - 0.5).abs() < 0.001);
        assert!((color[1] - 0.6).abs() < 0.001);
        assert!((color[2] - 0.7).abs() < 0.001);
        assert!((color[3] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_color_table_named_format() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("r", 0.1f32).unwrap();
        table.set("g", 0.2f32).unwrap();
        table.set("b", 0.3f32).unwrap();
        table.set("a", 0.4f32).unwrap();

        let color = table_to_color(&table).unwrap();
        assert!((color[0] - 0.1).abs() < 0.001);
        assert!((color[1] - 0.2).abs() < 0.001);
        assert!((color[2] - 0.3).abs() < 0.001);
        assert!((color[3] - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_color_defaults_alpha() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1, 1.0f32).unwrap();
        table.set(2, 0.0f32).unwrap();
        table.set(3, 0.0f32).unwrap();
        // No alpha specified

        let color = table_to_color(&table).unwrap();
        assert!((color[3] - 1.0).abs() < 0.001, "Default alpha should be 1.0");
    }

    #[test]
    fn test_color_named_defaults_missing_components() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("r", 1.0f32).unwrap();
        // g, b, a not specified

        let color = table_to_color(&table).unwrap();
        assert!((color[0] - 1.0).abs() < 0.001);
        assert!((color[1] - 0.0).abs() < 0.001, "Missing g should default to 0.0");
        assert!((color[2] - 0.0).abs() < 0.001, "Missing b should default to 0.0");
        assert!((color[3] - 1.0).abs() < 0.001, "Missing a should default to 1.0");
    }

    #[test]
    fn test_full_hud_workflow() {
        let lua = create_lua_with_hud();
        lua.load(
            r#"
            -- Typical game HUD setup
            local sw, sh = hud.screen_size()

            -- Health bar
            hud.text(10, 10, "Health", 14, {1, 1, 1, 1})
            hud.progress_bar(10, 30, 200, 20, 0.75,
                {0.2, 0.2, 0.2, 0.8},
                {0.8, 0.1, 0.1, 1.0})

            -- Ammo counter
            local ammo_text = string.format("Ammo: %d / %d", 25, 30)
            hud.text(sw - 150, 10, ammo_text, 16, {1, 1, 1, 1})

            -- Stamina bar
            hud.text(10, 60, "Stamina", 14, {1, 1, 1, 1})
            hud.progress_bar(10, 80, 200, 15, 0.5,
                {0.2, 0.2, 0.2, 0.8},
                {0.2, 0.6, 1.0, 1.0})

            -- Crosshair (centered)
            hud.rect(sw/2 - 10, sh/2 - 1, 20, 2, {1, 1, 1, 0.8})
            hud.rect(sw/2 - 1, sh/2 - 10, 2, 20, {1, 1, 1, 0.8})

            -- Minimap border
            hud.rect_outline(sw - 160, sh - 160, 150, 150, {1, 1, 1, 0.5}, 2)

            -- Game over text (example)
            -- hud.text_centered(sw/2, sh/2, "GAME OVER", 48, {1, 0, 0, 1})

            -- Damage flash (example)
            -- hud.flash({1, 0, 0, 0.3})
        "#,
        )
        .exec()
        .expect("Full HUD workflow should execute without errors");
    }

    #[test]
    fn test_hud_with_special_characters() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text(10, 10, "Score: 1,234,567", 18, {1, 1, 1, 1})"#)
            .exec()
            .expect("Should handle text with commas");

        lua.load(r#"hud.text(10, 10, "Health: 100%", 18, {1, 1, 1, 1})"#)
            .exec()
            .expect("Should handle text with percent sign");

        lua.load(r#"hud.text(10, 10, "Player \"Hero\"", 18, {1, 1, 1, 1})"#)
            .exec()
            .expect("Should handle text with quotes");
    }

    #[test]
    fn test_hud_with_empty_string() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text(10, 10, "", 16, {1, 1, 1, 1})"#)
            .exec()
            .expect("Should handle empty string");
    }

    #[test]
    fn test_hud_with_zero_dimensions() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.rect(10, 10, 0, 0, {1, 0, 0, 1})"#)
            .exec()
            .expect("Should handle zero-size rect");

        lua.load(r#"hud.progress_bar(10, 10, 0, 0, 0.5, {0.2, 0.2, 0.2, 1}, {0, 1, 0, 1})"#)
            .exec()
            .expect("Should handle zero-size progress bar");
    }

    #[test]
    fn test_hud_with_negative_positions() {
        let lua = create_lua_with_hud();
        lua.load(r#"hud.text(-10, -20, "Offscreen", 16, {1, 1, 1, 1})"#)
            .exec()
            .expect("Should handle negative positions (offscreen)");
    }
}
