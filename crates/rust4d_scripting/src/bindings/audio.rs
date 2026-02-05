//! Audio bindings for Lua
//!
//! Provides Lua access to the 4D audio system:
//! - `audio.load_sound(path)` - Load a sound file
//! - `audio.play(handle, bus)` - Play a sound on a bus
//! - `audio.play_oneshot(handle, bus)` - Fire and forget playback
//! - `audio.play_spatial(handle, position, bus)` - 4D spatial audio
//! - `audio.play_oneshot_spatial(handle, position, min_dist, max_dist, bus)` - Spatial oneshot
//! - `audio.set_volume(bus, volume)` - Set bus volume
//! - `audio.stop_all()` - Stop all sounds
//! - `audio.stop_bus(bus)` - Stop sounds on a specific bus
//!
//! ## Usage (Lua)
//!
//! ```lua
//! -- Load sounds at startup
//! local explosion_sound = audio.load_sound("sounds/explosion.ogg")
//! local music = audio.load_sound("music/theme.ogg")
//!
//! -- Play music on the music bus
//! audio.play(music, "music")
//!
//! -- Play sound effect
//! audio.play_oneshot(explosion_sound, "sfx")
//!
//! -- Play 4D spatial audio
//! local source_pos = Vec4.new(10, 0, 5, 2)
//! audio.play_spatial(explosion_sound, source_pos, "sfx")
//!
//! -- Play spatial with custom distance parameters
//! audio.play_oneshot_spatial(explosion_sound, source_pos, 1.0, 50.0, "sfx")
//!
//! -- Adjust volume
//! audio.set_volume("music", 0.5)  -- Half volume
//! audio.set_volume("sfx", 1.0)    -- Full volume
//!
//! -- Stop everything
//! audio.stop_bus("music")
//! audio.stop_all()
//! ```
//!
//! ## Audio Buses
//!
//! - `"master"` - Master bus, affects all audio
//! - `"sfx"` - Sound effects bus (default)
//! - `"music"` - Background music bus
//! - `"ambient"` - Environmental/ambient audio bus
//!
//! ## Design Note
//!
//! The actual AudioEngine4D lives in the engine, not in Lua. This implementation provides
//! the binding API structure with stub operations that log warnings and return defaults.
//! Full integration with the engine's AudioEngine4D happens when the engine binary wires up
//! the scripting system via `lua.set_app_data()`.
//!
//! This module is owned by Agent D3 (Input/Audio Bindings).

use mlua::prelude::*;

use super::math::LuaVec4;

/// Lua wrapper for a sound handle
///
/// This is a lightweight reference to a loaded sound asset.
/// The actual sound data is stored in the AudioEngine4D.
#[derive(Clone, Copy, Debug)]
pub struct LuaSoundHandle {
    /// Internal sound ID
    id: u64,
}

impl LuaSoundHandle {
    /// Create a new sound handle
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    /// Get the internal ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl LuaUserData for LuaSoundHandle {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Expose the ID for debugging/comparison
        fields.add_field_method_get("id", |_, this| Ok(this.id));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Equality comparison
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaSoundHandle| {
            Ok(this.id == other.id)
        });

        // String representation
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("SoundHandle({})", this.id))
        });
    }
}

impl FromLua for LuaSoundHandle {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaSoundHandle>().map(|h| *h),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "SoundHandle".to_string(),
                message: Some("expected SoundHandle userdata".to_string()),
            }),
        }
    }
}

/// Parse bus name string to a validated bus name
///
/// Returns the lowercase bus name if valid, or an error if invalid.
fn validate_bus_name(name: &str) -> LuaResult<String> {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "master" | "sfx" | "music" | "ambient" => Ok(lower),
        _ => Err(LuaError::RuntimeError(format!(
            "Invalid audio bus '{}'. Valid buses: master, sfx, music, ambient",
            name
        ))),
    }
}

