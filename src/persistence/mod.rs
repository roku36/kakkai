mod format;

pub use format::{FurnitureRecord, SAVE_VERSION, WorldSave};

use bevy::prelude::*;

use crate::furniture::{Furniture, FurnitureId, PlaceFurniture};
use crate::paths::AppDirs;

/// Set by apply.rs whenever furniture state changes; drives debounced saves.
#[derive(Resource, Default)]
pub struct WorldDirty {
    dirty: bool,
    since: f32,
}

impl WorldDirty {
    pub fn mark(&mut self) {
        self.dirty = true;
        self.since = 0.0;
    }
}

/// Save at most this often after a change settles.
const SAVE_DEBOUNCE_SECS: f32 = 2.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<WorldDirty>();
    app.add_systems(PostStartup, load_world);
    app.add_systems(
        Last,
        (debounced_save, save_on_exit.run_if(on_message::<AppExit>)).chain(),
    );
}

fn load_world(
    dirs: Res<AppDirs>,
    mut place: MessageWriter<PlaceFurniture>,
    mut undo: ResMut<crate::furniture::UndoStack>,
) {
    // Restored furniture must not be undoable as if the user had placed it.
    undo.suppress_frames = 2;
    let text = match std::fs::read_to_string(&dirs.save_file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("no save file yet, starting a fresh world");
            return;
        }
        Err(e) => {
            error!("failed to read {:?}: {e}", dirs.save_file);
            return;
        }
    };
    let save: WorldSave = match ron::from_str(&text) {
        Ok(save) => save,
        Err(e) => {
            error!("failed to parse {:?}: {e}", dirs.save_file);
            return;
        }
    };
    if save.version != SAVE_VERSION {
        warn!("save version {} (current {SAVE_VERSION})", save.version);
    }
    info!("loading {} furniture record(s)", save.furniture.len());
    // Loading goes through the same message funnel as gameplay.
    for record in &save.furniture {
        place.write(PlaceFurniture {
            id: FurnitureId(record.id),
            model: record.model.clone(),
            transform: record.transform(),
        });
    }
}

fn debounced_save(
    time: Res<Time>,
    mut dirty: ResMut<WorldDirty>,
    dirs: Res<AppDirs>,
    furniture: Query<(&FurnitureId, &Furniture, &Transform)>,
) {
    if !dirty.dirty {
        return;
    }
    dirty.since += time.delta_secs();
    if dirty.since < SAVE_DEBOUNCE_SECS {
        return;
    }
    dirty.dirty = false;
    save_world(&dirs, &furniture);
}

fn save_on_exit(
    mut dirty: ResMut<WorldDirty>,
    dirs: Res<AppDirs>,
    furniture: Query<(&FurnitureId, &Furniture, &Transform)>,
) {
    if dirty.dirty {
        dirty.dirty = false;
        save_world(&dirs, &furniture);
    }
}

fn save_world(dirs: &AppDirs, furniture: &Query<(&FurnitureId, &Furniture, &Transform)>) {
    let save = WorldSave {
        version: SAVE_VERSION,
        furniture: furniture
            .iter()
            .map(|(id, item, transform)| FurnitureRecord {
                id: id.0,
                model: item.model.clone(),
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            })
            .collect(),
    };
    let text = match ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default()) {
        Ok(text) => text,
        Err(e) => {
            error!("failed to serialize world: {e}");
            return;
        }
    };
    // Atomic write: never leave a truncated save behind.
    let tmp = dirs.save_file.with_extension("ron.tmp");
    let result = std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &dirs.save_file));
    match result {
        Ok(_) => info!("saved {} furniture record(s)", save.furniture.len()),
        Err(e) => error!("failed to write {:?}: {e}", dirs.save_file),
    }
}
