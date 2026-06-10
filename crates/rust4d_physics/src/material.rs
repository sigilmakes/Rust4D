//! Physical material properties for collision response

/// Physical material properties for collision response
///
/// Materials define how objects interact during collisions, including
/// friction (how much objects resist sliding) and restitution (bounciness).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial {
    /// Friction coefficient (0.0 = ice, 1.0 = rubber)
    pub friction: f32,
    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.5,
            restitution: 0.0,
        }
    }
}

impl PhysicsMaterial {
    /// Ice-like material: very low friction, slight bounce
    pub const ICE: Self = Self {
        friction: 0.05,
        restitution: 0.1,
    };

    /// Rubber-like material: high friction, very bouncy
    pub const RUBBER: Self = Self {
        friction: 0.9,
        restitution: 0.8,
    };

    /// Metal-like material: moderate friction and bounce
    pub const METAL: Self = Self {
        friction: 0.3,
        restitution: 0.3,
    };

    /// Wood-like material: moderate friction, low bounce
    pub const WOOD: Self = Self {
        friction: 0.5,
        restitution: 0.2,
    };

    /// Concrete-like material: high friction, very low bounce
    pub const CONCRETE: Self = Self {
        friction: 0.7,
        restitution: 0.1,
    };

    /// Create a new physics material with custom friction and restitution
    ///
    /// Values are clamped to the range [0.0, 1.0].
    pub fn new(friction: f32, restitution: f32) -> Self {
        Self {
            friction: friction.clamp(0.0, 1.0),
            restitution: restitution.clamp(0.0, 1.0),
        }
    }

    /// Combine two materials for collision response
    ///
    /// Uses geometric mean for friction (models surface interaction well)
    /// and maximum for restitution (most bouncy surface wins).
    pub fn combine(&self, other: &Self) -> Self {
        Self {
            friction: (self.friction * other.friction).sqrt(),
            restitution: self.restitution.max(other.restitution),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_material() {
        let material = PhysicsMaterial::default();
        assert_eq!(material.friction, 0.5);
        assert_eq!(material.restitution, 0.0);
    }

    #[test]
    fn test_new_clamps_values() {
        let material = PhysicsMaterial::new(1.5, -0.5);
        assert_eq!(material.friction, 1.0);
        assert_eq!(material.restitution, 0.0);

        let material = PhysicsMaterial::new(-1.0, 2.0);
        assert_eq!(material.friction, 0.0);
        assert_eq!(material.restitution, 1.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // deliberate sanity checks on preset values
    fn test_preset_constants() {
        // Test that presets have expected properties
        assert!(PhysicsMaterial::ICE.friction < 0.1);
        assert!(PhysicsMaterial::RUBBER.friction > 0.8);
        assert!(PhysicsMaterial::RUBBER.restitution > 0.7);
        assert!(PhysicsMaterial::CONCRETE.friction > 0.6);
        assert!(PhysicsMaterial::CONCRETE.restitution < 0.2);
    }

    #[test]
    fn test_combine_geometric_mean_friction() {
        let ice = PhysicsMaterial::ICE;
        let rubber = PhysicsMaterial::RUBBER;
        let combined = ice.combine(&rubber);

        // Geometric mean of 0.05 and 0.9 = sqrt(0.045) ≈ 0.212
        let expected_friction = (0.05_f32 * 0.9_f32).sqrt();
        assert!((combined.friction - expected_friction).abs() < 0.0001);
    }

    #[test]
    fn test_combine_max_restitution() {
        let metal = PhysicsMaterial::METAL; // restitution 0.3
        let rubber = PhysicsMaterial::RUBBER; // restitution 0.8
        let combined = metal.combine(&rubber);

        // Max of 0.3 and 0.8 = 0.8
        assert_eq!(combined.restitution, 0.8);
    }

    #[test]
    fn test_combine_is_commutative() {
        let a = PhysicsMaterial::new(0.3, 0.5);
        let b = PhysicsMaterial::new(0.7, 0.2);

        let ab = a.combine(&b);
        let ba = b.combine(&a);

        assert!((ab.friction - ba.friction).abs() < 0.0001);
        assert_eq!(ab.restitution, ba.restitution);
    }

    #[test]
    fn test_combine_same_material() {
        let material = PhysicsMaterial::new(0.5, 0.3);
        let combined = material.combine(&material);

        // sqrt(0.5 * 0.5) = 0.5
        assert_eq!(combined.friction, 0.5);
        assert_eq!(combined.restitution, 0.3);
    }
}
