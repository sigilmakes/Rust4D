//! Serializable shape templates
//!
//! ShapeTemplate provides a serializable representation of shapes,
//! solving the trait object serialization problem. Each variant
//! corresponds to a shape type and stores its construction parameters.
//!
//! All shapes are created in **local space** (centered at origin or with bottom at y=0).
//! The entity transform is used to position them in world space.

use rust4d_math::{primitives, ConvexShape4D, Hyperplane4D, Tesseract4D};
use serde::{Deserialize, Serialize};

/// Serializable shape template
///
/// This enum allows shapes to be serialized to/from RON files.
/// Each variant stores the parameters needed to construct the shape.
///
/// **Important:** Shapes are created in local space. Use the entity's transform
/// to position them in world space.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShapeTemplate {
    /// A 4D hypercube (tesseract)
    ///
    /// Created centered at origin with vertices at ±(size/2) on each axis.
    Tesseract {
        /// Full side length of the tesseract
        size: f32,
    },
    /// A floor/ground plane in 4D
    ///
    /// Created in local space with bottom surface at y=0.
    /// The `y` field is used for physics collider placement, NOT for the visual mesh.
    /// Use the entity transform to position the visual mesh.
    Hyperplane {
        /// Y-level for the physics collider (visual mesh uses entity transform)
        y: f32,
        /// Half-extent in X and Z (total size is 2*size)
        size: f32,
        /// Number of cells along each axis
        subdivisions: u32,
        /// Half-extent in W dimension (for slicing visibility)
        cell_size: f32,
        /// Y thickness (bottom at y=0 in local space)
        thickness: f32,
    },
    /// Solid 4-ball bounded by a 3-sphere (S³)
    ///
    /// Its cross-section is a sphere that grows and shrinks as the slice
    /// plane sweeps through — the canonical 4D demo object.
    Hypersphere {
        /// Radius of the 4-ball
        radius: f32,
        /// Boundary refinement level (0–4; cells = 16·8ˢ)
        #[serde(default = "default_sphere_subdivisions")]
        subdivisions: u32,
    },
    /// Regular 5-cell (4-simplex) — the 4D tetrahedron
    Pentachoron {
        /// Circumradius (vertex distance from center)
        circumradius: f32,
    },
    /// Regular 16-cell (4-orthoplex) — the 4D octahedron
    Hexadecachoron {
        /// Circumradius (vertex distance from center)
        circumradius: f32,
    },
    /// Regular 24-cell — 4D's unique extra regular polytope
    Icositetrachoron {
        /// Circumradius (vertex distance from center)
        circumradius: f32,
    },
    /// Regular 600-cell — the 4D icosahedron (600 tetrahedral cells)
    Hexacosichoron {
        /// Circumradius (vertex distance from center)
        circumradius: f32,
    },
    /// Ball × W-segment — the 4D cylinder with a spherical cross-section
    Spherinder {
        /// Radius of the 3-ball
        radius: f32,
        /// Half-length of the W extrusion
        half_height: f32,
        /// Icosphere refinement level (0–5)
        #[serde(default = "default_sphere_subdivisions")]
        subdivisions: u32,
    },
    /// Disk (XY) × square (ZW) — the 4D cylinder with a flat direction pair
    Cubinder {
        /// Disk radius
        radius: f32,
        /// Square half-extent in Z and W
        half_size: f32,
        /// Circle resolution (≥ 3)
        #[serde(default = "default_segments")]
        segments: u32,
    },
    /// Disk × disk — boundary is two solid tori meeting at a Clifford torus
    Duocylinder {
        /// Radius of the XY disk
        radius_xy: f32,
        /// Radius of the ZW disk
        radius_zw: f32,
        /// Circle resolution for both circles (≥ 3)
        #[serde(default = "default_segments")]
        segments: u32,
    },
}

fn default_sphere_subdivisions() -> u32 {
    2
}

fn default_segments() -> u32 {
    24
}

