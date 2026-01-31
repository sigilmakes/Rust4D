//! Application configuration
//!
//! Configuration is loaded from multiple sources with the following priority (lowest to highest):
//! 1. `config/default.toml` (version controlled)
//! 2. `config/user.toml` (gitignored, user overrides)
//! 3. Environment variables (`R4D_SECTION__KEY`)

use figment::{Figment, providers::{Format, Toml, Env}};
use serde::{Serialize, Deserialize};
use std::path::Path;

// Re-export PhysicsConfig from the physics crate for convenience
pub use rust4d_physics::PhysicsConfig;

/// Main application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Window configuration
    #[serde(default)]
    pub window: WindowConfig,
    /// Camera configuration
    #[serde(default)]
    pub camera: CameraConfig,
    /// Input configuration
    #[serde(default)]
    pub input: InputConfig,
    /// Physics configuration
    #[serde(default)]
    pub physics: PhysicsConfigToml,
    /// Rendering configuration
    #[serde(default)]
    pub rendering: RenderingConfig,
    /// Debug configuration
    #[serde(default)]
    pub debug: DebugConfig,
    /// Scene configuration
    #[serde(default)]
    pub scene: SceneConfig,
}

impl AppConfig {
    /// Load configuration from default locations
    ///
    /// Priority (lowest to highest):
    /// 1. `config/default.toml`
    /// 2. `config/user.toml`
    /// 3. Environment variables (`R4D_*`)
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from("config")
    }

    /// Load configuration from a specific config directory
    pub fn load_from<P: AsRef<Path>>(config_dir: P) -> Result<Self, ConfigError> {
        let config_dir = config_dir.as_ref();
        let default_path = config_dir.join("default.toml");
        let user_path = config_dir.join("user.toml");

        let mut figment = Figment::new();

        // Load default config (required)
        if default_path.exists() {
            log::debug!("Loading default config from {:?}", default_path);
            figment = figment.merge(Toml::file(&default_path));
        } else {
            log::warn!("Default config not found at {:?}", default_path);
        }

        // Load user config (optional)
        if user_path.exists() {
            log::info!("Loading user config from {:?}", user_path);
            figment = figment.merge(Toml::file(&user_path));
        } else {
            log::debug!("No user config at {:?}", user_path);
        }

        // Environment variables override everything
        // R4D_WINDOW__TITLE=Test -> window.title = "Test"
        figment = figment.merge(Env::prefixed("R4D_").split("__"));

        figment.extract().map_err(ConfigError::from)
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Start in fullscreen mode
    pub fullscreen: bool,
    /// Enable VSync
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Rust4D - 4D Rendering Engine".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
            vsync: true,
        }
    }
}

/// Camera configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Starting position [x, y, z, w]
    pub start_position: [f32; 4],
    /// Field of view in degrees
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Maximum pitch angle in degrees
    pub pitch_limit: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            start_position: [0.0, 0.0, 5.0, 0.0],
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            pitch_limit: 89.0,
        }
    }
}

/// Input configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// Movement speed (units per second)
    pub move_speed: f32,
    /// W-axis movement speed (units per second)
    pub w_move_speed: f32,
    /// Mouse sensitivity for 3D rotation
    pub mouse_sensitivity: f32,
    /// Mouse sensitivity for W rotation
    pub w_rotation_sensitivity: f32,
    /// Input smoothing half-life in seconds (lower = more responsive)
    pub smoothing_half_life: f32,
    /// Enable input smoothing by default
    pub smoothing_enabled: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            move_speed: 3.0,
            w_move_speed: 2.0,
            mouse_sensitivity: 0.002,
            w_rotation_sensitivity: 0.005,
            smoothing_half_life: 0.05,
            smoothing_enabled: false,
        }
    }
}

/// Physics configuration from TOML
///
/// This wraps the core PhysicsConfig. The `gravity` and `jump_velocity` fields
/// are passed to the physics engine.
///
/// Note: `player_radius` is in `[scene]` section. Floor positions are defined
/// per-scene in .ron files via Hyperplane entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfigToml {
    /// Gravity (negative = downward)
    pub gravity: f32,
    /// Jump velocity
    pub jump_velocity: f32,
}

impl Default for PhysicsConfigToml {
    fn default() -> Self {
        Self {
            gravity: -20.0,
            jump_velocity: 8.0,
        }
    }
}

impl PhysicsConfigToml {
    /// Convert to the physics engine's PhysicsConfig
    pub fn to_physics_config(&self) -> PhysicsConfig {
        PhysicsConfig::new(self.gravity)
            .with_jump_velocity(self.jump_velocity)
    }
}

/// Rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Maximum triangles for slice output
    pub max_triangles: u32,
    /// Background color [r, g, b, a]
    pub background_color: [f32; 4],
    /// Light direction [x, y, z]
    pub light_dir: [f32; 3],
    /// Ambient light strength
    pub ambient_strength: f32,
    /// Diffuse light strength
    pub diffuse_strength: f32,
    /// W-axis color tinting strength (0.0 = no tint, 1.0 = full tint)
    pub w_color_strength: f32,
    /// W-axis distance for full color effect
    pub w_range: f32,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            max_triangles: 900_000,
            background_color: [0.02, 0.02, 0.08, 1.0],
            light_dir: [0.5, 1.0, 0.3],
            ambient_strength: 0.3,
            diffuse_strength: 0.7,
            w_color_strength: 0.5,
            w_range: 2.0,
        }
    }
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Show debug overlay
    pub show_overlay: bool,
    /// Log level (error, warn, info, debug, trace)
    pub log_level: String,
    /// Show physics colliders
    pub show_colliders: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_overlay: false,
            log_level: "info".to_string(),
            show_colliders: false,
        }
    }
}

/// Scene configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneConfig {
    /// Path to the scene file to load
    pub path: String,
    /// Player collision radius
    pub player_radius: f32,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            path: "scenes/default.ron".to_string(),
            player_radius: 0.5,
        }
    }
}

/// Configuration error
#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl From<figment::Error> for ConfigError {
    fn from(e: figment::Error) -> Self {
        ConfigError {
            message: e.to_string(),
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Configuration error: {}", self.message)
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.window.width, 1280);
        assert_eq!(config.physics.gravity, -20.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("title"));
        assert!(toml.contains("gravity"));
    }
}
