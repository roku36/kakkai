mod apply;
mod components;
mod hydrate;
mod interact;
mod messages;

pub use apply::FurnitureIndex;
pub use components::{Furniture, FurnitureId};
pub use interact::PlacementState;
pub use messages::{MoveFurniture, PlaceFurniture, RemoveFurniture};

use bevy::prelude::*;
use transform_gizmo_bevy::{GizmoMode, GizmoOptions, TransformGizmoPlugin};

pub fn plugin(app: &mut App) {
    app.add_plugins(TransformGizmoPlugin);
    app.insert_resource(GizmoOptions {
        gizmo_modes: GizmoMode::all_translate() | GizmoMode::all_rotate(),
        ..default()
    });

    app.register_type::<FurnitureId>();
    app.register_type::<Furniture>();

    app.add_message::<PlaceFurniture>();
    app.add_message::<MoveFurniture>();
    app.add_message::<RemoveFurniture>();

    app.init_resource::<FurnitureIndex>();

    app.add_systems(
        Update,
        (
            (apply::apply_place, apply::apply_move, apply::apply_remove),
            hydrate::hydrate_furniture,
        )
            .chain(),
    );
    app.add_plugins(interact::plugin);
}
