mod avatar;

pub use avatar::{OwnAvatar, PlayerAvatar};

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
    ServerConfig,
};
use bevy_replicon_renet::renet::ConnectionConfig;
use bevy_replicon_renet::{RenetChannelsExt, RenetClient, RenetServer, RepliconRenetPlugins};

use crate::furniture::{
    Furniture, FurnitureId, FurnitureIndex, MoveFurniture, PlaceFurniture, RemoveFurniture,
    UndoStack,
};

/// "kakkai" as a protocol id — both sides must match.
const PROTOCOL_ID: u64 = 0x6B616B_6B6169;
pub const DEFAULT_PORT: u16 = 5151;

/// Network actions requested from the UI (or directly via BRP — every UI
/// button must stay invokable without pointer simulation).
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct NetRequest(pub Option<NetAction>);

#[derive(Reflect)]
pub enum NetAction {
    Host,
    Join(String),
    Leave,
}

/// UI state for the network panel.
#[derive(Resource)]
pub struct NetUi {
    pub address: String,
}

impl Default for NetUi {
    fn default() -> Self {
        Self {
            address: format!("127.0.0.1:{DEFAULT_PORT}"),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins((RepliconPlugins, RepliconRenetPlugins));
    app.init_resource::<NetRequest>();
    app.init_resource::<NetUi>();
    app.register_type::<NetRequest>();

    // The authoritative furniture state, replicated to every client.
    app.replicate::<FurnitureId>();
    app.replicate::<Furniture>();
    app.replicate::<Transform>();

    // The local message bus doubles as the client->server protocol: writing
    // these messages on a connected client delivers them to the server as
    // `FromClient<M>`; disconnected (singleplayer/host) they convert locally.
    app.add_client_message::<PlaceFurniture>(Channel::Ordered);
    app.add_client_message::<MoveFurniture>(Channel::Ordered);
    app.add_client_message::<RemoveFurniture>(Channel::Ordered);

    app.add_plugins(avatar::plugin);
    app.add_systems(Update, handle_requests);
}

fn handle_requests(world: &mut World) {
    let Some(action) = world.resource_mut::<NetRequest>().0.take() else {
        return;
    };
    let result = match action {
        NetAction::Host => host(world),
        NetAction::Join(address) => join(world, &address),
        NetAction::Leave => {
            leave(world);
            Ok(())
        }
    };
    if let Err(e) = result {
        error!("network action failed: {e}");
    }
}

fn host(world: &mut World) -> Result {
    let channels = world.resource::<RepliconChannels>();
    let server = RenetServer::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    });
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT))?;
    let transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time,
            max_clients: 8,
            protocol_id: PROTOCOL_ID,
            authentication: ServerAuthentication::Unsecure,
            public_addresses: Default::default(),
        },
        socket,
    )?;
    world.insert_resource(server);
    world.insert_resource(transport);

    // The host plays too: it gets an avatar like any client.
    let avatar = world
        .spawn((
            Name::new("PlayerAvatar"),
            PlayerAvatar,
            Replicated,
            Transform::from_translation(avatar::AVATAR_SPAWN),
        ))
        .id();
    world
        .resource_mut::<avatar::AvatarMap>()
        .0
        .insert(ClientId::Server, avatar);
    world.resource_mut::<OwnAvatar>().0 = Some(avatar);

    info!("hosting on port {DEFAULT_PORT}");
    Ok(())
}

fn join(world: &mut World, address: &str) -> Result {
    let server_addr: SocketAddr = address.parse()?;

    // The replicated world is about to arrive — drop the local solo world
    // (it stays safe in world.ron) and start from a clean slate.
    let local: Vec<Entity> = world
        .query_filtered::<Entity, With<Furniture>>()
        .iter(world)
        .collect();
    for entity in local {
        world.despawn(entity);
    }
    world.resource_mut::<FurnitureIndex>().0.clear();
    *world.resource_mut::<UndoStack>() = UndoStack::default();

    let channels = world.resource::<RepliconChannels>();
    let client = RenetClient::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    });
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let transport = NetcodeClientTransport::new(
        current_time,
        ClientAuthentication::Unsecure {
            client_id,
            protocol_id: PROTOCOL_ID,
            server_addr,
            user_data: None,
        },
        socket,
    )?;
    world.insert_resource(client);
    world.insert_resource(transport);

    info!("connecting to {server_addr}");
    Ok(())
}

fn leave(world: &mut World) {
    world.remove_resource::<RenetClient>();
    world.remove_resource::<NetcodeClientTransport>();
    world.remove_resource::<RenetServer>();
    world.remove_resource::<NetcodeServerTransport>();

    // Remove every avatar; furniture stays as a local copy of the world.
    let avatars: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerAvatar>>()
        .iter(world)
        .collect();
    for entity in avatars {
        world.despawn(entity);
    }
    world.resource_mut::<avatar::AvatarMap>().0.clear();
    world.resource_mut::<OwnAvatar>().0 = None;
    info!("left the session");
}
