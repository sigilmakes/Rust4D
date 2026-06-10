//! Tween/interpolation system for smooth animations
//!
//! This module provides a tween system for animating entity properties over time.
//!
//! # Overview
//!
//! - [`EasingFunction`] - Various easing curves (linear, quad, cubic)
//! - [`Tween<T>`] - A single tween that interpolates between two values
//! - [`TweenManager`] - Manages active tweens for entities
//!
//! # Example
//!
//! ```ignore
//! use rust4d_game::tween::{Tween, EasingFunction};
//!
//! // Create a position tween
//! let mut tween = Tween::new(0.0f32, 100.0, 1.0, EasingFunction::EaseInOutQuad);
//!
//! // Update each frame
//! let current_value = tween.update(delta_time);
//! ```

use rust4d_math::{Interpolatable, Vec4};
use std::collections::HashMap;

/// Easing functions for tweens
///
/// Easing functions transform a linear time value into a curved one,
/// creating more natural-feeling animations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EasingFunction {
    /// Linear interpolation (no easing)
    #[default]
    Linear,
    /// Quadratic ease-in (slow start)
    EaseInQuad,
    /// Quadratic ease-out (slow end)
    EaseOutQuad,
    /// Quadratic ease-in-out (slow start and end)
    EaseInOutQuad,
    /// Cubic ease-in (slower start)
    EaseInCubic,
    /// Cubic ease-out (slower end)
    EaseOutCubic,
    /// Cubic ease-in-out (slower start and end)
    EaseInOutCubic,
}

impl EasingFunction {
    /// Apply the easing function to a linear time value
    ///
    /// # Arguments
    /// * `t` - Linear time value, typically in range [0.0, 1.0]
    ///
    /// # Returns
    /// The eased value, also in range [0.0, 1.0] for inputs in that range
    #[inline]
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => t * (2.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::EaseInCubic => t * t * t,
            Self::EaseOutCubic => {
                let t1 = t - 1.0;
                t1 * t1 * t1 + 1.0
            }
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t1 = 2.0 * t - 2.0;
                    0.5 * t1 * t1 * t1 + 1.0
                }
            }
        }
    }

    /// Parse an easing function from a string (for Lua API)
    ///
    /// Accepts various formats:
    /// - `"linear"`
    /// - `"ease_in_quad"` or `"easeinquad"`
    /// - `"ease_out_quad"` or `"easeoutquad"`
    /// - etc.
    ///
    /// # Returns
    /// `Some(EasingFunction)` if recognized, `None` otherwise
    ///
    /// # Note on API Design
    ///
    /// This method returns `Option<Self>` rather than implementing `std::str::FromStr`
    /// because:
    /// - The Lua API benefits from `Option` (easy to provide defaults)
    /// - Error details aren't useful here (there's only one failure mode)
    /// - This avoids the ceremony of a custom error type
    ///
    /// If you need `FromStr` for integration with other Rust APIs (e.g., serde, clap),
    /// you can use the [`std::str::FromStr`] implementation which wraps this method.
    #[allow(clippy::should_implement_trait)] // FromStr IS implemented; this is the documented Option-returning convenience
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Some(Self::Linear),
            "ease_in_quad" | "easeinquad" | "in_quad" | "inquad" => Some(Self::EaseInQuad),
            "ease_out_quad" | "easeoutquad" | "out_quad" | "outquad" => Some(Self::EaseOutQuad),
            "ease_in_out_quad" | "easeinoutquad" | "in_out_quad" | "inoutquad" => {
                Some(Self::EaseInOutQuad)
            }
            "ease_in_cubic" | "easeincubic" | "in_cubic" | "incubic" => Some(Self::EaseInCubic),
            "ease_out_cubic" | "easeoutcubic" | "out_cubic" | "outcubic" => Some(Self::EaseOutCubic),
            "ease_in_out_cubic" | "easeinoutcubic" | "in_out_cubic" | "inoutcubic" => {
                Some(Self::EaseInOutCubic)
            }
            _ => None,
        }
    }
}

/// Error type for parsing easing function from string
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseEasingError {
    /// The input string that failed to parse
    pub input: String,
}

