// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
mod dev;
mod furniture;
mod library;
mod paths;
mod persistence;
mod player;
mod states;
mod ui;
mod world;

use bevy::asset::AssetMetaCheck;
use bevy::asset::io::AssetSourceBuilder;
use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let app_dirs = paths::AppDirs::init();
        // The `user://` asset source serves the player's model library.
        // Must be registered before AssetPlugin (inside DefaultPlugins).
        app.register_asset_source(
            "user",
            AssetSourceBuilder::platform_default(
                app_dirs
                    .models_dir
                    .to_str()
                    .expect("models dir must be valid UTF-8"),
                None,
            ),
        );
        app.insert_resource(app_dirs);

        app.add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "kakkai".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
            MeshPickingPlugin,
        ));

        app.add_plugins((
            states::plugin,
            world::plugin,
            player::plugin,
            furniture::plugin,
            library::plugin,
            persistence::plugin,
            ui::plugin,
            #[cfg(feature = "dev")]
            dev::plugin,
        ));
    }
}
