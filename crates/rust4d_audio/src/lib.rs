//! 4D Audio System for Rust4D
//!
//! This crate provides spatial 4D audio support using the kira audio library.
//!
//! ## Core Types
//!
//! - [`AudioEngine4D`] - Main audio engine with spatial support
//! - [`AudioBus`] - Audio bus identifiers for mixing
//! - [`SpatialConfig`] - Configuration for spatial audio playback
//! - [`SoundHandle`] - Handle to a loaded sound
//!
//! ## Example
//!
//! ```ignore
//! use rust4d_audio::{AudioEngine4D, AudioBus, SpatialConfig};
//! use rust4d_math::Vec4;
//!
//! let mut engine = AudioEngine4D::new()?;
//! let sound = engine.load_sound("explosion.ogg")?;
//!
//! // Play at a position in 4D space
//! let config = SpatialConfig::new(Vec4::new(10.0, 0.0, 5.0, 0.0));
//! engine.play_spatial(&sound, config, AudioBus::Sfx)?;
//! ```

mod bus;
mod sound;
mod spatial;

pub use bus::AudioBus;
pub use sound::SoundHandle;
pub use spatial::SpatialConfig;

use kira::manager::backend::DefaultBackend;
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::track::{TrackBuilder, TrackHandle};
use kira::tween::Tween;
use kira::Volume;
use rust4d_math::Vec4;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur in the audio system
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to initialize audio manager: {0}")]
    ManagerInit(String),

    #[error("Failed to load sound from path '{path}': {message}")]
    LoadSound { path: String, message: String },

    #[error("Failed to play sound: {0}")]
    PlaySound(String),

    #[error("Failed to create audio track: {0}")]
    TrackCreation(String),

    #[error("Sound not found")]
    SoundNotFound,
}

/// Main audio engine with 4D spatial support
pub struct AudioEngine4D {
    manager: AudioManager<DefaultBackend>,
    bus_tracks: HashMap<AudioBus, TrackHandle>,
    listener_position: Vec4,
    sounds: HashMap<u64, StaticSoundData>,
    next_sound_id: u64,
}

