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
    // Bevy Remote Protocol (port 15702) + screenshot/input injection, so AI
    // agents can inspect and drive the app via the bevy_brp_mcp MCP server.
    // BrpExtrasPlugin adds RemotePlugin/RemoteHttpPlugin itself.
    #[cfg(feature = "dev_native")]
    app.add_plugins(bevy_brp_extras::BrpExtrasPlugin::default());

    app.add_systems(Update, debug_raycast);
}

/// Right-click: grid-scan rays over the viewport and log which entities are
/// hittable anywhere (and the cursor ray's own hits).
fn debug_raycast(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<crate::player::PlayerCamera>>,
    names: Query<&Name>,
    mut ray_cast: bevy::picking::mesh_picking::ray_cast::MeshRayCast,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(win) = window.single() else { return };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let settings = bevy::picking::mesh_picking::ray_cast::MeshRayCastSettings::default();

    if let Some(cursor) = win.cursor_position()
        && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
    {
        let hits = ray_cast.cast_ray(ray, &settings);
        info!("cursor ray {cursor:?}: {} hit(s)", hits.len());
        for (entity, hit) in hits {
            info!(
                "  hit {:?} ({:?}) dist {:.2}",
                entity,
                names.get(*entity).map(|n| n.as_str()).unwrap_or("?"),
                hit.distance
            );
        }
    }

    use std::collections::BTreeMap;
    let size = win.size();
    let mut summary: BTreeMap<String, (u32, Vec2)> = BTreeMap::new();
    for gx in 0..32 {
        for gy in 0..24 {
            let p = Vec2::new(
                (gx as f32 + 0.5) / 32.0 * size.x,
                (gy as f32 + 0.5) / 24.0 * size.y,
            );
            let Ok(ray) = camera.viewport_to_world(camera_transform, p) else {
                continue;
            };
            if let Some((entity, _)) = ray_cast.cast_ray(ray, &settings).first() {
                let name = names
                    .get(*entity)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| format!("{entity:?}"));
                let e = summary.entry(name).or_insert((0, p));
                e.0 += 1;
            }
        }
    }
    info!("grid scan (32x24): {summary:?}");
}
