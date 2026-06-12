use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy::window::PrimaryWindow;
use transform_gizmo_bevy::GizmoTarget;

use super::components::{Furniture, FurnitureId};
use super::messages::{MoveFurniture, PlaceFurniture, RemoveFurniture};
use crate::player::PlayerCamera;
use crate::states::ControlMode;
use crate::ui::UiHover;
use crate::world::Ground;

/// Local placement intent (Build mode). Not authoritative state — confirming
/// a placement goes through the `PlaceFurniture` message like everything else.
/// Reflected so BRP clients (inspector, MCP agents) can read and drive it.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct PlacementState {
    /// Library model currently being placed, if any.
    pub model: Option<String>,
    /// Yaw applied to the preview (R rotates by 45°).
    pub rotation: f32,
    pub preview: Option<Entity>,
}

/// Currently selected furniture root entity.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct Selected(pub Option<Entity>);

pub fn plugin(app: &mut App) {
    app.init_resource::<PlacementState>();
    app.init_resource::<Selected>();
    app.register_type::<PlacementState>();
    app.register_type::<Selected>();
    app.add_systems(
        Update,
        (
            sync_preview,
            drive_preview,
            rotate_preview,
            cancel_placement,
            delete_selected,
        )
            .run_if(in_state(ControlMode::Build)),
    );
    app.add_systems(Update, (sync_gizmo_target, emit_move_on_drag_end));
    app.add_systems(OnExit(ControlMode::Build), clear_build_state);
    app.add_observer(select_on_click);
}

