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
    app.add_systems(OnEnter(ControlMode::Build), release_cursor);
    app.add_systems(OnEnter(ControlMode::Walk), grab_cursor);
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

fn spawn_player(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Player"),
            Player {
                yaw: 0.0,
                pitch: 0.0,
                speed: 5.0,
            },
            Transform::from_xyz(0.0, 0.0, 8.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("PlayerCamera"),
                PlayerCamera,
                Camera3d::default(),
                GizmoCamera,
                Transform::from_xyz(0.0, EYE_HEIGHT, 0.0),
            ));
        });
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
    time: Res<Time>,
    mut player: Query<(&Player, &mut Transform)>,
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
    if input == Vec3::ZERO {
        return;
    }
    let Ok((player, mut transform)) = player.single_mut() else {
        return;
    };
    let sprint = if keys.pressed(KeyCode::ShiftLeft) {
        2.0
    } else {
        1.0
    };
    // Move on the ground plane relative to where the player is facing.
    let direction = (transform.rotation * input).with_y(0.0).normalize_or_zero();
    transform.translation += direction * player.speed * sprint * time.delta_secs();
    // Kinematic v1: stay on the floor, no physics.
    transform.translation.y = 0.0;
}
