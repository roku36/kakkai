use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, block_on, poll_once};

use super::ModelLibrary;
use crate::paths::AppDirs;

/// Set by the UI (or directly via BRP); consumed by `start_import`.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ImportRequested(pub bool);

/// In-flight file dialog, if any. The dialog runs as an async task so the
/// game loop keeps rendering behind it (rfd dispatches the actual panel to
/// the main thread internally).
#[derive(Resource, Default)]
pub struct PendingImport(Option<Task<Option<PathBuf>>>);

pub fn start_import(mut requested: ResMut<ImportRequested>, mut pending: ResMut<PendingImport>) {
    if !std::mem::take(&mut requested.0) || pending.0.is_some() {
        return;
    }
    pending.0 = Some(IoTaskPool::get().spawn(async {
        rfd::AsyncFileDialog::new()
            .add_filter("glTF model", &["glb", "gltf"])
            .pick_file()
            .await
            .map(|handle| handle.path().to_path_buf())
    }));
}

pub fn poll_import(
    mut pending: ResMut<PendingImport>,
    dirs: Res<AppDirs>,
    mut library: ResMut<ModelLibrary>,
) {
    let Some(task) = pending.0.as_mut() else {
        return;
    };
    let Some(result) = block_on(poll_once(task)) else {
        return;
    };
    pending.0 = None;
    let Some(source) = result else {
        return; // dialog cancelled
    };
    let Some(file_name) = source.file_name() else {
        return;
    };
    let dest = dirs.models_dir.join(file_name);
    match std::fs::copy(&source, &dest) {
        Ok(_) => {
            info!("imported model {:?}", dest.file_name().unwrap_or_default());
            super::rescan(&dirs, &mut library);
        }
        Err(e) => error!("failed to import {source:?}: {e}"),
    }
}
