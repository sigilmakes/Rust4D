//! Scene validation
//!
//! Validates scenes for common errors before runtime. The [`SceneValidator`]
//! checks for issues like empty scenes, duplicate entity names, unreasonable
//! physics values, and extreme spawn positions.

use std::collections::HashSet;

use crate::scene::Scene;

/// Validation error found in a scene
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Scene has no entities
    EmptyScene,
    /// Duplicate entity name found
    DuplicateName(String),
    /// Entity has no shape (defensive check - shouldn't happen with current types)
    MissingShape(String),
    /// Gravity value seems unreasonable (absolute value > 1000)
    UnreasonableGravity(f32),
    /// Player spawn is at extreme coordinates (any component absolute value > 10000)
    ExtremeSpawnPosition([f32; 4]),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyScene => write!(f, "Scene has no entities"),
            ValidationError::DuplicateName(name) => {
                write!(f, "Duplicate entity name: '{}'", name)
            }
            ValidationError::MissingShape(name) => {
                write!(f, "Entity '{}' has no shape", name)
            }
            ValidationError::UnreasonableGravity(g) => {
                write!(f, "Unreasonable gravity value: {} (abs > 1000)", g)
            }
            ValidationError::ExtremeSpawnPosition(pos) => {
                write!(
                    f,
                    "Extreme spawn position: [{}, {}, {}, {}] (component abs > 10000)",
                    pos[0], pos[1], pos[2], pos[3]
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Scene validator that checks for common errors
///
/// Performs static analysis of a [`Scene`] to detect potential issues
/// before runtime. This includes checking for empty scenes, duplicate
/// entity names, unreasonable physics values, and extreme spawn positions.
///
/// # Example
/// ```ignore
/// let errors = SceneValidator::validate(&scene);
/// if errors.is_empty() {
///     println!("Scene is valid!");
/// } else {
///     for error in &errors {
///         eprintln!("Validation error: {}", error);
///     }
/// }
/// ```
pub struct SceneValidator;

impl SceneValidator {
    /// Validate a scene, returning all errors found
    ///
    /// Returns an empty vector if no validation errors are detected.
    pub fn validate(scene: &Scene) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check for empty scene
        if scene.entities.is_empty() {
            errors.push(ValidationError::EmptyScene);
        }

        // Check for duplicate entity names
        let mut seen_names = HashSet::new();
        for entity in &scene.entities {
            if let Some(ref name) = entity.name {
                if !seen_names.insert(name.clone()) {
                    errors.push(ValidationError::DuplicateName(name.clone()));
                }
            }
        }

        // Check for unreasonable gravity
        if let Some(gravity) = scene.gravity {
            if gravity.abs() > 1000.0 {
                errors.push(ValidationError::UnreasonableGravity(gravity));
            }
        }

        // Check for extreme spawn position
        if let Some(spawn) = scene.player_spawn {
            let is_extreme = spawn.iter().any(|c| c.abs() > 10000.0);
            if is_extreme {
                errors.push(ValidationError::ExtremeSpawnPosition(spawn));
            }
        }

        errors
    }

    /// Validate and return Result (Ok if no errors, Err with all errors)
    ///
    /// This is a convenience method that wraps [`validate`](SceneValidator::validate)
    /// for use in error-handling contexts.
    pub fn validate_or_error(scene: &Scene) -> Result<(), Vec<ValidationError>> {
        let errors = Self::validate(scene);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityTemplate;
    use crate::shapes::ShapeTemplate;
    use crate::{Material, Transform4D};

    fn make_valid_scene() -> Scene {
        let mut scene = Scene::new("Valid Scene")
            .with_gravity(-20.0)
            .with_player_spawn(0.0, 1.0, 5.0, 0.0);
        scene.add_entity(
            EntityTemplate::new(
                ShapeTemplate::tesseract(2.0),
                Transform4D::identity(),
                Material::WHITE,
            )
            .with_name("cube"),
        );
        scene
    }

    #[test]
    fn test_valid_scene_returns_no_errors() {
        let scene = make_valid_scene();
        let errors = SceneValidator::validate(&scene);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_or_error_ok_for_valid_scene() {
        let scene = make_valid_scene();
        assert!(SceneValidator::validate_or_error(&scene).is_ok());
    }

    #[test]
    fn test_empty_scene_error() {
        let scene = Scene::new("Empty");
        let errors = SceneValidator::validate(&scene);
        assert!(errors.contains(&ValidationError::EmptyScene));
    }

    #[test]
    fn test_duplicate_names_detected() {
        let mut scene = Scene::new("Dupes");
        scene.add_entity(
            EntityTemplate::new(
                ShapeTemplate::tesseract(1.0),
                Transform4D::identity(),
                Material::WHITE,
            )
            .with_name("cube"),
        );
        scene.add_entity(
            EntityTemplate::new(
                ShapeTemplate::tesseract(1.0),
                Transform4D::identity(),
                Material::RED,
            )
            .with_name("cube"),
        );

        let errors = SceneValidator::validate(&scene);
        assert!(
            errors.contains(&ValidationError::DuplicateName("cube".to_string())),
            "Expected DuplicateName error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_unreasonable_gravity_detected() {
        let mut scene = Scene::new("High Gravity").with_gravity(-5000.0);
        scene.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(1.0),
            Transform4D::identity(),
            Material::WHITE,
        ));

        let errors = SceneValidator::validate(&scene);
        assert!(
            errors.contains(&ValidationError::UnreasonableGravity(-5000.0)),
            "Expected UnreasonableGravity, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_reasonable_gravity_no_error() {
        let mut scene = Scene::new("Normal").with_gravity(-20.0);
        scene.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(1.0),
            Transform4D::identity(),
            Material::WHITE,
        ));

        let errors = SceneValidator::validate(&scene);
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::UnreasonableGravity(_))),
            "Did not expect gravity error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_extreme_spawn_position_detected() {
        let mut scene = Scene::new("Far Away").with_player_spawn(99999.0, 0.0, 0.0, 0.0);
        scene.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(1.0),
            Transform4D::identity(),
            Material::WHITE,
        ));

        let errors = SceneValidator::validate(&scene);
        assert!(
            errors.contains(&ValidationError::ExtremeSpawnPosition([
                99999.0, 0.0, 0.0, 0.0
            ])),
            "Expected ExtremeSpawnPosition, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_normal_spawn_no_error() {
        let scene = make_valid_scene();
        let errors = SceneValidator::validate(&scene);
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::ExtremeSpawnPosition(_))),
            "Did not expect spawn error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_or_error_returns_err_for_invalid() {
        let scene = Scene::new("Empty");
        let result = SceneValidator::validate_or_error(&scene);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&ValidationError::EmptyScene));
    }

    #[test]
    fn test_multiple_errors_detected() {
        // Scene with multiple issues: empty + unreasonable gravity + extreme spawn
        let scene = Scene::new("Broken")
            .with_gravity(-9999.0)
            .with_player_spawn(50000.0, 0.0, 0.0, 0.0);

        let errors = SceneValidator::validate(&scene);
        assert!(
            errors.len() >= 3,
            "Expected at least 3 errors, got {}: {:?}",
            errors.len(),
            errors
        );
        assert!(errors.contains(&ValidationError::EmptyScene));
        assert!(errors.contains(&ValidationError::UnreasonableGravity(-9999.0)));
        assert!(errors.contains(&ValidationError::ExtremeSpawnPosition([
            50000.0, 0.0, 0.0, 0.0
        ])));
    }

    #[test]
    fn test_unnamed_entities_dont_trigger_duplicate() {
        let mut scene = Scene::new("No Names");
        // Two entities without names should not trigger duplicate name error
        scene.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(1.0),
            Transform4D::identity(),
            Material::WHITE,
        ));
        scene.add_entity(EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::identity(),
            Material::RED,
        ));

        let errors = SceneValidator::validate(&scene);
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateName(_))),
            "Unnamed entities should not trigger duplicate name: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            format!("{}", ValidationError::EmptyScene),
            "Scene has no entities"
        );
        assert_eq!(
            format!("{}", ValidationError::DuplicateName("foo".to_string())),
            "Duplicate entity name: 'foo'"
        );
        assert_eq!(
            format!("{}", ValidationError::MissingShape("bar".to_string())),
            "Entity 'bar' has no shape"
        );
        assert!(format!("{}", ValidationError::UnreasonableGravity(-5000.0)).contains("-5000"));
        assert!(format!(
            "{}",
            ValidationError::ExtremeSpawnPosition([1.0, 2.0, 3.0, 4.0])
        )
        .contains("1, 2, 3, 4"));
    }
}
