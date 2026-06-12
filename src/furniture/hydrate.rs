use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;

use super::components::{Furniture, FurnitureVisual};
use crate::paths::AppDirs;

/// User-supplied glTF files may embed cameras (e.g. exporter defaults);
/// rendering through them is never what we want, so drop them on spawn.
pub fn strip_cameras_on_scene_ready(
    ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    cameras: Query<(), With<Camera>>,
    mut commands: Commands,
) {
    for child in children.iter_descendants(ready.entity) {
        if cameras.contains(child) {
            // try_despawn: an ancestor may already have been despawned.
            commands.entity(child).try_despawn();
        }
    }
}

/// State -> visuals. Spawns the glTF scene (or a placeholder when the model
/// file is missing) as a child of the furniture root. Runs on insert and on
/// model swap; a future multiplayer client runs this unchanged on replicated
/// entities.
pub fn hydrate_furniture(
    furniture: Query<(Entity, &Furniture), Changed<Furniture>>,
    children: Query<&Children>,
    visuals: Query<(), With<FurnitureVisual>>,
    asset_server: Res<AssetServer>,
    app_dirs: Res<AppDirs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for (entity, item) in &furniture {
        // Drop the previous visual on model swap.
        if let Ok(existing) = children.get(entity) {
            for child in existing {
                if visuals.contains(*child) {
                    commands.entity(*child).despawn();
                }
            }
        }

        if app_dirs.models_dir.join(&item.model).exists() {
            let scene: Handle<Scene> = asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(format!("user://{}", item.model)));
            let visual = commands
                .spawn((
                    Name::new("Visual"),
                    FurnitureVisual,
                    SceneRoot(scene),
                    ChildOf(entity),
                    // Static colliders for every mesh in the glTF (colliders
                    // without a RigidBody are static in avian).
                    ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
                ))
                .observe(strip_cameras_on_scene_ready)
                .id();
            debug!("hydrated furniture {entity:?} with visual {visual:?}");
        } else {
            warn!(
                "model file '{}' not found in library, spawning placeholder",
                item.model
            );
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Name::new("Placeholder"),
                    FurnitureVisual,
                    Mesh3d(meshes.add(Cuboid::from_length(0.5))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.9, 0.3, 0.6),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.25, 0.0),
                    Collider::cuboid(0.5, 0.5, 0.5),
                ));
            });
        }
    }
}
