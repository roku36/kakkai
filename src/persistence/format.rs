use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SAVE_VERSION: u32 = 1;

/// On-disk world state. A deliberate DTO (not reflect-serialized components):
/// the file format stays decoupled from ECS internals and maps 1:1 onto a
/// future server-side database table.
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldSave {
    pub version: u32,
    pub furniture: Vec<FurnitureRecord>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FurnitureRecord {
    pub id: Uuid,
    pub model: String,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl FurnitureRecord {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: self.translation,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
}
