//! World-to-GPU geometry building
//!
//! Converts ECS entities (Transform4D + ShapeRef + Material) into
//! [`RenderableGeometry`] for the slicing pipeline. Shared by the windowed
//! app, integration tests, and headless verification harnesses.

use rust4d_core::{Material, ShapeRef, Tags, Transform4D, World};
use rust4d_render::{position_gradient_color, CheckerboardGeometry, RenderableGeometry};

/// Build GPU geometry from the world using the app's coloring rules:
/// dynamic entities get a position gradient, static entities (floor) get a
/// checkerboard pattern.
pub fn build_geometry(world: &World) -> RenderableGeometry {
    let mut geometry = RenderableGeometry::new();

    // Checkerboard pattern for the floor
    let checkerboard = CheckerboardGeometry::new(
        [0.3, 0.3, 0.35, 1.0], // Dark gray
        [0.7, 0.7, 0.75, 1.0], // Light gray
        2.0,                   // Cell size
    );

    // Query all renderable entities (Transform4D + ShapeRef + Material)
    // Optionally check Tags for coloring strategy
    for (_entity, (transform, shape, material, tags)) in world
        .ecs()
        .query::<(&Transform4D, &ShapeRef, &Material, Option<&Tags>)>()
        .iter()
    {
        let is_dynamic = tags.map(|t| t.has("dynamic")).unwrap_or(false);
        if is_dynamic {
            // Dynamic entities (tesseract): use position gradient
            geometry.add_components_with_color(
                transform,
                shape.as_shape(),
                material,
                &position_gradient_color,
            );
        } else {
            // Static entities (floor): use checkerboard pattern
            geometry.add_components_with_color(transform, shape.as_shape(), material, &|v, _m| {
                checkerboard.color_for_position(v.x, v.z)
            });
        }
    }

    geometry
}