impl ShapeTemplate {
    /// Create the actual shape from this template
    ///
    /// Shapes are created in local space. The entity transform positions them in world space.
    pub fn create_shape(&self) -> Box<dyn ConvexShape4D> {
        match self {
            ShapeTemplate::Tesseract { size } => Box::new(Tesseract4D::new(*size)),
            ShapeTemplate::Hyperplane {
                size,
                subdivisions,
                cell_size,
                thickness,
                ..
            } => {
                // Note: `y` is not passed to the shape constructor - it's used for physics only.
                // The visual mesh is created at y=0 (local space) and positioned by entity transform.
                Box::new(Hyperplane4D::new(
                    *size,
                    *subdivisions as usize,
                    *cell_size,
                    *thickness,
                ))
            }
            ShapeTemplate::Hypersphere {
                radius,
                subdivisions,
            } => Box::new(primitives::hypersphere(*radius, *subdivisions)),
            ShapeTemplate::Pentachoron { circumradius } => {
                Box::new(primitives::pentachoron(*circumradius))
            }
            ShapeTemplate::Hexadecachoron { circumradius } => {
                Box::new(primitives::hexadecachoron(*circumradius))
            }
            ShapeTemplate::Icositetrachoron { circumradius } => {
                Box::new(primitives::icositetrachoron(*circumradius))
            }
            ShapeTemplate::Hexacosichoron { circumradius } => {
                Box::new(primitives::hexacosichoron(*circumradius))
            }
            ShapeTemplate::Spherinder {
                radius,
                half_height,
                subdivisions,
            } => Box::new(primitives::spherinder(*radius, *half_height, *subdivisions)),
            ShapeTemplate::Cubinder {
                radius,
                half_size,
                segments,
            } => Box::new(primitives::cubinder(*radius, *half_size, *segments)),
            ShapeTemplate::Duocylinder {
                radius_xy,
                radius_zw,
                segments,
            } => Box::new(primitives::duocylinder(
                *radius_xy, *radius_zw, *segments, *segments,
            )),
        }
    }

    /// Radius of the smallest origin-centered 4-ball containing the shape.
    ///
    /// Used for physics collider sizing and slice-range culling. Closed-form
    /// per variant — no mesh construction required.
    pub fn bounding_radius(&self) -> f32 {
        match self {
            ShapeTemplate::Tesseract { size } => size * 0.5 * 2.0, // half-diagonal = (s/2)·√4
            ShapeTemplate::Hyperplane {
                size,
                cell_size,
                thickness,
                ..
            } => (2.0 * size * size + cell_size * cell_size + thickness * thickness).sqrt(),
            ShapeTemplate::Hypersphere { radius, .. } => *radius,
            ShapeTemplate::Pentachoron { circumradius }
            | ShapeTemplate::Hexadecachoron { circumradius }
            | ShapeTemplate::Icositetrachoron { circumradius }
            | ShapeTemplate::Hexacosichoron { circumradius } => *circumradius,
            ShapeTemplate::Spherinder {
                radius,
                half_height,
                ..
            } => (radius * radius + half_height * half_height).sqrt(),
            ShapeTemplate::Cubinder {
                radius, half_size, ..
            } => (radius * radius + 2.0 * half_size * half_size).sqrt(),
            ShapeTemplate::Duocylinder {
                radius_xy,
                radius_zw,
                ..
            } => (radius_xy * radius_xy + radius_zw * radius_zw).sqrt(),
        }
    }

    /// Preferred physics collider for this shape: `(is_sphere, radius_or_half_extent)`.
    ///
    /// Round shapes (hypersphere, 600-cell — which is within 2% of its
    /// circumsphere) map to sphere colliders; everything else gets a
    /// conservative AABB from [`Self::bounding_radius`].
    pub fn collider_hint(&self) -> ColliderHint {
        match self {
            ShapeTemplate::Hypersphere { radius, .. } => ColliderHint::Sphere { radius: *radius },
            ShapeTemplate::Hexacosichoron { circumradius } => ColliderHint::Sphere {
                radius: *circumradius,
            },
            ShapeTemplate::Tesseract { size } => ColliderHint::Aabb {
                half_extent: size * 0.5,
            },
            other => ColliderHint::Aabb {
                half_extent: other.bounding_radius() * std::f32::consts::FRAC_1_SQRT_2,
            },
        }
    }

    /// Create a tesseract template
    pub fn tesseract(size: f32) -> Self {
        ShapeTemplate::Tesseract { size }
    }

    /// Create a hyperplane template
    ///
    /// The `y` parameter specifies the Y-level for the physics collider.
    /// The visual mesh is created in local space (y=0) and should be positioned
    /// using the entity transform.
    pub fn hyperplane(
        y: f32,
        size: f32,
        subdivisions: u32,
        cell_size: f32,
        thickness: f32,
    ) -> Self {
        ShapeTemplate::Hyperplane {
            y,
            size,
            subdivisions,
            cell_size,
            thickness,
        }
    }

    /// Create a hypersphere template at default quality
    pub fn hypersphere(radius: f32) -> Self {
        ShapeTemplate::Hypersphere {
            radius,
            subdivisions: 2,
        }
    }

    /// Create a spherinder template at default quality
    pub fn spherinder(radius: f32, half_height: f32) -> Self {
        ShapeTemplate::Spherinder {
            radius,
            half_height,
            subdivisions: 2,
        }
    }

    /// Create a cubinder template at default quality
    pub fn cubinder(radius: f32, half_size: f32) -> Self {
        ShapeTemplate::Cubinder {
            radius,
            half_size,
            segments: 24,
        }
    }