impl std::fmt::Display for ParseEasingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown easing function '{}', expected one of: linear, ease_in_quad, \
             ease_out_quad, ease_in_out_quad, ease_in_cubic, ease_out_cubic, ease_in_out_cubic",
            self.input
        )
    }
}

impl std::error::Error for ParseEasingError {}

impl std::str::FromStr for EasingFunction {
    type Err = ParseEasingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EasingFunction::from_str(s).ok_or_else(|| ParseEasingError {
            input: s.to_string(),
        })
    }
}

impl EasingFunction {
    /// Get the string name of this easing function
    pub fn name(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseInQuad => "ease_in_quad",
            Self::EaseOutQuad => "ease_out_quad",
            Self::EaseInOutQuad => "ease_in_out_quad",
            Self::EaseInCubic => "ease_in_cubic",
            Self::EaseOutCubic => "ease_out_cubic",
            Self::EaseInOutCubic => "ease_in_out_cubic",
        }
    }
}

/// State of a tween
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TweenState {
    /// Tween is actively running
    Running,
    /// Tween is paused (can be resumed)
    Paused,
    /// Tween has finished
    Completed,
}

/// A tween that interpolates a value over time
///
/// Generic over any type that implements [`Interpolatable`].
#[derive(Clone, Debug)]
pub struct Tween<T: Interpolatable> {
    from: T,
    to: T,
    duration: f32,
    elapsed: f32,
    easing: EasingFunction,
    state: TweenState,
}

impl<T: Interpolatable> Tween<T> {
    /// Create a new tween
    ///
    /// # Arguments
    /// * `from` - Starting value
    /// * `to` - Ending value
    /// * `duration` - Duration in seconds (clamped to >= 0)
    /// * `easing` - Easing function to use
    ///
    /// # Zero Duration Behavior
    ///
    /// When `duration` is 0 (or negative, which is clamped to 0):
    /// - The tween starts in [`TweenState::Running`] state
    /// - [`current()`](Self::current) immediately returns the `to` value
    /// - The first call to [`update()`](Self::update) completes the tween
    ///
    /// This "instant completion on first update" behavior is intentional:
    /// - It allows the tween system to fire completion callbacks
    /// - The caller's update loop handles the transition uniformly
    /// - Starting as `Running` means `is_running()` returns true before the first update
    ///
    /// If you need truly instant completion (no update required), check the duration
    /// and handle the zero case before creating the tween.
    pub fn new(from: T, to: T, duration: f32, easing: EasingFunction) -> Self {
        Self {
            from,
            to,
            duration: duration.max(0.0),
            elapsed: 0.0,
            easing,
            state: TweenState::Running,
        }
    }

    /// Update the tween by `dt` seconds
    ///
    /// Returns the current interpolated value after advancing time.
    /// If the tween is paused or completed, returns the current value
    /// without advancing time.
    pub fn update(&mut self, dt: f32) -> T {
        if self.state == TweenState::Running {
            self.elapsed += dt;
            if self.elapsed >= self.duration {
                self.elapsed = self.duration;
                self.state = TweenState::Completed;
            }
        }
        self.current()
    }

    /// Get the current interpolated value without advancing time
    pub fn current(&self) -> T {
        let linear_t = if self.duration > 0.0 {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let eased_t = self.easing.apply(linear_t);
        T::lerp(&self.from, &self.to, eased_t)
    }

    /// Get the current progress as a value from 0.0 to 1.0
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Get the current state of the tween
    pub fn state(&self) -> TweenState {
        self.state
    }

    /// Check if the tween has completed
    pub fn is_complete(&self) -> bool {
        self.state == TweenState::Completed
    }

    /// Check if the tween is currently running
    pub fn is_running(&self) -> bool {
        self.state == TweenState::Running
    }

    /// Pause the tween
    pub fn pause(&mut self) {
        if self.state == TweenState::Running {
            self.state = TweenState::Paused;
        }
    }

    /// Resume a paused tween
    pub fn resume(&mut self) {
        if self.state == TweenState::Paused {
            self.state = TweenState::Running;
        }
    }

    /// Reset the tween to its initial state
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.state = TweenState::Running;
    }