/// Keep the preview entity in sync with the selected library model.
/// Guarded with a `Local` snapshot instead of change detection: this system
/// mutates `PlacementState` itself, which would re-trigger `is_changed`
/// every frame and respawn the preview forever.
fn sync_preview(
    mut placement: ResMut<PlacementState>,
    mut current_model: Local<Option<String>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if *current_model == placement.model {
        return;
    }
    *current_model = placement.model.clone();
    if let Some(preview) = placement.preview.take() {
        commands.entity(preview).despawn();
    }
    if let Some(model) = placement.model.clone() {
        let scene: Handle<Scene> = asset_server
            .load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(format!("user://{model}")));
        let preview = commands
            .spawn((
                Name::new("PlacementPreview"),
                SceneRoot(scene),
                Transform::default(),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .observe(ignore_picking_on_scene_children)
            .observe(super::hydrate::strip_cameras_on_scene_ready)
            .id();
        placement.preview = Some(preview);
    }
}

/// Once the preview glTF scene has spawned its children, exclude the whole
/// tree from picking so it never blocks placement/selection raycasts.
fn ignore_picking_on_scene_children(
    ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    for child in children.iter_descendants(ready.entity) {
        // try_insert: a sibling observer may despawn parts of the scene
        // (e.g. embedded cameras) in the same frame.
        commands.entity(child).try_insert(Pickable::IGNORE);
    }
}

/// Move the preview to the ground point under the cursor; confirm with click.
fn drive_preview(
    placement: Res<PlacementState>,
    ui_hover: Res<UiHover>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    ground: Query<(), With<Ground>>,
    mut ray_cast: MeshRayCast,
    mut previews: Query<(&mut Transform, &mut Visibility)>,
    mut place: MessageWriter<PlaceFurniture>,
) {
    let Some(preview) = placement.preview else {
        return;
    };
    let Some(model) = placement.model.clone() else {
        return;
    };
    let Ok((mut preview_transform, mut visibility)) = previews.get_mut(preview) else {
        return;
    };

    let cursor = window.single().ok().and_then(Window::cursor_position);
    let ray = cursor.and_then(|cursor| {
        let (camera, camera_transform) = camera.single().ok()?;
        camera.viewport_to_world(camera_transform, cursor).ok()
    });
    let hit = ray.and_then(|ray| {
        let filter = |entity: Entity| ground.contains(entity);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        ray_cast.cast_ray(ray, &settings).first().cloned()
    });

    let Some((_, hit)) = hit else {
        *visibility = Visibility::Hidden;
        return;
    };
    if ui_hover.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Inherited;
    *preview_transform = Transform {
        translation: hit.point,
        rotation: Quat::from_rotation_y(placement.rotation),
        ..default()
    };

    if buttons.just_pressed(MouseButton::Left) {
        place.write(PlaceFurniture {
            id: FurnitureId::new(),
            model,
            transform: *preview_transform,
        });
    }
}

fn rotate_preview(keys: Res<ButtonInput<KeyCode>>, mut placement: ResMut<PlacementState>) {
    if keys.just_pressed(KeyCode::KeyR) && placement.model.is_some() {
        placement.rotation += std::f32::consts::FRAC_PI_4;
    }
}

fn cancel_placement(keys: Res<ButtonInput<KeyCode>>, mut placement: ResMut<PlacementState>) {
    if keys.just_pressed(KeyCode::Escape) && placement.model.is_some() {
        placement.model = None;
        placement.rotation = 0.0;
    }
}

fn clear_build_state(mut placement: ResMut<PlacementState>, mut selected: ResMut<Selected>) {
    placement.model = None;
    placement.rotation = 0.0;
    selected.0 = None;
}

/// Picking clicks bubble up glTF hierarchies, but the `Furniture` root itself
/// has no mesh, so resolve the clicked mesh to its furniture ancestor by hand.
fn select_on_click(
    click: On<Pointer<Click>>,
    mode: Res<State<ControlMode>>,
    placement: Res<PlacementState>,
    ui_hover: Res<UiHover>,
    furniture: Query<(), With<Furniture>>,
    parents: Query<&ChildOf>,
    gizmos: Query<&GizmoTarget>,
    windows: Query<(), With<Window>>,
    mut selected: ResMut<Selected>,
) {
    // Every click also fires a window-targeted event; without this filter it
    // would immediately overwrite a successful mesh selection with None.
    if windows.contains(click.entity) {
        return;
    }
    if *mode.get() != ControlMode::Build || placement.model.is_some() || ui_hover.0 {
        return;
    }
    // Don't steal the selection while the user is interacting with the gizmo.
    if gizmos.iter().any(|t| t.is_focused() || t.is_active()) {
        return;
    }
    if click.button != PointerButton::Primary {
        return;
    }
    let mut current = click.entity;
    let root = loop {
        if furniture.contains(current) {
            break Some(current);
        }
        match parents.get(current) {
            Ok(parent) => current = parent.parent(),
            Err(_) => break None,
        }
    };
    debug!(
        "click on {:?} resolved furniture root {root:?}",
        click.entity
    );
    selected.0 = root;
}

/// Attach the transform gizmo to the selected furniture root.
fn sync_gizmo_target(
    selected: Res<Selected>,
    targets: Query<Entity, With<GizmoTarget>>,
    mut commands: Commands,
) {
    if !selected.is_changed() {
        return;
    }
    for entity in &targets {
        if Some(entity) != selected.0 {
            commands.entity(entity).remove::<GizmoTarget>();
        }
    }
    if let Some(entity) = selected.0 {
        commands.entity(entity).insert(GizmoTarget::default());
    }
}

/// The gizmo mutates `Transform` live during a drag (presentation latency);
/// the authoritative state is confirmed through `MoveFurniture` on release.
fn emit_move_on_drag_end(
    targets: Query<(&FurnitureId, &Transform, &GizmoTarget)>,
    mut was_active: Local<bool>,
    mut move_msg: MessageWriter<MoveFurniture>,
) {
    let mut any_active = false;
    for (id, transform, target) in &targets {
        if target.is_active() {
            any_active = true;
        } else if *was_active {
            move_msg.write(MoveFurniture {
                id: *id,
                transform: *transform,
            });
        }
    }
    *was_active = any_active;
}

fn delete_selected(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<Selected>,
    ids: Query<&FurnitureId>,
    mut remove: MessageWriter<RemoveFurniture>,
) {
    if !(keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Delete)) {
        return;
    }
    let Some(entity) = selected.0.take() else {
        return;
    };
    if let Ok(id) = ids.get(entity) {
        remove.write(RemoveFurniture { id: *id });
    }
}
