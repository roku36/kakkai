use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub fn plugin(app: &mut App) {
    app.add_plugins(FpsOverlayPlugin::default());
    // Toggle the world inspector with F12.
    app.add_plugins(
        WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F12)),
    );
}
