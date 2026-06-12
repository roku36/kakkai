mod import;

pub use import::ImportRequested;

use bevy::prelude::*;

use crate::paths::AppDirs;

/// The user's model collection: every .glb/.gltf in the models dir.
#[derive(Resource, Default)]
pub struct ModelLibrary {
    pub models: Vec<ModelEntry>,
}

#[derive(Clone, Debug)]
pub struct ModelEntry {
    /// File name inside the models dir, e.g. "chair.glb".
    pub file_name: String,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<ModelLibrary>();
    app.init_resource::<ImportRequested>();
    app.init_resource::<import::PendingImport>();
    app.add_systems(
        Startup,
        |dirs: Res<AppDirs>, mut library: ResMut<ModelLibrary>| {
            rescan(&dirs, &mut library);
        },
    );
    app.add_systems(Update, (import::start_import, import::poll_import).chain());
}

pub fn rescan(dirs: &AppDirs, library: &mut ModelLibrary) {
    let mut models: Vec<ModelEntry> = std::fs::read_dir(&dirs.models_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| {
                    let lower = name.to_lowercase();
                    lower.ends_with(".glb") || lower.ends_with(".gltf")
                })
                .map(|file_name| ModelEntry { file_name })
                .collect()
        })
        .unwrap_or_default();
    models.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    info!("model library: {} model(s)", models.len());
    library.models = models;
}
