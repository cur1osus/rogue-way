use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::{
    Enemy, GoldPickup, Health, MovementSpeed, Player, PlayerId, PlayerInputState, Projectile, XpGem,
};
use crate::components::{Experience, Gold};
use crate::components::{LocalPlayer, RemotePlayer};
use crate::components::{Pet, PetOwner, Velocity};
use crate::components::{PhysicsPosition, PreviousPhysicsPosition};
use crate::constants::{ENEMY_HITBOX_SCALE, GOLD_SCALE, XP_GEM_SCALE};
use crate::resources::{EnemySpriteSheet, GoldSprites, PetSpriteSheet, XpGemSprites};
use crate::systems::player::spawn_pet;
use crate::systems::player::spawn_player_entity;

pub const DEFAULT_PORT: u16 = 14000;
pub const LOCAL_PLAYER_ID: u32 = 1;

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Offline,
    Host,
    Client,
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Offline
    }
}

#[derive(Resource)]
pub struct NetworkServer {
    listener: TcpListener,
    clients: Vec<ClientConnection>,
    pub join_code: String,
    pub port: u16,
    next_player_id: u32,
    snapshot_timer: Timer,
}

#[derive(Resource)]
pub struct NetworkClient {
    stream: TcpStream,
    buffer: Vec<u8>,
    pub server_addr: SocketAddr,
    pub player_id: Option<u32>,
    input_timer: Timer,
}

#[derive(Resource, Default)]
pub struct NetworkEntityMap {
    pub entities: HashMap<u32, Entity>,
}

