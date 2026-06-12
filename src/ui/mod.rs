use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::furniture::PlacementState;
use crate::library::{ImportRequested, ModelLibrary};
use crate::paths::AppDirs;
use crate::states::ControlMode;

/// True while the pointer is over (or captured by) an egui area, so world
/// interaction systems can ignore those clicks.
#[derive(Resource, Default)]
pub struct UiHover(pub bool);

pub fn plugin(app: &mut App) {
    app.add_plugins(EguiPlugin::default());
    app.init_resource::<UiHover>();
    app.add_systems(EguiPrimaryContextPass, library_panel);
}

fn library_panel(
    mut contexts: EguiContexts,
    mode: Res<State<ControlMode>>,
    mut library: ResMut<ModelLibrary>,
    dirs: Res<AppDirs>,
    mut placement: ResMut<PlacementState>,
    mut import: ResMut<ImportRequested>,
    mut ui_hover: ResMut<UiHover>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    if *mode.get() == ControlMode::Build {
        egui::SidePanel::left("library")
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Model Library");
                ui.separator();

                if ui.button("Import model…").clicked() {
                    import.0 = true;
                }
                if ui.button("Rescan").clicked() {
                    crate::library::rescan(&dirs, &mut library);
                }
                ui.separator();

                if library.models.is_empty() {
                    ui.label("No models yet.\nImport a .glb/.gltf, or drop\nfiles into the models folder.");
                    ui.small(dirs.models_dir.display().to_string());
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in &library.models {
                            let selected = placement.model.as_deref() == Some(&entry.file_name);
                            if ui.selectable_label(selected, &entry.file_name).clicked() {
                                placement.model = if selected {
                                    None
                                } else {
                                    Some(entry.file_name.clone())
                                };
                            }
                        }
                    });
                }

                ui.separator();
                ui.label(if placement.snap {
                    "Snap: ON (G)"
                } else {
                    "Snap: OFF (G)"
                });
                ui.separator();
                ui.small("Click a model, then click the\nground to place it.");
                ui.small("R: rotate / Esc: cancel");
                ui.small("Click furniture: gizmo");
                ui.small("D: duplicate selection");
                ui.small("Backspace: delete / Tab: walk");
                ui.small("Cmd+Z: undo / Cmd+Shift+Z: redo");
            });
    } else {
        egui::Area::new(egui::Id::new("hint"))
            .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -12.0])
            .show(ctx, |ui| {
                ui.label("WASD: move / Tab: build mode / Esc: free cursor");
            });
    }

    ui_hover.0 = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
    Ok(())
}