impl AudioEngine4D {
    /// Create a new audio engine
    pub fn new() -> Result<Self, AudioError> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e: kira::manager::backend::cpal::Error| AudioError::ManagerInit(e.to_string()))?;

        let mut engine = Self {
            manager,
            bus_tracks: HashMap::new(),
            listener_position: Vec4::ZERO,
            sounds: HashMap::new(),
            next_sound_id: 0,
        };

        // Create bus tracks
        engine.create_bus_tracks()?;

        log::info!("AudioEngine4D initialized successfully");
        Ok(engine)
    }

    /// Create the audio bus tracks
    fn create_bus_tracks(&mut self) -> Result<(), AudioError> {
        for bus in [
            AudioBus::Master,
            AudioBus::Sfx,
            AudioBus::Music,
            AudioBus::Ambient,
        ] {
            let track = self
                .manager
                .add_sub_track(TrackBuilder::new())
                .map_err(|e| AudioError::TrackCreation(e.to_string()))?;
            self.bus_tracks.insert(bus, track);
        }
        Ok(())
    }

    /// Load a sound from a file path
    pub fn load_sound(&mut self, path: &str) -> Result<SoundHandle, AudioError> {
        let sound_data = StaticSoundData::from_file(path)
            .map_err(|e| AudioError::LoadSound {
                path: path.to_string(),
                message: e.to_string(),
            })?;

        let id = self.next_sound_id;
        self.next_sound_id += 1;
        self.sounds.insert(id, sound_data);

        log::debug!("Loaded sound '{}' with id {}", path, id);
        Ok(SoundHandle::new(id))
    }

    /// Play a sound on a bus (non-spatial)
    pub fn play(&mut self, sound: &SoundHandle, bus: AudioBus) -> Result<(), AudioError> {
        let sound_data = self
            .sounds
            .get(&sound.id())
            .ok_or(AudioError::SoundNotFound)?;

        let track = self
            .bus_tracks
            .get(&bus)
            .ok_or_else(|| AudioError::PlaySound("Bus track not found".to_string()))?;

        let settings = StaticSoundSettings::new().output_destination(track);
        let sound_with_settings = sound_data.with_settings(settings);

        self.manager
            .play(sound_with_settings)
            .map_err(|e| AudioError::PlaySound(e.to_string()))?;

        Ok(())
    }

    /// Play a sound once on a bus (non-spatial, one-shot)
    pub fn play_oneshot(&mut self, sound: &SoundHandle, bus: AudioBus) -> Result<(), AudioError> {
        // For now, oneshot is the same as play since kira handles cleanup
        self.play(sound, bus)
    }

    /// Play a sound with spatial positioning in 4D space
    pub fn play_spatial(
        &mut self,
        sound: &SoundHandle,
        config: SpatialConfig,
        bus: AudioBus,
    ) -> Result<(), AudioError> {
        let sound_data = self
            .sounds
            .get(&sound.id())
            .ok_or(AudioError::SoundNotFound)?;

        let track = self
            .bus_tracks
            .get(&bus)
            .ok_or_else(|| AudioError::PlaySound("Bus track not found".to_string()))?;

        // Calculate volume based on 4D distance
        let volume = spatial::calculate_attenuation(self.listener_position, &config);

        // Calculate stereo panning based on XZ projection
        let panning = spatial::calculate_panning(self.listener_position, &config);

        let settings = StaticSoundSettings::new()
            .output_destination(track)
            .volume(Volume::Amplitude(volume as f64))
            .panning(panning as f64);

        let sound_with_settings = sound_data.with_settings(settings);

        self.manager
            .play(sound_with_settings)
            .map_err(|e| AudioError::PlaySound(e.to_string()))?;

        log::trace!(
            "Playing spatial sound at {:?}, volume: {:.2}, panning: {:.2}",
            config.position,
            volume,
            panning
        );

        Ok(())
    }

    /// Play a sound once with spatial positioning in 4D space
    pub fn play_oneshot_spatial(
        &mut self,
        sound: &SoundHandle,
        config: SpatialConfig,
        bus: AudioBus,
    ) -> Result<(), AudioError> {
        // For now, oneshot is the same as play_spatial since kira handles cleanup
        self.play_spatial(sound, config, bus)
    }

    /// Set the volume of a specific bus
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32) {
        if let Some(track) = self.bus_tracks.get_mut(&bus) {
            let _ = track.set_volume(Volume::Amplitude(volume.clamp(0.0, 1.0) as f64), Tween::default());
            log::debug!("Set {:?} bus volume to {:.2}", bus, volume);
        }
    }

    /// Set the master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.set_bus_volume(AudioBus::Master, volume);
    }

    /// Update the listener position (call each frame)
    pub fn update_listener(&mut self, position: Vec4) {
        self.listener_position = position;
    }

    /// Get the current listener position
    pub fn listener_position(&self) -> Vec4 {
        self.listener_position
    }

    /// Stop all sounds
    pub fn stop_all(&mut self) {
        // kira doesn't have a direct "stop all" on manager,
        // we'd need to track individual sound handles for this
        log::warn!("stop_all: Not fully implemented - would need sound handle tracking");
    }

    /// Stop all sounds on a specific bus
    pub fn stop_bus(&mut self, bus: AudioBus) {
        // Similar limitation as stop_all
        log::warn!("stop_bus {:?}: Not fully implemented - would need sound handle tracking", bus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_bus_default() {
        assert_eq!(AudioBus::default(), AudioBus::Sfx);
    }

    #[test]
    fn test_spatial_config_creation() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let config = SpatialConfig::new(pos);
        assert_eq!(config.position, pos);
        assert_eq!(config.min_distance, 1.0);
        assert_eq!(config.max_distance, 50.0);
    }

    #[test]
    fn test_spatial_config_builder() {
        let config = SpatialConfig::new(Vec4::ZERO)
            .with_min_distance(2.0)
            .with_max_distance(100.0);
        assert_eq!(config.min_distance, 2.0);
        assert_eq!(config.max_distance, 100.0);
    }

    #[test]
    fn test_sound_handle_id() {
        let handle = SoundHandle::new(42);
        assert_eq!(handle.id(), 42);
    }

    // Note: AudioEngine4D::new() requires audio hardware, so we can't easily test it
    // in CI environments. These tests would need to be integration tests or use mocking.
}
