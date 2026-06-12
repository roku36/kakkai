use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_replicon::prelude::{FromClient, Replicated};

use super::components::{Furniture, FurnitureId};
use super::messages::{MoveFurniture, PlaceFurniture, RemoveFurniture};
use crate::persistence::WorldDirty;

/// Lookup from stable id to the live ECS entity. Maintained by component
/// lifecycle observers (see furniture/mod.rs), so it works on clients with
/// replicated entities just as well as on the authority.
#[derive(Resource, Default)]
pub struct FurnitureIndex(pub HashMap<FurnitureId, Entity>);

/// Keep furniture from sinking below the floor. Floating is allowed (e.g.
/// on top of other furniture); going underground never is.
fn validate(mut transform: Transform) -> Transform {
    transform.translation.y = transform.translation.y.max(0.0);
    transform
}

/// The single writer of furniture state. These systems consume
/// `FromClient<M>` messages, which only exist on the authority: locally
/// written messages on the server/singleplayer convert in place, while
/// messages written on a connected client travel over the network first.
/// This is where validation (ownership, bounds) lives.
pub fn apply_place(
    mut messages: MessageReader<FromClient<PlaceFurniture>>,
    mut commands: Commands,
    index: Res<FurnitureIndex>,
    mut dirty: ResMut<WorldDirty>,
) {
    for msg in messages.read() {
        if index.0.contains_key(&msg.id) {
            warn!("PlaceFurniture for existing id {:?}, ignoring", msg.id);
            continue;
        }
        commands.spawn((
            Name::new(format!("Furniture({})", msg.model)),
            msg.id,
            Furniture {
                model: msg.model.clone(),
            },
            validate(msg.transform),
            Replicated,
        ));
        dirty.mark();
    }
}

pub fn apply_move(
    mut messages: MessageReader<FromClient<MoveFurniture>>,
    mut transforms: Query<&mut Transform, With<Furniture>>,
    index: Res<FurnitureIndex>,
    mut dirty: ResMut<WorldDirty>,
) {
    for msg in messages.read() {
        let Some(&entity) = index.0.get(&msg.id) else {
            warn!("MoveFurniture for unknown id {:?}", msg.id);
            continue;
        };
        if let Ok(mut transform) = transforms.get_mut(entity) {
            *transform = validate(msg.transform);
            dirty.mark();
        }
    }
}

pub fn apply_remove(
    mut messages: MessageReader<FromClient<RemoveFurniture>>,
    mut commands: Commands,
    index: Res<FurnitureIndex>,
    mut dirty: ResMut<WorldDirty>,
    mut selected: ResMut<super::interact::Selected>,
) {
    for msg in messages.read() {
        let Some(&entity) = index.0.get(&msg.id) else {
            warn!("RemoveFurniture for unknown id {:?}", msg.id);
            continue;
        };
        // Removal can come from anywhere (undo, network) — never leave a
        // dangling selection behind.
        if selected.0 == Some(entity) {
            selected.0 = None;
        }
        commands.entity(entity).despawn();
        dirty.mark();
    }
}
