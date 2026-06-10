//! Scene transition effects
//!
//! Provides transition effects for smooth scene changes including fade,
//! crossfade, and slide transitions. Each transition tracks its own progress
//! and provides rendering information like alpha values.

use std::time::{Duration, Instant};

/// Direction for slide transitions
#[derive(Clone, Debug, PartialEq)]
pub enum SlideDirection {
    /// Slide to the left
    Left,
    /// Slide to the right
    Right,
    /// Slide upward
    Up,
    /// Slide downward
    Down,
}

/// Transition effect between scenes
#[derive(Clone, Debug)]
pub enum TransitionEffect {
    /// Instant cut (no transition)
    Instant,
    /// Fade to black, then fade in new scene
    Fade {
        /// Total duration of the fade (out + in)
        duration: Duration,
    },
    /// Crossfade between scenes (blend alpha)
    Crossfade {
        /// Duration of the crossfade blend
        duration: Duration,
    },
    /// Slide old scene out, slide new scene in
    Slide {
        /// Duration of the slide transition
        duration: Duration,
        /// Direction of the slide
        direction: SlideDirection,
    },
}

impl TransitionEffect {
    /// Get duration of this effect (Instant returns Duration::ZERO)
    pub fn duration(&self) -> Duration {
        match self {
            TransitionEffect::Instant => Duration::ZERO,
            TransitionEffect::Fade { duration } => *duration,
            TransitionEffect::Crossfade { duration } => *duration,
            TransitionEffect::Slide { duration, .. } => *duration,
        }
    }
}

/// Active transition state tracking progress between two scenes
pub struct SceneTransition {
    /// The transition effect being applied
    effect: TransitionEffect,
    /// Name of the scene being transitioned from
    from_scene: String,
    /// Name of the scene being transitioned to
    to_scene: String,
    /// When the transition started
    start_time: Instant,
    /// Current progress from 0.0 (start) to 1.0 (complete)
    progress: f32,
}

impl SceneTransition {
    /// Create a new scene transition
    ///
    /// The transition begins immediately from the given start time.
    pub fn new(from: String, to: String, effect: TransitionEffect) -> Self {
        Self {
            effect,
            from_scene: from,
            to_scene: to,
            start_time: Instant::now(),
            progress: 0.0,
        }
    }

    /// Update transition progress based on elapsed time
    ///
    /// Returns true when the transition is complete.
    pub fn update(&mut self) -> bool {
        let duration = self.effect.duration();
        if duration.is_zero() {
            self.progress = 1.0;
            return true;
        }

        let elapsed = self.start_time.elapsed();
        self.progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
        self.progress >= 1.0
    }

    /// Get current progress (0.0 = start, 1.0 = complete)
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Get the destination scene name
    pub fn to_scene(&self) -> &str {
        &self.to_scene
    }

    /// Get the source scene name
    pub fn from_scene(&self) -> &str {
        &self.from_scene
    }

    /// Get the transition effect
    pub fn effect(&self) -> &TransitionEffect {
        &self.effect
    }

    /// Get current alpha for rendering fade effects
    ///
    /// For Fade: goes 1.0 -> 0.0 -> 1.0 (fade out old scene in first half,
    /// fade in new scene in second half)
    ///
    /// For Crossfade: goes 0.0 -> 1.0 (blend from old to new)
    ///
    /// For Instant/Slide: always 1.0
    pub fn alpha(&self) -> f32 {
        match &self.effect {
            TransitionEffect::Instant => 1.0,
            TransitionEffect::Fade { .. } => {
                // First half: fade out (1.0 -> 0.0)
                // Second half: fade in (0.0 -> 1.0)
                if self.progress < 0.5 {
                    1.0 - (self.progress * 2.0)
                } else {
                    (self.progress - 0.5) * 2.0
                }
            }
            TransitionEffect::Crossfade { .. } => {
                // Linear blend: 0.0 (all old) -> 1.0 (all new)
                self.progress
            }
            TransitionEffect::Slide { .. } => 1.0,
        }
    }

    /// Check if transition is complete
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_transition_completes_immediately() {
        let mut transition = SceneTransition::new(
            "scene_a".to_string(),
            "scene_b".to_string(),
            TransitionEffect::Instant,
        );
        let complete = transition.update();
        assert!(complete);
        assert!(transition.is_complete());
        assert_eq!(transition.progress(), 1.0);
    }

    #[test]
    fn test_instant_alpha_is_one() {
        let transition =
            SceneTransition::new("a".to_string(), "b".to_string(), TransitionEffect::Instant);
        assert_eq!(transition.alpha(), 1.0);
    }