    /// Get the starting value
    pub fn from_value(&self) -> &T {
        &self.from
    }

    /// Get the ending value
    pub fn to_value(&self) -> &T {
        &self.to
    }

    /// Get the duration in seconds
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Get the elapsed time in seconds
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Get the easing function
    pub fn easing(&self) -> EasingFunction {
        self.easing
    }
}

/// Unique identifier for a tween
pub type TweenId = u64;

/// Manages active position tweens for entities
///
/// This manager tracks tweens by entity and provides an interface for
/// starting, updating, and cancelling tweens.
///
/// # Current Limitations
///
/// The manager currently only supports **position tweens** (animating [`Transform4D::position`]).
/// Other properties that could benefit from tweening are not yet supported:
///
/// - **Rotation** (`Rotor4`): Would need separate tracking via `rotation_tweens` map
/// - **Scale** (`Vec4` or `f32`): Would need `scale_tweens` map
/// - **Color** (`[f32; 4]`): Useful for fade effects, would need `color_tweens`
/// - **Custom properties**: Could be supported via a generic property system
///
/// Each additional property type would require:
/// 1. A new `HashMap<Entity, (TweenId, Tween<T>)>` field
/// 2. A `tween_<property>()` method to start tweens
/// 3. Update logic in `update()` to apply values to components
///
/// For now, rotation and scale changes should be applied directly or via
/// custom tween handling outside this manager.
pub struct TweenManager {
    /// Position tweens indexed by entity
    position_tweens: HashMap<hecs::Entity, (TweenId, Tween<Vec4>)>,
    /// Next tween ID to assign
    next_id: TweenId,
}

impl Default for TweenManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TweenManager {
    /// Create a new empty tween manager
    pub fn new() -> Self {
        Self {
            position_tweens: HashMap::new(),
            next_id: 1,
        }
    }

    /// Start a position tween for an entity
    ///
    /// If the entity already has a position tween, it is replaced.
    ///
    /// # Arguments
    /// * `entity` - The entity to tween
    /// * `from` - Starting position
    /// * `to` - Ending position
    /// * `duration` - Duration in seconds
    /// * `easing` - Easing function to use
    ///
    /// # Returns
    /// A unique ID for this tween (can be used with callbacks)
    pub fn tween_position(
        &mut self,
        entity: hecs::Entity,
        from: Vec4,
        to: Vec4,
        duration: f32,
        easing: EasingFunction,
    ) -> TweenId {
        let id = self.next_id;
        self.next_id += 1;

        let tween = Tween::new(from, to, duration, easing);
        self.position_tweens.insert(entity, (id, tween));

        id
    }

    /// Update all active tweens and apply to world
    ///
    /// # Arguments
    /// * `dt` - Delta time in seconds
    /// * `world` - The hecs world to update transforms in
    ///
    /// # Returns
    /// Vector of tween IDs that completed this frame
    pub fn update(&mut self, dt: f32, world: &mut hecs::World) -> Vec<TweenId> {
        let mut completed = Vec::new();
        let mut to_remove = Vec::new();

        for (entity, (id, tween)) in &mut self.position_tweens {
            let new_position = tween.update(dt);

            // Apply to entity's transform if it exists
            if let Ok(mut transform) = world.get::<&mut rust4d_core::Transform4D>(*entity) {
                transform.position = new_position;
            }

            if tween.is_complete() {
                completed.push(*id);
                to_remove.push(*entity);
            }
        }

        // Remove completed tweens
        for entity in to_remove {
            self.position_tweens.remove(&entity);
        }

        completed
    }

    /// Cancel a tween by entity
    ///
    /// Does nothing if the entity has no active tween.
    pub fn cancel(&mut self, entity: hecs::Entity) {
        self.position_tweens.remove(&entity);
    }

    /// Pause a tween by entity
    ///
    /// Does nothing if the entity has no active tween.
    pub fn pause(&mut self, entity: hecs::Entity) {
        if let Some((_, tween)) = self.position_tweens.get_mut(&entity) {
            tween.pause();
        }
    }