#[derive(Resource, Default)]
pub struct NetworkIdAllocator {
    pub next_id: u32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkId(pub u32);

#[derive(Debug)]
struct ClientConnection {
    id: u32,
    stream: TcpStream,
    buffer: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct NetInput {
    pub dir: [f32; 2],
    pub pushback: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSnapshot {
    pub id: u32,
    pub player_id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub hp: f32,
    pub hp_max: f32,
    pub move_speed: f32,
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub gold: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PetSnapshot {
    pub id: u32,
    pub owner_id: u32,
    pub pet_type: u8,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemySnapshot {
    pub id: u32,
    pub enemy_type: u8,
    pub boss_type: Option<u8>,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub hp: f32,
    pub hp_max: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectileSnapshot {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XpSnapshot {
    pub id: u32,
    pub pos: [f32; 2],
    pub value: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldSnapshot {
    pub id: u32,
    pub pos: [f32; 2],
    pub value: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetMessage {
    Hello {
        protocol: u32,
    },
    Welcome {
        player_id: u32,
    },
    Input {
        player_id: u32,
        input: NetInput,
    },
    Snapshot {
        tick: u64,
        players: Vec<PlayerSnapshot>,
        pets: Vec<PetSnapshot>,
        enemies: Vec<EnemySnapshot>,
        projectiles: Vec<ProjectileSnapshot>,
        xp_gems: Vec<XpSnapshot>,
        gold: Vec<GoldSnapshot>,
    },
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkMode>()
            .init_resource::<NetworkIdAllocator>()
            .add_systems(Update, assign_network_ids_system)
            .add_systems(Update, host_accept_system)
            .add_systems(Update, host_receive_system)
            .add_systems(Update, host_snapshot_system)
            .add_systems(Update, client_receive_system)
            .add_systems(Update, client_send_input_system);
    }
}

pub fn is_host(mode: Option<Res<NetworkMode>>) -> bool {
    matches!(mode.map(|m| *m), Some(NetworkMode::Host))
}

pub fn is_client(mode: Option<Res<NetworkMode>>) -> bool {
    matches!(mode.map(|m| *m), Some(NetworkMode::Client))
}

pub fn is_authoritative(mode: Option<Res<NetworkMode>>) -> bool {
    match mode.map(|m| *m) {
        Some(NetworkMode::Client) => false,
        _ => true,
    }
}

pub fn start_host(commands: &mut Commands, port: u16) -> Result<String, String> {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .map_err(|err| format!("Failed to bind server: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("Failed to set non-blocking: {err}"))?;

    let join_code = build_join_code(port);
    commands.insert_resource(NetworkMode::Host);
    commands.insert_resource(NetworkServer {
        listener,
        clients: Vec::new(),
        join_code: join_code.clone(),
        port,
        next_player_id: LOCAL_PLAYER_ID + 1,
        snapshot_timer: Timer::from_seconds(0.066, TimerMode::Repeating),
    });
    commands.insert_resource(NetworkIdAllocator { next_id: 1 });

    Ok(join_code)
}

pub fn start_client(commands: &mut Commands, addr: SocketAddr) -> Result<(), String> {
    let stream = TcpStream::connect(addr).map_err(|err| format!("Connect failed: {err}"))?;
    stream
        .set_nonblocking(true)
        .map_err(|err| format!("Failed to set non-blocking: {err}"))?;

    let mut client = NetworkClient {
        stream,
        buffer: Vec::new(),
        server_addr: addr,
        player_id: None,
        input_timer: Timer::from_seconds(0.033, TimerMode::Repeating),
    };
    let _ = send_message(&mut client.stream, &NetMessage::Hello { protocol: 1 });

    commands.insert_resource(NetworkMode::Client);
    commands.insert_resource(client);
    commands.insert_resource(NetworkEntityMap::default());

    Ok(())
}

fn build_join_code(port: u16) -> String {
    let ip = detect_local_ip().unwrap_or(IpAddr::from([127, 0, 0, 1]));
    format!("{ip}:{port}")
}

fn detect_local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}

fn send_message(stream: &mut TcpStream, message: &NetMessage) -> std::io::Result<()> {
    let payload = bincode::serialize(message)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(&payload)?;
    Ok(())
}

fn read_messages(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> std::io::Result<Vec<NetMessage>> {
    let mut temp = [0u8; 4096];
    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&temp[..n]),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(err) => return Err(err),
        }
    }

    let mut messages = Vec::new();
    loop {
        if buffer.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        if buffer.len() < 4 + len {
            break;
        }
        let payload = buffer[4..4 + len].to_vec();
        buffer.drain(0..4 + len);
        let msg: NetMessage = bincode::deserialize(&payload)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        messages.push(msg);
    }
    Ok(messages)
}

fn assign_network_ids_system(
    mut commands: Commands,
    mut allocator: ResMut<NetworkIdAllocator>,
    players: Query<Entity, (With<Player>, Without<NetworkId>)>,
    pets: Query<Entity, (With<Pet>, Without<NetworkId>)>,
    enemies: Query<Entity, (With<Enemy>, Without<NetworkId>)>,
    projectiles: Query<Entity, (With<Projectile>, Without<NetworkId>)>,
    xp_gems: Query<Entity, (With<XpGem>, Without<NetworkId>)>,
    gold: Query<Entity, (With<GoldPickup>, Without<NetworkId>)>,
    mode: Option<Res<NetworkMode>>,
) {
    if !matches!(mode.map(|m| *m), Some(NetworkMode::Host)) {
        return;
    }

    let assign = |entity: Entity, allocator: &mut NetworkIdAllocator, commands: &mut Commands| {
        let id = allocator.next_id;
        allocator.next_id = allocator.next_id.saturating_add(1);
        commands.entity(entity).insert(NetworkId(id));
    };

    for entity in players.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
    for entity in pets.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
    for entity in enemies.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
    for entity in projectiles.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
    for entity in xp_gems.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
    for entity in gold.iter() {
        assign(entity, &mut allocator, &mut commands);
    }
}

fn host_accept_system(
    mut commands: Commands,
    mut server: Option<ResMut<NetworkServer>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    meta: Res<crate::resources::MetaProgression>,
    upgrade_state: Res<crate::resources::UpgradeState>,
    pet_sprites: Res<PetSpriteSheet>,
) {
    let Some(mut server) = server else {
        return;
    };

    loop {
        match server.listener.accept() {
            Ok((mut stream, addr)) => {
                let _ = stream.set_nonblocking(true);
                let player_id = server.next_player_id;
                server.next_player_id += 1;

                let _ = send_message(&mut stream, &NetMessage::Welcome { player_id });

                server.clients.push(ClientConnection {
                    id: player_id,
                    stream,
                    buffer: Vec::new(),
                });

                let player_entity = spawn_player_entity(
                    &mut commands,
                    &asset_server,
                    &mut texture_atlas_layouts,
                    &meta,
                    PlayerId(player_id),
                    false,
                );
                commands.entity(player_entity).insert(RemotePlayer);

                spawn_pet(
                    &mut commands,
                    crate::components::PetType::GuardDog,
                    Vec2::ZERO,
                    &upgrade_state,
                    &pet_sprites,
                    player_id,
                );

                println!("Client {player_id} connected from {addr}");
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(err) => {
                eprintln!("Accept failed: {err}");
                break;
            }
        }
    }
}

fn host_receive_system(
    mut commands: Commands,
    mut server: Option<ResMut<NetworkServer>>,
    mut input_query: Query<(&PlayerId, &mut PlayerInputState), With<Player>>,
    player_entities: Query<(Entity, &PlayerId), With<Player>>,
    pet_entities: Query<(Entity, &PetOwner), With<Pet>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    meta: Res<crate::resources::MetaProgression>,
    upgrade_state: Res<crate::resources::UpgradeState>,
    pet_sprites: Res<PetSpriteSheet>,
) {
    let Some(mut server) = server else {
        return;
    };
    let mut disconnected = Vec::new();
    for (index, client) in server.clients.iter_mut().enumerate() {
        let messages = match read_messages(&mut client.stream, &mut client.buffer) {
            Ok(messages) => messages,
            Err(err) => {
                eprintln!("Client {} read error: {err}", client.id);
                disconnected.push(index);
                continue;
            }
        };

        for message in messages {
            match message {
                NetMessage::Input { player_id, input } => {
                    let mut found = false;
                    for (id, mut state) in input_query.iter_mut() {
                        if id.0 == player_id {
                            state.movement = Vec2::new(input.dir[0], input.dir[1]);
                            if input.pushback {
                                state.pushback = true;
                            }
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        let player_entity = spawn_player_entity(
                            &mut commands,
                            &asset_server,
                            &mut texture_atlas_layouts,
                            &meta,
                            PlayerId(player_id),
                            false,
                        );
                        commands.entity(player_entity).insert(RemotePlayer);
                        spawn_pet(
                            &mut commands,
                            crate::components::PetType::GuardDog,
                            Vec2::ZERO,
                            &upgrade_state,
                            &pet_sprites,
                            player_id,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    for index in disconnected.into_iter().rev() {
        let client = server.clients.swap_remove(index);
        eprintln!("Client {} disconnected", client.id);
        for (entity, player_id) in player_entities.iter() {
            if player_id.0 == client.id {
                commands.entity(entity).despawn();
            }
        }
        for (entity, owner) in pet_entities.iter() {
            if owner.0 == client.id {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn host_snapshot_system(
    time: Res<Time>,
    mut server: Option<ResMut<NetworkServer>>,
    players: Query<(
        &NetworkId,
        &PlayerId,
        &PhysicsPosition,
        &Velocity,
        &Health,
        &MovementSpeed,
        &Experience,
        &Gold,
    )>,
    pets: Query<(&NetworkId, &Pet, &PetOwner, &PhysicsPosition, &Velocity)>,
    enemies: Query<(
        &NetworkId,
        &Enemy,
        Option<&crate::components::Boss>,
        &PhysicsPosition,
        &Velocity,
        &Health,
    )>,
    projectiles: Query<(&NetworkId, &Projectile, &Transform)>,
    xp_gems: Query<(&NetworkId, &XpGem, &Transform)>,
    gold: Query<(&NetworkId, &GoldPickup, &Transform)>,
) {
    let Some(mut server) = server else {
        return;
    };
    if server.clients.is_empty() {
        return;
    }

    server.snapshot_timer.tick(time.delta());
    if !server.snapshot_timer.just_finished() {
        return;
    }

    let mut player_snapshots = Vec::new();
    for (net_id, player_id, pos, vel, health, speed, xp, gold) in players.iter() {
        player_snapshots.push(PlayerSnapshot {
            id: net_id.0,
            player_id: player_id.0,
            pos: [pos.0.x, pos.0.y],
            vel: [vel.0.x, vel.0.y],
            hp: health.current,
            hp_max: health.max,
            move_speed: speed.0,
            level: xp.level,
            xp: xp.current,
            xp_next: xp.to_next_level,
            gold: gold.amount,
        });
    }

    let mut pet_snapshots = Vec::new();
    for (net_id, pet, owner, pos, vel) in pets.iter() {
        pet_snapshots.push(PetSnapshot {
            id: net_id.0,
            owner_id: owner.0,
            pet_type: pet_type_to_u8(pet.pet_type),
            pos: [pos.0.x, pos.0.y],
            vel: [vel.0.x, vel.0.y],
        });
    }

    let mut enemy_snapshots = Vec::new();
    for (net_id, enemy, boss_opt, pos, vel, health) in enemies.iter() {
        enemy_snapshots.push(EnemySnapshot {
            id: net_id.0,
            enemy_type: enemy_type_to_u8(enemy.enemy_type),
            boss_type: boss_opt.map(|boss| boss_type_to_u8(boss.boss_type)),
            pos: [pos.0.x, pos.0.y],
            vel: [vel.0.x, vel.0.y],
            hp: health.current,
            hp_max: health.max,
        });
    }

    let mut projectile_snapshots = Vec::new();
    for (net_id, projectile, transform) in projectiles.iter() {
        projectile_snapshots.push(ProjectileSnapshot {
            id: net_id.0,
            pos: [transform.translation.x, transform.translation.y],
            vel: [projectile.velocity.x, projectile.velocity.y],
        });
    }

    let mut xp_snapshots = Vec::new();
    for (net_id, gem, transform) in xp_gems.iter() {
        xp_snapshots.push(XpSnapshot {
            id: net_id.0,
            pos: [transform.translation.x, transform.translation.y],
            value: gem.value,
        });
    }

    let mut gold_snapshots = Vec::new();
    for (net_id, coin, transform) in gold.iter() {
        gold_snapshots.push(GoldSnapshot {
            id: net_id.0,
            pos: [transform.translation.x, transform.translation.y],
            value: coin.value,
        });
    }

    let snapshot = NetMessage::Snapshot {
        tick: time.elapsed_secs() as u64,
        players: player_snapshots,
        pets: pet_snapshots,
        enemies: enemy_snapshots,
        projectiles: projectile_snapshots,
        xp_gems: xp_snapshots,
        gold: gold_snapshots,
    };

    for client in server.clients.iter_mut() {
        if let Err(err) = send_message(&mut client.stream, &snapshot) {
            eprintln!("Snapshot send failed for {}: {err}", client.id);
        }
    }
}

fn client_receive_system(
    mut commands: Commands,
    mut client: Option<ResMut<NetworkClient>>,
    mut entity_map: Option<ResMut<NetworkEntityMap>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    meta: Res<crate::resources::MetaProgression>,
    mut accumulator: ResMut<crate::resources::PhysicsAccumulator>,
    enemy_sprites: Res<EnemySpriteSheet>,
    pet_sprites: Res<PetSpriteSheet>,
    xp_sprites: Res<XpGemSprites>,
    gold_sprites: Res<GoldSprites>,
    mut query_set: ParamSet<(
        Query<
            (
                Entity,
                &NetworkId,
                &PlayerId,
                &mut PhysicsPosition,
                &mut PreviousPhysicsPosition,
                &mut Velocity,
                &mut Health,
                &mut MovementSpeed,
                &mut Experience,
                &mut Gold,
            ),
            With<Player>,
        >,
        Query<(Entity, &PlayerId), With<Player>>,
        Query<
            (
                Entity,
                &NetworkId,
                &mut PhysicsPosition,
                &mut PreviousPhysicsPosition,
                &mut Velocity,
            ),
            With<Pet>,
        >,
        Query<
            (
                Entity,
                &NetworkId,
                &mut PhysicsPosition,
                &mut PreviousPhysicsPosition,
                &mut Velocity,
                &mut Health,
            ),
            With<Enemy>,
        >,
        Query<(Entity, &NetworkId, &mut Transform), With<Projectile>>,
        Query<(Entity, &NetworkId, &mut Transform), With<XpGem>>,
        Query<(Entity, &NetworkId, &mut Transform), With<GoldPickup>>,
    )>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(mut entity_map) = entity_map else {
        return;
    };

    let messages = {
        let mut buffer = std::mem::take(&mut client.buffer);
        let result = match read_messages(&mut client.stream, &mut buffer) {
            Ok(messages) => Ok(messages),
            Err(err) => Err(err),
        };
        client.buffer = buffer;

        match result {
            Ok(messages) => messages,
            Err(err) => {
                eprintln!("Client receive error: {err}");
                return;
            }
        }
    };

    for message in messages {
        match message {
            NetMessage::Welcome { player_id } => {
                client.player_id = Some(player_id);
                {
                    let player_id_query = query_set.p1();
                    for (entity, id) in player_id_query.iter() {
                        if id.0 == player_id {
                            commands
                                .entity(entity)
                                .insert(LocalPlayer)
                                .remove::<RemotePlayer>();
                        }
                    }
                }
                println!("Connected. Player id: {player_id}");
            }
            NetMessage::Snapshot {
                players,
                pets,
                enemies,
                projectiles,
                xp_gems,
                gold,
                ..
            } => {
                accumulator.accumulator = crate::resources::FIXED_TIMESTEP;
                let mut present_ids: HashSet<u32> = HashSet::new();

                {
                    let mut players_query = query_set.p0();
                    for snapshot in players {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((
                                _entity,
                                _net_id,
                                _player_id,
                                mut pos,
                                mut prev,
                                mut vel,
                                mut health,
                                mut speed,
                                mut xp,
                                mut gold,
                            )) = players_query.get_mut(entity)
                            {
                                prev.0 = pos.0;
                                pos.0 = Vec2::new(snapshot.pos[0], snapshot.pos[1]);
                                vel.0 = Vec2::new(snapshot.vel[0], snapshot.vel[1]);
                                health.current = snapshot.hp;
                                health.max = snapshot.hp_max;
                                speed.0 = snapshot.move_speed;
                                xp.level = snapshot.level;
                                xp.current = snapshot.xp;
                                xp.to_next_level = snapshot.xp_next;
                                gold.amount = snapshot.gold;
                            }
                        } else {
                            let player_entity = spawn_player_entity(
                                &mut commands,
                                &asset_server,
                                &mut texture_atlas_layouts,
                                &meta,
                                PlayerId(snapshot.player_id),
                                false,
                            );
                            commands
                                .entity(player_entity)
                                .insert(NetworkId(snapshot.id))
                                .insert(Health {
                                    current: snapshot.hp,
                                    max: snapshot.hp_max,
                                })
                                .insert(MovementSpeed(snapshot.move_speed))
                                .insert(Experience {
                                    current: snapshot.xp,
                                    to_next_level: snapshot.xp_next,
                                    level: snapshot.level,
                                })
                                .insert(Gold {
                                    amount: snapshot.gold,
                                })
                                .insert(PhysicsPosition(Vec2::new(
                                    snapshot.pos[0],
                                    snapshot.pos[1],
                                )))
                                .insert(PreviousPhysicsPosition(Vec2::new(
                                    snapshot.pos[0],
                                    snapshot.pos[1],
                                )))
                                .insert(Velocity(Vec2::new(snapshot.vel[0], snapshot.vel[1])));

                            if client.player_id == Some(snapshot.player_id) {
                                commands.entity(player_entity).insert(LocalPlayer);
                            } else {
                                commands.entity(player_entity).insert(RemotePlayer);
                            }

                            entity_map.entities.insert(snapshot.id, player_entity);
                        }
                    }
                }

                {
                    let mut pets_query = query_set.p2();
                    for snapshot in pets {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((_entity, _net_id, mut pos, mut prev, mut vel)) =
                                pets_query.get_mut(entity)
                            {
                                prev.0 = pos.0;
                                pos.0 = Vec2::new(snapshot.pos[0], snapshot.pos[1]);
                                vel.0 = Vec2::new(snapshot.vel[0], snapshot.vel[1]);
                            }
                        } else if let Some(pet_type) = pet_type_from_u8(snapshot.pet_type) {
                            let entity = spawn_remote_pet(
                                &mut commands,
                                &pet_sprites,
                                pet_type,
                                Vec2::new(snapshot.pos[0], snapshot.pos[1]),
                                snapshot.owner_id,
                            );
                            commands
                                .entity(entity)
                                .insert(NetworkId(snapshot.id))
                                .insert(Velocity(Vec2::new(snapshot.vel[0], snapshot.vel[1])));
                            entity_map.entities.insert(snapshot.id, entity);
                        }
                    }
                }

                {
                    let mut enemies_query = query_set.p3();
                    for snapshot in enemies {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((_entity, _net_id, mut pos, mut prev, mut vel, mut health)) =
                                enemies_query.get_mut(entity)
                            {
                                prev.0 = pos.0;
                                pos.0 = Vec2::new(snapshot.pos[0], snapshot.pos[1]);
                                vel.0 = Vec2::new(snapshot.vel[0], snapshot.vel[1]);
                                health.current = snapshot.hp;
                                health.max = snapshot.hp_max;
                            }
                        } else if let Some(enemy_type) = enemy_type_from_u8(snapshot.enemy_type) {
                            let boss_type = snapshot.boss_type.and_then(boss_type_from_u8);
                            let entity = spawn_remote_enemy(
                                &mut commands,
                                &enemy_sprites,
                                enemy_type,
                                boss_type,
                                Vec2::new(snapshot.pos[0], snapshot.pos[1]),
                                snapshot.hp,
                                snapshot.hp_max,
                            );
                            commands
                                .entity(entity)
                                .insert(NetworkId(snapshot.id))
                                .insert(Velocity(Vec2::new(snapshot.vel[0], snapshot.vel[1])));
                            entity_map.entities.insert(snapshot.id, entity);
                        }
                    }
                }

                {
                    let mut projectile_query = query_set.p4();
                    for snapshot in projectiles {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((_entity, _net_id, mut transform)) =
                                projectile_query.get_mut(entity)
                            {
                                transform.translation.x = snapshot.pos[0];
                                transform.translation.y = snapshot.pos[1];
                            }
                        } else {
                            let entity = spawn_remote_projectile(
                                &mut commands,
                                Vec2::new(snapshot.pos[0], snapshot.pos[1]),
                            );
                            commands.entity(entity).insert(NetworkId(snapshot.id));
                            entity_map.entities.insert(snapshot.id, entity);
                        }
                    }
                }

                {
                    let mut xp_query = query_set.p5();
                    for snapshot in xp_gems {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((_entity, _net_id, mut transform)) = xp_query.get_mut(entity)
                            {
                                transform.translation.x = snapshot.pos[0];
                                transform.translation.y = snapshot.pos[1];
                            }
                        } else {
                            let entity = spawn_remote_xp(
                                &mut commands,
                                &xp_sprites,
                                Vec2::new(snapshot.pos[0], snapshot.pos[1]),
                                snapshot.value,
                            );
                            commands.entity(entity).insert(NetworkId(snapshot.id));
                            entity_map.entities.insert(snapshot.id, entity);
                        }
                    }
                }

                {
                    let mut gold_query = query_set.p6();
                    for snapshot in gold {
                        present_ids.insert(snapshot.id);
                        if let Some(&entity) = entity_map.entities.get(&snapshot.id) {
                            if let Ok((_entity, _net_id, mut transform)) =
                                gold_query.get_mut(entity)
                            {
                                transform.translation.x = snapshot.pos[0];
                                transform.translation.y = snapshot.pos[1];
                            }
                        } else {
                            let entity = spawn_remote_gold(
                                &mut commands,
                                &gold_sprites,
                                Vec2::new(snapshot.pos[0], snapshot.pos[1]),
                                snapshot.value,
                            );
                            commands.entity(entity).insert(NetworkId(snapshot.id));
                            entity_map.entities.insert(snapshot.id, entity);
                        }
                    }
                }

                let mut to_remove = Vec::new();
                for (id, entity) in entity_map.entities.iter() {
                    if !present_ids.contains(id) {
                        commands.entity(*entity).despawn();
                        to_remove.push(*id);
                    }
                }
                for id in to_remove {
                    entity_map.entities.remove(&id);
                }
            }
            _ => {}
        }
    }
}

fn client_send_input_system(
    time: Res<Time>,
    mut client: Option<ResMut<NetworkClient>>,
    mut query: Query<&mut PlayerInputState, With<LocalPlayer>>,
) {
    let Some(mut client) = client else {
        return;
    };
    let Some(player_id) = client.player_id else {
        return;
    };

    client.input_timer.tick(time.delta());
    if !client.input_timer.just_finished() {
        return;
    }

    if let Ok(mut input_state) = query.single_mut() {
        let input = NetInput {
            dir: [input_state.movement.x, input_state.movement.y],
            pushback: input_state.pushback,
        };
        let _ = send_message(&mut client.stream, &NetMessage::Input { player_id, input });
        input_state.pushback = false;
    }
}

fn pet_type_to_u8(pet_type: crate::components::PetType) -> u8 {
    match pet_type {
        crate::components::PetType::GuardDog => 1,
        crate::components::PetType::FireSprite => 2,
        crate::components::PetType::SlimeCompanion => 3,
        crate::components::PetType::CrowScout => 4,
        crate::components::PetType::XpCollector => 5,
    }
}

fn pet_type_from_u8(value: u8) -> Option<crate::components::PetType> {
    match value {
        1 => Some(crate::components::PetType::GuardDog),
        2 => Some(crate::components::PetType::FireSprite),
        3 => Some(crate::components::PetType::SlimeCompanion),
        4 => Some(crate::components::PetType::CrowScout),
        5 => Some(crate::components::PetType::XpCollector),
        _ => None,
    }
}

fn enemy_type_to_u8(enemy_type: crate::components::EnemyType) -> u8 {
    match enemy_type {
        crate::components::EnemyType::Bandit => 1,
        crate::components::EnemyType::Thief => 2,
        crate::components::EnemyType::Brute => 3,
    }
}

fn enemy_type_from_u8(value: u8) -> Option<crate::components::EnemyType> {
    match value {
        1 => Some(crate::components::EnemyType::Bandit),
        2 => Some(crate::components::EnemyType::Thief),
        3 => Some(crate::components::EnemyType::Brute),
        _ => None,
    }
}

fn boss_type_to_u8(boss_type: crate::components::BossType) -> u8 {
    match boss_type {
        crate::components::BossType::BanditLeader => 1,
        crate::components::BossType::ThiefKing => 2,
        crate::components::BossType::BruteChieftain => 3,
    }
}

fn boss_type_from_u8(value: u8) -> Option<crate::components::BossType> {
    match value {
        1 => Some(crate::components::BossType::BanditLeader),
        2 => Some(crate::components::BossType::ThiefKing),
        3 => Some(crate::components::BossType::BruteChieftain),
        _ => None,
    }
}

fn spawn_remote_enemy(
    commands: &mut Commands,
    enemy_sprites: &EnemySpriteSheet,
    enemy_type: crate::components::EnemyType,
    boss_type: Option<crate::components::BossType>,
    position: Vec2,
    current_health: f32,
    max_health: f32,
) -> Entity {
    let size = if let Some(boss_type) = boss_type {
        boss_type.get_size()
    } else {
        enemy_type.get_size()
    };
    let scale = size / enemy_sprites.idle.frame_size.y;
    let hitbox_size = Vec2::splat(size * ENEMY_HITBOX_SCALE);

    let mut entity_commands = commands.spawn((
        Sprite {
            image: enemy_sprites.idle.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: enemy_sprites.idle.layout.clone(),
                index: enemy_sprites.idle.first,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 1.0).with_scale(Vec3::splat(scale)),
        Enemy { enemy_type },
        Health {
            current: current_health,
            max: max_health,
        },
        Velocity::default(),
        PhysicsPosition(position),
        PreviousPhysicsPosition(position),
        crate::components::Hitbox::from_full_size(hitbox_size),
        crate::components::AnimationIndices {
            first: enemy_sprites.idle.first,
            last: enemy_sprites.idle.last,
        },
        crate::components::AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
    ));

    if let Some(boss_type) = boss_type {
        entity_commands.insert(crate::components::Boss { boss_type });
    }

    entity_commands.id()
}

fn spawn_remote_pet(
    commands: &mut Commands,
    pet_sprites: &PetSpriteSheet,
    pet_type: crate::components::PetType,
    position: Vec2,
    owner_id: u32,
) -> Entity {
    let sheet = pet_sprites.get_idle_sheet(&pet_type);
    let size = pet_type.get_size();
    let pet_scale = size / sheet.frame_size.y;
    let hitbox_size = Vec2::splat(size * crate::constants::PET_HITBOX_SCALE);

    commands
        .spawn((
            Sprite {
                image: sheet.texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: sheet.first,
                }),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 1.0).with_scale(Vec3::splat(pet_scale)),
            Pet { pet_type },
            PetOwner(owner_id),
            Velocity::default(),
            PhysicsPosition(position),
            PreviousPhysicsPosition(position),
            crate::components::Hitbox::from_full_size(hitbox_size),
            crate::components::AnimationIndices {
                first: sheet.first,
                last: sheet.last,
            },
            crate::components::AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
        ))
        .id()
}

fn spawn_remote_projectile(commands: &mut Commands, position: Vec2) -> Entity {
    commands
        .spawn((
            Projectile {
                velocity: Vec2::ZERO,
                damage: 0.0,
                lifetime: Timer::from_seconds(10.0, TimerMode::Once),
                piercing: false,
                pierced_count: 0,
                max_pierce: 0,
                area_radius: 0.0,
            },
            Sprite {
                color: Color::srgb(1.0, 0.9, 0.6),
                custom_size: Some(Vec2::splat(8.0)),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 0.8),
        ))
        .id()
}

fn spawn_remote_xp(
    commands: &mut Commands,
    sprites: &XpGemSprites,
    position: Vec2,
    value: u32,
) -> Entity {
    commands
        .spawn((
            XpGem { value },
            Sprite {
                image: sprites.texture.clone(),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 0.5).with_scale(Vec3::splat(XP_GEM_SCALE)),
        ))
        .id()
}

fn spawn_remote_gold(
    commands: &mut Commands,
    sprites: &GoldSprites,
    position: Vec2,
    value: u32,
) -> Entity {
    commands
        .spawn((
            GoldPickup { value },
            Sprite {
                image: sprites.normal_texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sprites.normal_layout.clone(),
                    index: sprites.normal_index,
                }),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 0.5).with_scale(Vec3::splat(GOLD_SCALE)),
            crate::components::GoldHighlightTimer {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
                is_highlighted: false,
            },
        ))
        .id()
}
