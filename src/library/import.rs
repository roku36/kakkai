use bevy::prelude::*;

use super::ModelLibrary;
use crate::paths::AppDirs;

/// Set by the UI; consumed by `run_import_dialog`.
#[derive(Resource, Default)]
pub struct ImportRequested(pub bool);

/// Opens a blocking native file dialog and copies the chosen model into the
/// library. Exclusive system: rfd must run on the main thread on macOS, and
/// the game freezing during the modal dialog is fine for v1.
pub fn run_import_dialog(world: &mut World) {
    if !std::mem::take(&mut world.resource_mut::<ImportRequested>().0) {
        return;
    }
    let Some(source) = rfd::FileDialog::new()
        .add_filter("glTF model", &["glb", "gltf"])
        .pick_file()
    else {
        return;
    };
    let Some(file_name) = source.file_name() else {
        return;
    };
    let dirs = world.resource::<AppDirs>().clone();
    let dest = dirs.models_dir.join(file_name);
    match std::fs::copy(&source, &dest) {
        Ok(_) => {
            info!("imported model {:?}", dest.file_name().unwrap_or_default());
            let mut library = world.resource_mut::<ModelLibrary>();
            super::rescan(&dirs, &mut library);
        }
        Err(e) => error!("failed to import {source:?}: {e}"),
    }
}
