use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_world);
    app.add_systems(Update, draw_grid);
}

/// The walkable floor. Placement raycasts are filtered to this entity.
#[derive(Component)]
pub struct Ground;

pub const GROUND_SIZE: f32 = 100.0;

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Ground"),
        Ground,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.42, 0.38),
            perceptual_roughness: 0.95,
            ..default()
        })),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 8_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -0.9)),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        ..default()
    });
    commands.insert_resource(ClearColor(Color::srgb(0.55, 0.7, 0.85)));
}

fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Isometry3d::new(
            Vec3::new(0.0, 0.01, 0.0),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        UVec2::splat(GROUND_SIZE as u32),
        Vec2::splat(1.0),
        Color::srgba(1.0, 1.0, 1.0, 0.08),
    );
}
