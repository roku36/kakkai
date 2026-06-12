use avian3d::prelude::{Collider, Friction, LinearVelocity, LockedAxes, RigidBody};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use transform_gizmo_bevy::GizmoCamera;

use crate::states::ControlMode;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
    app.add_systems(
        Update,
        (
            grab_cursor_on_click.run_if(in_state(ControlMode::Walk)),
            release_cursor_on_escape,
            mouse_look.run_if(in_state(ControlMode::Walk)),
            walk.run_if(in_state(ControlMode::Walk)),
        ),
    );
    // No grab on entering Walk: OnEnter also fires for the initial state, and
    // grabbing the cursor at app start steals the user's mouse (any motion
    // then rotates the camera before they ever clicked into the window).
    app.add_systems(OnEnter(ControlMode::Build), (release_cursor, stop_player));
}

#[derive(Component)]
pub struct Player {
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
}

#[derive(Component)]
pub struct PlayerCamera;

pub const EYE_HEIGHT: f32 = 1.6;
/// Capsule: radius 0.3, segment length 1.2 -> total height 1.8, center at 0.9.
const CAPSULE_RADIUS: f32 = 0.3;
const CAPSULE_LENGTH: f32 = 1.2;
const PLAYER_CENTER_Y: f32 = CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS;

fn spawn_player(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Player"),
            Player {
                yaw: 0.0,
                pitch: 0.0,
                speed: 5.0,
            },
            Transform::from_xyz(0.0, PLAYER_CENTER_Y, 8.0),
            Visibility::default(),
            // Dynamic body with locked rotation: physics resolves collisions
            // against furniture, mouse-look drives yaw via Transform.
            RigidBody::Dynamic,
            Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.0),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("PlayerCamera"),
                PlayerCamera,
                Camera3d::default(),
                GizmoCamera,
                Transform::from_xyz(0.0, EYE_HEIGHT - PLAYER_CENTER_Y, 0.0),
            ));
        });
}

fn stop_player(mut player: Query<&mut LinearVelocity, With<Player>>) {
    if let Ok(mut velocity) = player.single_mut() {
        velocity.x = 0.0;
        velocity.z = 0.0;
    }
}

fn grab_cursor_on_click(
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        grab_cursor(cursor);
    }
}

fn release_cursor_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        release_cursor(cursor);
    }
}

fn grab_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    // Escape hatch for automated testing: grabbing the cursor locks the REAL
    // OS pointer to the window, which hijacks the mouse of whoever is using
    // the machine while agent-driven instances run in the background.
    if std::env::var_os("KAKKAI_NO_GRAB").is_some() {
        return;
    }
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    mut player: Query<(&mut Player, &mut Transform)>,
    mut camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    // Only look around while the cursor is captured.
    if !cursor
        .single()
        .is_ok_and(|c| c.grab_mode == CursorGrabMode::Locked)
    {
        return;
    }
    if motion.delta == Vec2::ZERO {
        return;
    }
    let Ok((mut player, mut player_transform)) = player.single_mut() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    const SENSITIVITY: f32 = 0.003;
    player.yaw -= motion.delta.x * SENSITIVITY;
    player.pitch = (player.pitch - motion.delta.y * SENSITIVITY).clamp(-1.54, 1.54);
    player_transform.rotation = Quat::from_rotation_y(player.yaw);
    camera_transform.rotation = Quat::from_rotation_x(player.pitch);
}

fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: Query<(&Player, &Transform, &mut LinearVelocity)>,
) {
    let mut input = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }
    let Ok((player, transform, mut velocity)) = player.single_mut() else {
        return;
    };
    let sprint = if keys.pressed(KeyCode::ShiftLeft) {
        2.0
    } else {
        1.0
    };
    // Drive horizontal velocity from input (zero when idle so the body never
    // slides on its own); vertical velocity stays with the physics engine.
    let direction = (transform.rotation * input).with_y(0.0).normalize_or_zero();
    let horizontal = direction * player.speed * sprint;
    velocity.x = horizontal.x;
    velocity.z = horizontal.z;
}
