//! Regression tests for `scenes/gallery.ron`.
//!
//! The gallery is both user-facing content and the canonical scene-file
//! exercise for every 4D primitive variant. If a new `ShapeTemplate` breaks
//! serde, collider hints, or renderable geometry collection, this test fails
//! before someone discovers it in the windowed app.

use rust4d::systems::build_geometry;
use rust4d_core::{SceneManager, ShapeTemplate};
use rust4d_physics::PhysicsConfig;

#[test]
fn gallery_scene_loads_instantiates_and_builds_geometry() {
    let mut manager = SceneManager::new().with_physics(PhysicsConfig::new(-20.0));
    let name = manager
        .load_scene("scenes/gallery.ron")
        .expect("gallery scene should parse");

    let template = manager
        .get_template(&name)
        .expect("gallery template should be registered");
    assert_eq!(template.name, "Shape Gallery");
    assert_eq!(template.entities.len(), 10, "floor + 9 exhibit shapes");

    let variants: Vec<&'static str> = template
        .entities
        .iter()
        .map(|e| match &e.shape {
            ShapeTemplate::Tesseract { .. } => "Tesseract",
            ShapeTemplate::Hyperplane { .. } => "Hyperplane",
            ShapeTemplate::Hypersphere { .. } => "Hypersphere",
            ShapeTemplate::Pentachoron { .. } => "Pentachoron",
            ShapeTemplate::Hexadecachoron { .. } => "Hexadecachoron",
            ShapeTemplate::Icositetrachoron { .. } => "Icositetrachoron",
            ShapeTemplate::Hexacosichoron { .. } => "Hexacosichoron",
            ShapeTemplate::Spherinder { .. } => "Spherinder",
            ShapeTemplate::Cubinder { .. } => "Cubinder",
            ShapeTemplate::Duocylinder { .. } => "Duocylinder",
        })
        .collect();

    for expected in [
        "Tesseract",
        "Hypersphere",
        "Pentachoron",
        "Hexadecachoron",
        "Icositetrachoron",
        "Hexacosichoron",
        "Spherinder",
        "Cubinder",
        "Duocylinder",
    ] {
        assert!(variants.contains(&expected), "gallery missing {expected}");
    }

    manager.instantiate(&name).expect("gallery should instantiate");
    manager.push_scene(&name).expect("gallery should become active");

    let world = manager.active_world().expect("active gallery world");
    let geometry = build_geometry(world);

    assert!(geometry.vertex_count() > 2_000, "gallery should contain substantial geometry");
    assert!(geometry.tetrahedron_count() > 8_000, "gallery should upload all primitive meshes");

    let active = manager.active_scene().unwrap();
    assert_eq!(active.world.entity_count(), 10);
    assert!(
        active.world.physics().is_some(),
        "gallery should create a physics world because the hypersphere is dynamic"
    );
}