    /// Create a duocylinder template at default quality
    pub fn duocylinder(radius_xy: f32, radius_zw: f32) -> Self {
        ShapeTemplate::Duocylinder {
            radius_xy,
            radius_zw,
            segments: 24,
        }
    }
}

/// Physics collider suggestion derived from a [`ShapeTemplate`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderHint {
    /// Use a sphere collider of this radius
    Sphere {
        /// Sphere radius
        radius: f32,
    },
    /// Use an axis-aligned box collider with this uniform half-extent
    Aabb {
        /// Half-extent on every axis
        half_extent: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_template() {
        let template = ShapeTemplate::tesseract(2.0);
        let shape = template.create_shape();
        assert_eq!(shape.vertex_count(), 16);
    }

    #[test]
    fn test_hyperplane_template() {
        let template = ShapeTemplate::hyperplane(-2.0, 4.0, 2, 2.0, 0.01);
        let shape = template.create_shape();
        // 2x2 grid = 4 cells, each with 16 vertices
        assert_eq!(shape.vertex_count(), 4 * 16);
    }

    #[test]
    fn test_all_primitive_templates_create_valid_shapes() {
        let templates = [
            (ShapeTemplate::hypersphere(1.0), 1024),
            (ShapeTemplate::Pentachoron { circumradius: 1.0 }, 5),
            (ShapeTemplate::Hexadecachoron { circumradius: 1.0 }, 16),
            (ShapeTemplate::Icositetrachoron { circumradius: 1.0 }, 96),
            (ShapeTemplate::Hexacosichoron { circumradius: 1.0 }, 600),
            (ShapeTemplate::spherinder(1.0, 0.5), 1600),
            (ShapeTemplate::cubinder(1.0, 0.5), 24 * 2 * 3 + 4 * 24 * 3),
            (ShapeTemplate::duocylinder(1.0, 1.0), 2 * 24 * 24 * 3),
        ];
        for (template, expected_tets) in templates {
            let shape = template.create_shape();
            assert_eq!(
                shape.tetrahedron_count(),
                expected_tets,
                "template {template:?}"
            );
        }
    }

    #[test]
    fn test_primitive_template_ron_round_trip() {
        let originals = vec![
            ShapeTemplate::hypersphere(1.5),
            ShapeTemplate::Hexacosichoron { circumradius: 0.8 },
            ShapeTemplate::spherinder(1.0, 0.75),
            ShapeTemplate::duocylinder(1.2, 0.9),
        ];
        for original in originals {
            let text = ron::to_string(&original).unwrap();
            let parsed: ShapeTemplate = ron::from_str(&text).unwrap();
            assert_eq!(format!("{original:?}"), format!("{parsed:?}"));
        }
    }

    #[test]
    fn test_ron_defaults_for_resolution_fields() {
        // Scene files may omit resolution fields; they get sane defaults.
        let s: ShapeTemplate = ron::from_str("(type: \"Hypersphere\", radius: 2.0)").unwrap();
        match s {
            ShapeTemplate::Hypersphere {
                radius,
                subdivisions,
            } => {
                assert_eq!(radius, 2.0);
                assert_eq!(subdivisions, 2);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_collider_hints() {
        assert_eq!(
            ShapeTemplate::hypersphere(1.5).collider_hint(),
            ColliderHint::Sphere { radius: 1.5 }
        );
        assert_eq!(
            ShapeTemplate::tesseract(2.0).collider_hint(),
            ColliderHint::Aabb { half_extent: 1.0 }
        );
        match ShapeTemplate::duocylinder(1.0, 1.0).collider_hint() {
            ColliderHint::Aabb { half_extent } => assert!(half_extent > 0.9 && half_extent < 1.1),
            other => panic!("expected AABB, got {other:?}"),
        }
    }

    #[test]
    fn test_tesseract_serialization() {
        let template = ShapeTemplate::tesseract(2.5);
        let serialized = ron::to_string(&template).unwrap();
        let deserialized: ShapeTemplate = ron::from_str(&serialized).unwrap();

        match deserialized {
            ShapeTemplate::Tesseract { size } => assert_eq!(size, 2.5),
            _ => panic!("Expected Tesseract variant"),
        }
    }

    #[test]
    fn test_hyperplane_serialization() {
        let template = ShapeTemplate::hyperplane(-2.0, 4.0, 4, 2.0, 0.01);
        let serialized = ron::to_string(&template).unwrap();
        let deserialized: ShapeTemplate = ron::from_str(&serialized).unwrap();

        match deserialized {
            ShapeTemplate::Hyperplane {
                y,
                size,
                subdivisions,
                cell_size,
                thickness,
            } => {
                assert_eq!(y, -2.0);
                assert_eq!(size, 4.0);
                assert_eq!(subdivisions, 4);
                assert_eq!(cell_size, 2.0);
                assert_eq!(thickness, 0.01);
            }
            _ => panic!("Expected Hyperplane variant"),
        }
    }
}
