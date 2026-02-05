//! Spatial query types for physics world
//!
//! Provides types for spatial queries like sphere searches and area effects.

use crate::world::RayTarget;
use rust4d_math::Vec4;

/// Result of a spatial query (e.g., sphere search)
#[derive(Clone, Copy, Debug)]
pub struct SpatialQueryResult {
    /// What was found (body or static collider)
    pub target: RayTarget,
    /// Position of the found object
    pub position: Vec4,
    /// Distance from the query origin to the object
    pub distance: f32,
}

/// Result of an area effect query (explosions, AoE attacks, etc.)
#[derive(Clone, Copy, Debug)]
pub struct AreaEffectHit {
    /// What was hit (body or static collider)
    pub target: RayTarget,
    /// Position of the hit object
    pub position: Vec4,
    /// Distance from the effect center to the object
    pub distance: f32,
    /// Distance falloff factor (1.0 at center, 0.0 at edge when with_falloff is true)
    ///
    /// Use this to scale damage or force based on distance from the explosion center.
    pub falloff: f32,
    /// Normalized direction from effect center toward the target
    ///
    /// Use this for knockback direction calculations.
    pub direction: Vec4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyKey;

    #[test]
    fn test_spatial_query_result_fields() {
        // This test just ensures the struct can be constructed
        let key = BodyKey::default();
        let result = SpatialQueryResult {
            target: RayTarget::Body(key),
            position: Vec4::new(1.0, 2.0, 3.0, 4.0),
            distance: 5.0,
        };
        assert_eq!(result.distance, 5.0);
        assert_eq!(result.position.x, 1.0);
    }

    #[test]
    fn test_area_effect_hit_fields() {
        let key = BodyKey::default();
        let hit = AreaEffectHit {
            target: RayTarget::Body(key),
            position: Vec4::new(5.0, 0.0, 0.0, 0.0),
            distance: 5.0,
            falloff: 0.5,
            direction: Vec4::X,
        };
        assert_eq!(hit.falloff, 0.5);
        assert_eq!(hit.direction, Vec4::X);
    }
}
