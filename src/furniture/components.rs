use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity that survives save/load and, later, network replication.
/// Always address furniture by this id, never by `Entity`.
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[reflect(opaque, Component, Serialize, Deserialize)]
pub struct FurnitureId(pub Uuid);

impl FurnitureId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A placed piece of furniture. Together with `FurnitureId` and `Transform`
/// this is the entire authoritative world state — visuals are derived in
/// `hydrate.rs` and are never persisted or replicated.
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Debug)]
#[reflect(Component, Serialize, Deserialize)]
#[require(Transform, Visibility)]
pub struct Furniture {
    /// Library-relative model file name (e.g. "chair.glb"), never an
    /// absolute path — this is the portable model identifier.
    pub model: String,
}

/// Marker for the visual child (glTF scene or placeholder) under a furniture root.
#[derive(Component)]
pub struct FurnitureVisual;
