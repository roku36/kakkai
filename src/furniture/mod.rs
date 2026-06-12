mod apply;
mod components;
mod hydrate;
mod interact;
mod messages;
mod undo;

pub use apply::FurnitureIndex;
pub use components::{Furniture, FurnitureId};
pub use interact::PlacementState;
pub use messages::{MoveFurniture, PlaceFurniture, RemoveFurniture};
pub use undo::UndoStack;

use bevy::prelude::*;
use transform_gizmo_bevy::{GizmoMode, GizmoOptions, TransformGizmoPlugin};

pub fn plugin(app: &mut App) {
    app.add_plugins(TransformGizmoPlugin);
    app.insert_resource(GizmoOptions {
        gizmo_modes: GizmoMode::all_translate() | GizmoMode::all_rotate(),
        snapping: true,
        snap_distance: interact::SNAP_DISTANCE,
        snap_angle: std::f32::consts::FRAC_PI_4,
        ..default()
    });

    app.register_type::<FurnitureId>();
    app.register_type::<Furniture>();

    app.add_message::<PlaceFurniture>();
    app.add_message::<MoveFurniture>();
    app.add_message::<RemoveFurniture>();

    app.init_resource::<FurnitureIndex>();
    app.init_resource::<UndoStack>();

    // Maintain the id->entity index from component lifecycle so it stays
    // correct for replicated entities on clients too.
    app.add_observer(
        |add: On<Add, FurnitureId>, ids: Query<&FurnitureId>, mut index: ResMut<FurnitureIndex>| {
            if let Ok(id) = ids.get(add.entity) {
                index.0.insert(*id, add.entity);
            }
        },
    );
    app.add_observer(
        |remove: On<Remove, FurnitureId>,
         ids: Query<&FurnitureId>,
         mut index: ResMut<FurnitureIndex>| {
            if let Ok(id) = ids.get(remove.entity) {
                index.0.remove(id);
            }
        },
    );

    app.add_systems(
        Update,
        (
            undo::undo_redo_input,
            undo::record_actions,
            (apply::apply_place, apply::apply_move, apply::apply_remove),
            hydrate::hydrate_furniture,
        )
            .chain(),
    );
    app.add_plugins(interact::plugin);
}
