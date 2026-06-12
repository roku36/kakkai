mod controller;

pub use controller::PlayerCamera;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(controller::plugin);
}