    #[test]
    fn test_fade_alpha_at_start() {
        // At progress 0.0, alpha should be 1.0 (fully visible old scene)
        let transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Fade {
                duration: Duration::from_secs(2),
            },
        );
        // Progress is 0.0 at creation
        assert_eq!(transition.alpha(), 1.0);
    }

    #[test]
    fn test_fade_alpha_progression() {
        // Manually test fade alpha logic
        // At progress 0.0: alpha = 1.0 - (0.0 * 2.0) = 1.0
        // At progress 0.25: alpha = 1.0 - (0.25 * 2.0) = 0.5
        // At progress 0.5: alpha = (0.5 - 0.5) * 2.0 = 0.0
        // At progress 0.75: alpha = (0.75 - 0.5) * 2.0 = 0.5
        // At progress 1.0: alpha = (1.0 - 0.5) * 2.0 = 1.0

        let mut transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Fade {
                duration: Duration::from_secs(2),
            },
        );

        // Simulate different progress values
        transition.progress = 0.0;
        assert!((transition.alpha() - 1.0).abs() < 0.001);

        transition.progress = 0.25;
        assert!((transition.alpha() - 0.5).abs() < 0.001);

        transition.progress = 0.5;
        assert!((transition.alpha() - 0.0).abs() < 0.001);

        transition.progress = 0.75;
        assert!((transition.alpha() - 0.5).abs() < 0.001);

        transition.progress = 1.0;
        assert!((transition.alpha() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_crossfade_alpha_goes_zero_to_one() {
        let mut transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Crossfade {
                duration: Duration::from_secs(1),
            },
        );

        // At start, alpha is 0.0 (all old scene)
        transition.progress = 0.0;
        assert_eq!(transition.alpha(), 0.0);

        // At midpoint, alpha is 0.5
        transition.progress = 0.5;
        assert!((transition.alpha() - 0.5).abs() < 0.001);

        // At end, alpha is 1.0 (all new scene)
        transition.progress = 1.0;
        assert!((transition.alpha() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_slide_alpha_is_always_one() {
        let mut transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Slide {
                duration: Duration::from_secs(1),
                direction: SlideDirection::Left,
            },
        );

        transition.progress = 0.0;
        assert_eq!(transition.alpha(), 1.0);

        transition.progress = 0.5;
        assert_eq!(transition.alpha(), 1.0);

        transition.progress = 1.0;
        assert_eq!(transition.alpha(), 1.0);
    }

    #[test]
    fn test_progress_increases_over_time() {
        let mut transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Fade {
                duration: Duration::from_millis(100),
            },
        );

        // Right after creation, progress should be near 0
        let initial_progress = transition.progress();
        assert!(
            initial_progress <= 0.1,
            "Initial progress too high: {}",
            initial_progress
        );

        // After updating with some time passed, progress should have increased
        std::thread::sleep(Duration::from_millis(50));
        transition.update();
        assert!(
            transition.progress() > initial_progress,
            "Progress did not increase"
        );
    }

    #[test]
    fn test_transition_completes_after_duration() {
        let mut transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Crossfade {
                duration: Duration::from_millis(50),
            },
        );

        // Wait for it to complete
        std::thread::sleep(Duration::from_millis(60));
        let complete = transition.update();
        assert!(complete);
        assert!(transition.is_complete());
        assert!((transition.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_transition_scene_accessors() {
        let transition = SceneTransition::new(
            "from_scene".to_string(),
            "to_scene".to_string(),
            TransitionEffect::Instant,
        );

        assert_eq!(transition.from_scene(), "from_scene");
        assert_eq!(transition.to_scene(), "to_scene");
    }

    #[test]
    fn test_transition_effect_accessor() {
        let transition = SceneTransition::new(
            "a".to_string(),
            "b".to_string(),
            TransitionEffect::Slide {
                duration: Duration::from_secs(1),
                direction: SlideDirection::Right,
            },
        );

        match transition.effect() {
            TransitionEffect::Slide { direction, .. } => {
                assert_eq!(*direction, SlideDirection::Right);
            }
            _ => panic!("Expected Slide effect"),
        }
    }

    #[test]
    fn test_transition_effect_duration() {
        assert_eq!(TransitionEffect::Instant.duration(), Duration::ZERO);

        assert_eq!(
            TransitionEffect::Fade {
                duration: Duration::from_secs(2)
            }
            .duration(),
            Duration::from_secs(2)
        );

        assert_eq!(
            TransitionEffect::Crossfade {
                duration: Duration::from_millis(500)
            }
            .duration(),
            Duration::from_millis(500)
        );

        assert_eq!(
            TransitionEffect::Slide {
                duration: Duration::from_secs(1),
                direction: SlideDirection::Up,
            }
            .duration(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn test_slide_directions() {
        // Verify all directions are distinct
        assert_ne!(SlideDirection::Left, SlideDirection::Right);
        assert_ne!(SlideDirection::Up, SlideDirection::Down);
        assert_ne!(SlideDirection::Left, SlideDirection::Up);
        assert_eq!(SlideDirection::Left.clone(), SlideDirection::Left);
    }
}