    /// Resume a paused tween by entity
    ///
    /// Does nothing if the entity has no active tween.
    pub fn resume(&mut self, entity: hecs::Entity) {
        if let Some((_, tween)) = self.position_tweens.get_mut(&entity) {
            tween.resume();
        }
    }

    /// Check if an entity has an active position tween
    pub fn has_tween(&self, entity: hecs::Entity) -> bool {
        self.position_tweens.contains_key(&entity)
    }

    /// Get the current progress of an entity's position tween
    ///
    /// Returns `None` if the entity has no active tween.
    pub fn get_progress(&self, entity: hecs::Entity) -> Option<f32> {
        self.position_tweens.get(&entity).map(|(_, t)| t.progress())
    }

    /// Get the number of active tweens
    pub fn active_count(&self) -> usize {
        self.position_tweens.len()
    }

    /// Clear all active tweens
    pub fn clear(&mut self) {
        self.position_tweens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        assert_eq!(EasingFunction::Linear.apply(0.0), 0.0);
        assert_eq!(EasingFunction::Linear.apply(0.5), 0.5);
        assert_eq!(EasingFunction::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn test_easing_boundaries() {
        // All easings should be 0 at t=0 and 1 at t=1
        let easings = [
            EasingFunction::Linear,
            EasingFunction::EaseInQuad,
            EasingFunction::EaseOutQuad,
            EasingFunction::EaseInOutQuad,
            EasingFunction::EaseInCubic,
            EasingFunction::EaseOutCubic,
            EasingFunction::EaseInOutCubic,
        ];

        for easing in easings {
            assert!(
                easing.apply(0.0).abs() < 0.001,
                "{:?} at 0.0 = {}",
                easing,
                easing.apply(0.0)
            );
            assert!(
                (easing.apply(1.0) - 1.0).abs() < 0.001,
                "{:?} at 1.0 = {}",
                easing,
                easing.apply(1.0)
            );
        }
    }

    #[test]
    fn test_easing_in_out_symmetry() {
        // In-out easings should be 0.5 at t=0.5
        assert!(
            (EasingFunction::EaseInOutQuad.apply(0.5) - 0.5).abs() < 0.001,
            "EaseInOutQuad at 0.5 = {}",
            EasingFunction::EaseInOutQuad.apply(0.5)
        );
        assert!(
            (EasingFunction::EaseInOutCubic.apply(0.5) - 0.5).abs() < 0.001,
            "EaseInOutCubic at 0.5 = {}",
            EasingFunction::EaseInOutCubic.apply(0.5)
        );
    }

    #[test]
    fn test_easing_in_slow_start() {
        // Ease-in should be slower at the start (below linear)
        let t = 0.25;
        assert!(
            EasingFunction::EaseInQuad.apply(t) < t,
            "EaseInQuad at {} should be below linear",
            t
        );
        assert!(
            EasingFunction::EaseInCubic.apply(t) < t,
            "EaseInCubic at {} should be below linear",
            t
        );
    }

    #[test]
    fn test_easing_out_slow_end() {
        // Ease-out should be faster at the start (above linear)
        let t = 0.25;
        assert!(
            EasingFunction::EaseOutQuad.apply(t) > t,
            "EaseOutQuad at {} should be above linear",
            t
        );
        assert!(
            EasingFunction::EaseOutCubic.apply(t) > t,
            "EaseOutCubic at {} should be above linear",
            t
        );
    }

    #[test]
    fn test_easing_from_str() {
        assert_eq!(
            EasingFunction::from_str("linear"),
            Some(EasingFunction::Linear)
        );
        assert_eq!(
            EasingFunction::from_str("ease_in_quad"),
            Some(EasingFunction::EaseInQuad)
        );
        assert_eq!(
            EasingFunction::from_str("easeinquad"),
            Some(EasingFunction::EaseInQuad)
        );
        assert_eq!(
            EasingFunction::from_str("EASE_OUT_CUBIC"),
            Some(EasingFunction::EaseOutCubic)
        );
        assert!(EasingFunction::from_str("invalid").is_none());
        assert!(EasingFunction::from_str("").is_none());
    }

    #[test]
    fn test_easing_name() {
        assert_eq!(EasingFunction::Linear.name(), "linear");
        assert_eq!(EasingFunction::EaseInQuad.name(), "ease_in_quad");
        assert_eq!(EasingFunction::EaseInOutCubic.name(), "ease_in_out_cubic");
    }

    #[test]
    fn test_tween_completes() {
        let mut tween = Tween::new(0.0f32, 10.0, 1.0, EasingFunction::Linear);

        assert!(!tween.is_complete());
        assert!(tween.is_running());
        assert_eq!(tween.state(), TweenState::Running);

        tween.update(0.5);
        assert!(!tween.is_complete());
        assert!((tween.progress() - 0.5).abs() < 0.001);

        tween.update(0.5);
        assert!(tween.is_complete());
        assert!((tween.current() - 10.0).abs() < 0.001);
        assert!((tween.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_tween_zero_duration() {
        let mut tween = Tween::new(0.0f32, 10.0, 0.0, EasingFunction::Linear);

        // Zero duration should immediately complete
        let value = tween.update(0.0);
        assert!((value - 10.0).abs() < 0.001);
        assert!(tween.is_complete());
    }

    #[test]
    fn test_tween_pause_resume() {
        let mut tween = Tween::new(0.0f32, 10.0, 1.0, EasingFunction::Linear);

        tween.update(0.25);
        let progress_before = tween.progress();

        tween.pause();
        assert_eq!(tween.state(), TweenState::Paused);

        // Update while paused should not change progress
        tween.update(0.25);
        assert!((tween.progress() - progress_before).abs() < 0.001);

        tween.resume();
        assert_eq!(tween.state(), TweenState::Running);

        tween.update(0.25);
        assert!(tween.progress() > progress_before);
    }

    #[test]
    fn test_tween_reset() {
        let mut tween = Tween::new(0.0f32, 10.0, 1.0, EasingFunction::Linear);

        tween.update(1.0);
        assert!(tween.is_complete());

        tween.reset();
        assert!(tween.is_running());
        assert!((tween.progress() - 0.0).abs() < 0.001);
        assert!((tween.current() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_vec4_tween() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 20.0, 30.0, 40.0);

        let mut tween = Tween::new(a, b, 1.0, EasingFunction::Linear);

        let mid = tween.update(0.5);
        assert!((mid.x - 5.0).abs() < 0.001);
        assert!((mid.y - 10.0).abs() < 0.001);
        assert!((mid.z - 15.0).abs() < 0.001);
        assert!((mid.w - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_tween_with_easing() {
        let mut tween = Tween::new(0.0f32, 100.0, 1.0, EasingFunction::EaseInQuad);

        tween.update(0.5);

        // At t=0.5, EaseInQuad gives 0.25, so value should be 25.0
        assert!((tween.current() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_tween_accessors() {
        let tween = Tween::new(5.0f32, 15.0, 2.0, EasingFunction::EaseOutQuad);

        assert!((tween.from_value() - 5.0).abs() < 0.001);
        assert!((tween.to_value() - 15.0).abs() < 0.001);
        assert!((tween.duration() - 2.0).abs() < 0.001);
        assert!((tween.elapsed() - 0.0).abs() < 0.001);
        assert_eq!(tween.easing(), EasingFunction::EaseOutQuad);
    }

    #[test]
    fn test_tween_manager_new() {
        let manager = TweenManager::new();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_tween_manager_basic() {
        let mut manager = TweenManager::new();
        let mut world = hecs::World::new();

        // Spawn an entity with a transform
        let entity = world.spawn((rust4d_core::Transform4D::identity(),));

        // Start a tween
        let tween_id = manager.tween_position(
            entity,
            Vec4::ZERO,
            Vec4::new(10.0, 0.0, 0.0, 0.0),
            1.0,
            EasingFunction::Linear,
        );

        assert!(tween_id > 0);
        assert!(manager.has_tween(entity));
        assert_eq!(manager.active_count(), 1);

        // Update halfway
        let completed = manager.update(0.5, &mut world);
        assert!(completed.is_empty());

        // Check position was updated
        {
            let transform = world.get::<&rust4d_core::Transform4D>(entity).unwrap();
            assert!((transform.position.x - 5.0).abs() < 0.001);
        }

        // Update to completion
        let completed = manager.update(0.5, &mut world);
        assert_eq!(completed, vec![tween_id]);
        assert!(!manager.has_tween(entity));
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_tween_manager_cancel() {
        let mut manager = TweenManager::new();
        let entity = hecs::Entity::DANGLING; // Doesn't need to exist for this test

        manager.tween_position(
            entity,
            Vec4::ZERO,
            Vec4::new(10.0, 0.0, 0.0, 0.0),
            1.0,
            EasingFunction::Linear,
        );

        assert!(manager.has_tween(entity));

        manager.cancel(entity);
        assert!(!manager.has_tween(entity));
    }

    #[test]
    fn test_tween_manager_pause_resume() {
        let mut manager = TweenManager::new();
        let mut world = hecs::World::new();
        let entity = world.spawn((rust4d_core::Transform4D::identity(),));

        manager.tween_position(
            entity,
            Vec4::ZERO,
            Vec4::new(10.0, 0.0, 0.0, 0.0),
            1.0,
            EasingFunction::Linear,
        );

        // Update a bit
        manager.update(0.25, &mut world);
        let progress_before = manager.get_progress(entity).unwrap();

        // Pause
        manager.pause(entity);
        manager.update(0.25, &mut world);

        // Progress should not have changed
        assert!((manager.get_progress(entity).unwrap() - progress_before).abs() < 0.001);

        // Resume
        manager.resume(entity);
        manager.update(0.25, &mut world);

        // Progress should have increased
        assert!(manager.get_progress(entity).unwrap() > progress_before);
    }

    #[test]
    fn test_tween_manager_replace() {
        let mut manager = TweenManager::new();
        let entity = hecs::Entity::DANGLING;

        let id1 = manager.tween_position(
            entity,
            Vec4::ZERO,
            Vec4::new(10.0, 0.0, 0.0, 0.0),
            1.0,
            EasingFunction::Linear,
        );

        let id2 = manager.tween_position(
            entity,
            Vec4::new(5.0, 0.0, 0.0, 0.0),
            Vec4::new(20.0, 0.0, 0.0, 0.0),
            2.0,
            EasingFunction::EaseInQuad,
        );

        // Should have replaced, IDs should be different
        assert_ne!(id1, id2);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_tween_manager_clear() {
        let mut manager = TweenManager::new();
        let mut world = hecs::World::new();

        for _ in 0..5 {
            let entity = world.spawn((rust4d_core::Transform4D::identity(),));
            manager.tween_position(
                entity,
                Vec4::ZERO,
                Vec4::X,
                1.0,
                EasingFunction::Linear,
            );
        }

        assert_eq!(manager.active_count(), 5);

        manager.clear();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_tween_manager_multiple_entities() {
        let mut manager = TweenManager::new();
        let mut world = hecs::World::new();

        let e1 = world.spawn((rust4d_core::Transform4D::identity(),));
        let e2 = world.spawn((rust4d_core::Transform4D::identity(),));

        manager.tween_position(e1, Vec4::ZERO, Vec4::X * 10.0, 1.0, EasingFunction::Linear);
        manager.tween_position(e2, Vec4::ZERO, Vec4::Y * 20.0, 2.0, EasingFunction::Linear);

        assert_eq!(manager.active_count(), 2);

        // After 1 second, e1 should complete but e2 should not
        let completed = manager.update(1.0, &mut world);
        assert_eq!(completed.len(), 1);
        assert!(!manager.has_tween(e1));
        assert!(manager.has_tween(e2));

        // Check transforms
        let t1 = world.get::<&rust4d_core::Transform4D>(e1).unwrap();
        let t2 = world.get::<&rust4d_core::Transform4D>(e2).unwrap();
        assert!((t1.position.x - 10.0).abs() < 0.001);
        assert!((t2.position.y - 10.0).abs() < 0.001); // Halfway
    }
}
