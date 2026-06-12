use std::path::PathBuf;

use bevy::prelude::*;

/// Filesystem layout for user data. Everything user-generated lives outside
/// the repo, under the platform data dir (`~/Library/Application Support/kakkai`
/// on macOS). `models_dir` is the backing directory of the `user://` asset source.
#[derive(Resource, Clone, Debug)]
pub struct AppDirs {
    pub models_dir: PathBuf,
    pub save_file: PathBuf,
}

impl AppDirs {
    /// Resolves the data directories and creates them on first run.
    /// Must be called before the asset source is registered.
    pub fn init() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kakkai");
        let models_dir = data_dir.join("models");
        std::fs::create_dir_all(&models_dir)
            .unwrap_or_else(|e| panic!("failed to create data dir {models_dir:?}: {e}"));
        Self {
            save_file: data_dir.join("world.ron"),
            models_dir,
        }
    }
}
