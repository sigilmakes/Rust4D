//! Audio bus system for mixing and routing

/// Audio bus identifiers for organizing and mixing sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioBus {
    /// Master bus - all other buses route through this
    Master,
    /// Sound effects bus - short, one-shot sounds
    Sfx,
    /// Music bus - background music tracks
    Music,
    /// Ambient bus - environmental audio
    Ambient,
}

impl Default for AudioBus {
    /// Returns `AudioBus::Sfx` as the default.
    ///
    /// Sfx is chosen as the default rather than Master because:
    /// - Most game sounds (explosions, footsteps, UI clicks) are sound effects
    /// - Playing directly on Master bypasses per-category volume control
    /// - Users expect `play_sound(sound)` to respect their SFX volume setting
    /// - Music and Ambient are typically longer-running and set explicitly
    fn default() -> Self {
        Self::Sfx
    }
}

impl AudioBus {
    /// Get the display name for this bus
    pub fn name(&self) -> &'static str {
        match self {
            Self::Master => "Master",
            Self::Sfx => "SFX",
            Self::Music => "Music",
            Self::Ambient => "Ambient",
        }
    }

    /// Get all available buses
    pub fn all() -> &'static [AudioBus] {
        &[Self::Master, Self::Sfx, Self::Music, Self::Ambient]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_names() {
        assert_eq!(AudioBus::Master.name(), "Master");
        assert_eq!(AudioBus::Sfx.name(), "SFX");
        assert_eq!(AudioBus::Music.name(), "Music");
        assert_eq!(AudioBus::Ambient.name(), "Ambient");
    }

    #[test]
    fn test_bus_all() {
        let all = AudioBus::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&AudioBus::Master));
        assert!(all.contains(&AudioBus::Sfx));
        assert!(all.contains(&AudioBus::Music));
        assert!(all.contains(&AudioBus::Ambient));
    }
}
