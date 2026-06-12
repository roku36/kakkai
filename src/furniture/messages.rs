use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::components::FurnitureId;

// These messages are the only way furniture state changes (see apply.rs).
// They derive Serialize/Deserialize on purpose: routed through a transport
// instead of a local MessageWriter, they become the client->server protocol.

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub struct PlaceFurniture {
    pub id: FurnitureId,
    pub model: String,
    pub transform: Transform,
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub struct MoveFurniture {
    pub id: FurnitureId,
    pub transform: Transform,
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub struct RemoveFurniture {
    pub id: FurnitureId,
}
