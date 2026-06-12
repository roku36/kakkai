use bevy::ecs::entity::MapEntities;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::player::Player;

/// A remote player's presence in the world. Replicated; visuals are derived
/// locally (same state/visual split as furniture).
#[derive(Component, Serialize, Deserialize)]
#[require(Transform, Visibility)]
pub struct PlayerAvatar;

/// Marker for the locally spawned avatar mesh children.
#[derive(Component)]
pub struct AvatarVisual;

/// Client -> server: my player moved (sent at a fixed rate, unreliable).
#[derive(Message, Serialize, Deserialize, Clone)]
pub struct UpdateAvatar {
    pub transform: Transform,
}

/// Server -> client: which replicated avatar entity is yours (so the client
/// can skip rendering it inside its own first-person camera).
#[derive(Message, Serialize, Deserialize, Clone, MapEntities)]
pub struct YourAvatar {
    #[entities]
    pub entity: Entity,
}

/// Server-side: avatar entity per connected client (including the host).
#[derive(Resource, Default)]
pub struct AvatarMap(pub HashMap<ClientId, Entity>);

/// The avatar entity that represents the local player, if any.
#[derive(Resource, Default)]
pub struct OwnAvatar(pub Option<Entity>);

pub const AVATAR_SPAWN: Vec3 = Vec3::new(0.0, 0.9, 8.0);

const UPDATE_INTERVAL: f32 = 0.05;

pub fn plugin(app: &mut App) {
    app.init_resource::<AvatarMap>();
    app.init_resource::<OwnAvatar>();
    app.replicate::<PlayerAvatar>();
    app.add_client_message::<UpdateAvatar>(Channel::Unreliable);
    app.add_mapped_server_message::<YourAvatar>(Channel::Ordered);

    app.add_observer(on_client_connected);
    app.add_observer(on_client_disconnected);
    app.add_systems(
        Update,
        (
            send_avatar_updates,
            apply_avatar_updates,
            receive_own_avatar,
            hydrate_avatars,
        ),
    );
}

/// Server: a client finished the replication handshake — give it an avatar
/// and tell it which one. (On `ConnectedClient` the client is not yet
/// authorized, so server messages to it would be dropped.)
fn on_client_connected(
    add: On<Add, AuthorizedClient>,
    mut map: ResMut<AvatarMap>,
    mut commands: Commands,
    mut your_avatar: MessageWriter<ToClients<YourAvatar>>,
) {
    let client_id = ClientId::Client(add.entity);
    let avatar = commands
        .spawn((
            Name::new("PlayerAvatar"),
            PlayerAvatar,
            Replicated,
            Transform::from_translation(AVATAR_SPAWN),
        ))
        .id();
    map.0.insert(client_id, avatar);
    your_avatar.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: YourAvatar { entity: avatar },
    });
    info!("client {client_id:?} connected, avatar {avatar:?}");
}

fn on_client_disconnected(
    remove: On<Remove, ConnectedClient>,
    mut map: ResMut<AvatarMap>,
    mut commands: Commands,
) {
    if let Some(avatar) = map.0.remove(&ClientId::Client(remove.entity)) {
        commands.entity(avatar).try_despawn();
        info!("client disconnected, avatar {avatar:?} removed");
    }
}

/// Everyone (host and clients): broadcast own position at a fixed rate.
/// In pure singleplayer there is no avatar in the map, so the locally drained
/// message is simply ignored.
fn send_avatar_updates(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    server: Res<State<ServerState>>,
    client: Res<State<ClientState>>,
    player: Query<&Transform, With<Player>>,
    mut updates: MessageWriter<UpdateAvatar>,
) {
    if *server.get() == ServerState::Stopped && *client.get() == ClientState::Disconnected {
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < UPDATE_INTERVAL {
        return;
    }
    *elapsed = 0.0;
    if let Ok(transform) = player.single() {
        updates.write(UpdateAvatar {
            transform: *transform,
        });
    }
}

/// Server: apply position updates to the sender's avatar.
fn apply_avatar_updates(
    mut updates: MessageReader<FromClient<UpdateAvatar>>,
    map: Res<AvatarMap>,
    mut transforms: Query<&mut Transform, With<PlayerAvatar>>,
) {
    for update in updates.read() {
        let Some(&avatar) = map.0.get(&update.client_id) else {
            continue;
        };
        if let Ok(mut transform) = transforms.get_mut(avatar) {
            *transform = update.transform;
        }
    }
}

/// Client: remember which avatar is ours and strip any visuals it already got.
fn receive_own_avatar(
    mut messages: MessageReader<YourAvatar>,
    mut own: ResMut<OwnAvatar>,
    children: Query<&Children>,
    visuals: Query<(), With<AvatarVisual>>,
    mut commands: Commands,
) {
    for msg in messages.read() {
        own.0 = Some(msg.entity);
        info!("own avatar is {:?}", msg.entity);
        if let Ok(existing) = children.get(msg.entity) {
            for child in existing {
                if visuals.contains(*child) {
                    commands.entity(*child).try_despawn();
                }
            }
        }
    }
}

/// Spawn a simple capsule body for every avatar that isn't our own.
fn hydrate_avatars(
    avatars: Query<Entity, Added<PlayerAvatar>>,
    own: Res<OwnAvatar>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for entity in &avatars {
        if own.0 == Some(entity) {
            continue;
        }
        let body = materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.6, 0.9),
            ..default()
        });
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new("AvatarBody"),
                AvatarVisual,
                Mesh3d(meshes.add(Capsule3d::new(0.3, 1.2))),
                MeshMaterial3d(body.clone()),
            ));
            parent.spawn((
                Name::new("AvatarNose"),
                AvatarVisual,
                Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 0.2))),
                MeshMaterial3d(body),
                // Forward marker at eye height so you can tell which way
                // another player is facing.
                Transform::from_xyz(0.0, 0.55, -0.3),
            ));
        });
    }
}