/// Register audio bindings with the Lua VM
///
/// Creates a global `audio` table with the following functions:
/// - `audio.load_sound(path: string) -> SoundHandle` - Load a sound file
/// - `audio.play(handle: SoundHandle, bus: string)` - Play sound on bus
/// - `audio.play_oneshot(handle: SoundHandle, bus: string)` - Fire and forget
/// - `audio.play_spatial(handle: SoundHandle, position: Vec4, bus: string)` - 4D spatial
/// - `audio.play_oneshot_spatial(handle, position, min_dist, max_dist, bus)` - Spatial oneshot
/// - `audio.set_volume(bus: string, volume: number)` - Set bus volume (0.0 to 1.0)
/// - `audio.stop_all()` - Stop all sounds
/// - `audio.stop_bus(bus: string)` - Stop sounds on a bus
///
/// # Stub Implementation
///
/// These functions are currently stubs that log warnings but return sensible defaults.
/// Full integration requires the engine to provide AudioEngine4D via `lua.set_app_data()`.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let audio_table = lua.create_table()?;

    // audio.load_sound(path: string) -> SoundHandle
    //
    // Load a sound file from disk.
    //
    // Arguments:
    // - path: Path to the sound file (relative to assets directory)
    //
    // Returns:
    // - SoundHandle that can be used with play functions
    //
    // Note: This is an expensive operation. Load sounds at startup, not during gameplay.
    audio_table.set(
        "load_sound",
        lua.create_function(|_, path: String| {
            // STUB: Return a dummy handle and log warning
            // Real implementation would:
            // 1. Get AudioEngine4D from lua.app_data()
            // 2. Call engine.load_sound(&path)
            // 3. Return the resulting handle
            log::trace!(
                "[audio] load_sound('{}') - stub returning dummy handle",
                path
            );
            Ok(LuaSoundHandle::new(0))
        })?,
    )?;

    // audio.play(handle: SoundHandle, bus: string)
    //
    // Play a sound on the specified audio bus.
    //
    // Arguments:
    // - handle: SoundHandle from load_sound
    // - bus: Audio bus name ("master", "sfx", "music", "ambient")
    audio_table.set(
        "play",
        lua.create_function(|_, (handle, bus): (LuaSoundHandle, String)| {
            let bus_name = validate_bus_name(&bus)?;
            // STUB: Log warning
            // Real implementation would call engine.play(&handle, bus)
            log::trace!("[audio] play(handle={}, bus='{}') - stub", handle.id, bus_name);
            Ok(())
        })?,
    )?;

    // audio.play_oneshot(handle: SoundHandle, bus: string)
    //
    // Play a sound once and forget about it.
    // Use this for short sound effects that don't need to be tracked.
    //
    // Arguments:
    // - handle: SoundHandle from load_sound
    // - bus: Audio bus name
    audio_table.set(
        "play_oneshot",
        lua.create_function(|_, (handle, bus): (LuaSoundHandle, String)| {
            let bus_name = validate_bus_name(&bus)?;
            log::trace!("[audio] play_oneshot(handle={}, bus='{}') - stub", handle.id, bus_name);
            Ok(())
        })?,
    )?;

    // audio.play_spatial(handle: SoundHandle, position: Vec4, bus: string)
    //
    // Play a sound with 4D spatial positioning.
    // Volume and panning are calculated based on distance from the listener.
    //
    // Arguments:
    // - handle: SoundHandle from load_sound
    // - position: Vec4 position in 4D world space
    // - bus: Audio bus name
    audio_table.set(
        "play_spatial",
        lua.create_function(|_, (handle, pos, bus): (LuaSoundHandle, LuaVec4, String)| {
            let bus_name = validate_bus_name(&bus)?;
            log::trace!(
                "[audio] play_spatial(handle={}, pos=({:.2}, {:.2}, {:.2}, {:.2}), bus='{}') - stub",
                handle.id, pos.0.x, pos.0.y, pos.0.z, pos.0.w, bus_name
            );
            Ok(())
        })?,
    )?;

    // audio.play_oneshot_spatial(handle, position, min_dist, max_dist, bus)
    //
    // Play a spatial sound once with custom distance parameters.
    //
    // Arguments:
    // - handle: SoundHandle from load_sound
    // - position: Vec4 position in 4D world space
    // - min_dist: Distance at which sound is at full volume (default: 1.0)
    // - max_dist: Distance at which sound is silent (default: 50.0)
    // - bus: Audio bus name
    audio_table.set(
        "play_oneshot_spatial",
        lua.create_function(
            |_, (handle, pos, min_dist, max_dist, bus): (LuaSoundHandle, LuaVec4, f32, f32, String)| {
                let bus_name = validate_bus_name(&bus)?;

                // Validate distance parameters
                if min_dist < 0.0 {
                    return Err(LuaError::RuntimeError(
                        "min_dist must be >= 0".to_string(),
                    ));
                }
                if max_dist <= min_dist {
                    return Err(LuaError::RuntimeError(
                        "max_dist must be > min_dist".to_string(),
                    ));
                }

                log::trace!(
                    "[audio] play_oneshot_spatial(handle={}, pos=({:.2}, {:.2}, {:.2}, {:.2}), min={:.2}, max={:.2}, bus='{}') - stub",
                    handle.id, pos.0.x, pos.0.y, pos.0.z, pos.0.w, min_dist, max_dist, bus_name
                );
                Ok(())
            },
        )?,
    )?;

    // audio.set_volume(bus: string, volume: number)
    //
    // Set the volume of an audio bus.
    //
    // Arguments:
    // - bus: Audio bus name
    // - volume: Volume level (0.0 = silent, 1.0 = full volume)
    audio_table.set(
        "set_volume",
        lua.create_function(|_, (bus, volume): (String, f32)| {
            let bus_name = validate_bus_name(&bus)?;

            // Clamp volume to valid range
            let clamped = volume.clamp(0.0, 1.0);
            if clamped != volume {
                log::warn!(
                    "[audio] Volume {} clamped to {} for bus '{}'",
                    volume,
                    clamped,
                    bus_name
                );
            }

            log::trace!("[audio] set_volume(bus='{}', volume={:.2}) - stub", bus_name, clamped);
            Ok(())
        })?,
    )?;

    // audio.stop_all()
    //
    // Stop all currently playing sounds.
    audio_table.set(
        "stop_all",
        lua.create_function(|_, ()| {
            log::trace!("[audio] stop_all() - stub");
            Ok(())
        })?,
    )?;

    // audio.stop_bus(bus: string)
    //
    // Stop all sounds playing on a specific bus.
    //
    // Arguments:
    // - bus: Audio bus name
    audio_table.set(
        "stop_bus",
        lua.create_function(|_, bus: String| {
            let bus_name = validate_bus_name(&bus)?;
            log::trace!("[audio] stop_bus('{}') - stub", bus_name);
            Ok(())
        })?,
    )?;

    // audio.update_listener(position: Vec4)
    //
    // Update the listener position for spatial audio calculations.
    // Should be called each frame with the camera/player position.
    //
    // Arguments:
    // - position: Vec4 position of the listener in 4D world space
    audio_table.set(
        "update_listener",
        lua.create_function(|_, pos: LuaVec4| {
            log::trace!(
                "[audio] update_listener(pos=({:.2}, {:.2}, {:.2}, {:.2})) - stub",
                pos.0.x, pos.0.y, pos.0.z, pos.0.w
            );
            Ok(())
        })?,
    )?;

    // audio.get_listener_position() -> Vec4
    //
    // Get the current listener position.
    //
    // Returns:
    // - Vec4 position of the listener
    audio_table.set(
        "get_listener_position",
        lua.create_function(|_, ()| {
            log::trace!("[audio] get_listener_position() - stub returning ZERO");
            Ok(LuaVec4(rust4d_math::Vec4::ZERO))
        })?,
    )?;

    // Register the audio table as a global
    lua.globals().set("audio", audio_table)?;

    log::debug!("[audio] Audio bindings registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::math;

    fn create_lua_with_audio() -> Lua {
        let lua = Lua::new();
        math::register(&lua).expect("Failed to register math bindings");
        register(&lua).expect("Failed to register audio bindings");
        lua
    }

    #[test]
    fn test_audio_table_exists() {
        let lua = create_lua_with_audio();
        let audio: LuaTable = lua
            .globals()
            .get("audio")
            .expect("audio table should exist");
        assert!(audio.contains_key("load_sound").unwrap());
        assert!(audio.contains_key("play").unwrap());
        assert!(audio.contains_key("play_oneshot").unwrap());
        assert!(audio.contains_key("play_spatial").unwrap());
        assert!(audio.contains_key("play_oneshot_spatial").unwrap());
        assert!(audio.contains_key("set_volume").unwrap());
        assert!(audio.contains_key("stop_all").unwrap());
        assert!(audio.contains_key("stop_bus").unwrap());
        assert!(audio.contains_key("update_listener").unwrap());
        assert!(audio.contains_key("get_listener_position").unwrap());
    }

    #[test]
    fn test_load_sound_returns_handle() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local handle = audio.load_sound("test.ogg")
            assert(type(handle) == "userdata", "should return userdata")
            assert(handle.id == 0, "stub should return id 0")
        "#,
        )
        .exec()
        .expect("load_sound should work");
    }

    #[test]
    fn test_sound_handle_tostring() {
        let lua = create_lua_with_audio();
        let s: String = lua
            .load(
                r#"
            local handle = audio.load_sound("test.ogg")
            return tostring(handle)
        "#,
            )
            .eval()
            .expect("tostring should work");
        assert!(s.contains("SoundHandle"), "should contain SoundHandle");
        assert!(s.contains("0"), "should contain id");
    }

    #[test]
    fn test_sound_handle_equality() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local h1 = audio.load_sound("a.ogg")
            local h2 = audio.load_sound("b.ogg")
            assert(h1 == h2, "both stubs have id 0, should be equal")
        "#,
        )
        .exec()
        .expect("equality should work");
    }

    #[test]
    fn test_play_requires_valid_bus() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local handle = audio.load_sound("test.ogg")
            audio.play(handle, "sfx")  -- valid
            audio.play(handle, "SFX")  -- case insensitive
            audio.play(handle, "music")
            audio.play(handle, "ambient")
            audio.play(handle, "master")
        "#,
        )
        .exec()
        .expect("valid bus names should work");
    }

    #[test]
    fn test_play_invalid_bus_error() {
        let lua = create_lua_with_audio();
        let result: LuaResult<()> = lua
            .load(
                r#"
            local handle = audio.load_sound("test.ogg")
            audio.play(handle, "invalid_bus")
        "#,
            )
            .exec();
        assert!(result.is_err(), "invalid bus should error");
    }

    #[test]
    fn test_play_oneshot() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local handle = audio.load_sound("explosion.ogg")
            audio.play_oneshot(handle, "sfx")
        "#,
        )
        .exec()
        .expect("play_oneshot should work");
    }

    #[test]
    fn test_play_spatial() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local handle = audio.load_sound("explosion.ogg")
            local pos = Vec4.new(10, 5, 3, 1)
            audio.play_spatial(handle, pos, "sfx")
        "#,
        )
        .exec()
        .expect("play_spatial should work");
    }

    #[test]
    fn test_play_oneshot_spatial() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local handle = audio.load_sound("explosion.ogg")
            local pos = Vec4.new(10, 5, 3, 1)
            audio.play_oneshot_spatial(handle, pos, 1.0, 50.0, "sfx")
        "#,
        )
        .exec()
        .expect("play_oneshot_spatial should work");
    }

    #[test]
    fn test_play_oneshot_spatial_invalid_distances() {
        let lua = create_lua_with_audio();

        // min_dist < 0
        let result: LuaResult<()> = lua
            .load(
                r#"
            local handle = audio.load_sound("test.ogg")
            local pos = Vec4.new(0, 0, 0, 0)
            audio.play_oneshot_spatial(handle, pos, -1.0, 50.0, "sfx")
        "#,
            )
            .exec();
        assert!(result.is_err(), "negative min_dist should error");

        // max_dist <= min_dist
        let result: LuaResult<()> = lua
            .load(
                r#"
            local handle = audio.load_sound("test.ogg")
            local pos = Vec4.new(0, 0, 0, 0)
            audio.play_oneshot_spatial(handle, pos, 10.0, 5.0, "sfx")
        "#,
            )
            .exec();
        assert!(result.is_err(), "max_dist <= min_dist should error");
    }

    #[test]
    fn test_set_volume() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            audio.set_volume("sfx", 0.5)
            audio.set_volume("music", 1.0)
            audio.set_volume("master", 0.0)
            audio.set_volume("ambient", 0.75)
        "#,
        )
        .exec()
        .expect("set_volume should work");
    }

    #[test]
    fn test_set_volume_clamps() {
        let lua = create_lua_with_audio();
        // Should not error, just clamp
        lua.load(
            r#"
            audio.set_volume("sfx", 2.0)  -- clamped to 1.0
            audio.set_volume("sfx", -1.0) -- clamped to 0.0
        "#,
        )
        .exec()
        .expect("set_volume should clamp out-of-range values");
    }

    #[test]
    fn test_stop_all() {
        let lua = create_lua_with_audio();
        lua.load("audio.stop_all()")
            .exec()
            .expect("stop_all should work");
    }

    #[test]
    fn test_stop_bus() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            audio.stop_bus("sfx")
            audio.stop_bus("music")
        "#,
        )
        .exec()
        .expect("stop_bus should work");
    }

    #[test]
    fn test_stop_bus_invalid() {
        let lua = create_lua_with_audio();
        let result: LuaResult<()> = lua.load("audio.stop_bus('invalid')").exec();
        assert!(result.is_err(), "invalid bus should error");
    }

    #[test]
    fn test_update_listener() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local pos = Vec4.new(1, 2, 3, 4)
            audio.update_listener(pos)
        "#,
        )
        .exec()
        .expect("update_listener should work");
    }

    #[test]
    fn test_get_listener_position() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            local pos = audio.get_listener_position()
            assert(pos.x == 0 and pos.y == 0 and pos.z == 0 and pos.w == 0, "stub returns ZERO")
        "#,
        )
        .exec()
        .expect("get_listener_position should work");
    }

    #[test]
    fn test_full_audio_workflow() {
        let lua = create_lua_with_audio();
        lua.load(
            r#"
            -- Typical game audio setup

            -- Load sounds
            local music = audio.load_sound("music/theme.ogg")
            local explosion = audio.load_sound("sfx/explosion.ogg")
            local ambient = audio.load_sound("ambient/wind.ogg")

            -- Set up volumes
            audio.set_volume("master", 1.0)
            audio.set_volume("music", 0.7)
            audio.set_volume("sfx", 1.0)
            audio.set_volume("ambient", 0.5)

            -- Play background audio
            audio.play(music, "music")
            audio.play(ambient, "ambient")

            -- Update listener position (camera)
            local camera_pos = Vec4.new(0, 5, 0, 0)
            audio.update_listener(camera_pos)

            -- Play spatial sound effect
            local explosion_pos = Vec4.new(10, 0, 5, 2)
            audio.play_oneshot_spatial(explosion, explosion_pos, 1.0, 100.0, "sfx")

            -- Get listener position back
            local listener = audio.get_listener_position()

            -- Cleanup
            audio.stop_bus("ambient")
        "#,
        )
        .exec()
        .expect("full audio workflow should work");
    }
}
