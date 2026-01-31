use bevy::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::components::{Health, LocalPlayer, PhysicsPosition, PlayerId, RemotePlayer, Velocity};
use crate::network::protocol::delta::WorldDelta;
use crate::network::protocol::messages::{C2S, InputCmd, S2C};
use crate::network::protocol::quantization::Dequantize;
use crate::network::protocol::snapshots::{PlayerStateNet, Snapshot};
use crate::network::transport::channels::ClientChannelsResource;
use crate::network::world::net_id::{NetworkEntityMap, NetworkId};
use crate::network::NetworkMode;

/// Размер буфера снапшотов для интерполяции
const SNAPSHOT_BUFFER_SIZE: usize = 10;

/// Целевая задержка для интерполяции (в снапшотах)
const TARGET_INTERPOLATION_DELAY: usize = 2;

/// Resource для отслеживания client tick
#[derive(Resource)]
pub struct ClientTick {
    /// Клиентский sequence number для input команд
    pub client_seq: u32,
    /// Клиентский tick (для предсказания)
    pub client_tick: u32,
    /// Последний полученный server tick
    pub last_server_tick: u32,
}

impl Default for ClientTick {
    fn default() -> Self {
        Self {
            client_seq: 0,
            client_tick: 0,
            last_server_tick: 0,
        }
    }
}

/// Буферизованный снапшот для интерполяции
#[derive(Debug, Clone)]
pub struct BufferedSnapshot {
    /// Server tick этого снапшота
    pub server_tick: u32,
    /// Данные снапшота
    pub snapshot: Snapshot,
}

/// Resource для буферизации снапшотов
#[derive(Resource)]
pub struct SnapshotBuffer {
    /// Буфер снапшотов (сортированный по server_tick)
    pub snapshots: VecDeque<BufferedSnapshot>,
    /// Целевая задержка для smooth интерполяции
    pub target_delay: usize,
}

impl Default for SnapshotBuffer {
    fn default() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(SNAPSHOT_BUFFER_SIZE),
            target_delay: TARGET_INTERPOLATION_DELAY,
        }
    }
}

impl SnapshotBuffer {
    /// Добавляет снапшот в буфер
    pub fn push(&mut self, server_tick: u32, snapshot: Snapshot) {
        // Вставляем в правильную позицию (сортировка по tick)
        let pos = self
            .snapshots
            .iter()
            .position(|s| s.server_tick > server_tick)
            .unwrap_or(self.snapshots.len());

        self.snapshots.insert(
            pos,
            BufferedSnapshot {
                server_tick,
                snapshot,
            },
        );

        // Ограничиваем размер буфера
        while self.snapshots.len() > SNAPSHOT_BUFFER_SIZE {
            self.snapshots.pop_front();
        }
    }

    /// Получает два снапшота для интерполяции
    pub fn get_interpolation_pair(&self) -> Option<(&BufferedSnapshot, &BufferedSnapshot)> {
        if self.snapshots.len() < 2 {
            return None;
        }

        // Берем два последних снапшота с учетом задержки
        let idx = self.snapshots.len().saturating_sub(self.target_delay + 1);
        if idx + 1 < self.snapshots.len() {
            Some((&self.snapshots[idx], &self.snapshots[idx + 1]))
        } else {
            None
        }
    }

    /// Получает последний снапшот
    pub fn get_latest(&self) -> Option<&BufferedSnapshot> {
        self.snapshots.back()
    }
}

/// Wrapper для incoming receiver (нужен Mutex для мутабельности из системы)
#[derive(Resource)]
pub struct ClientIncomingRx(pub Arc<Mutex<mpsc::UnboundedReceiver<S2C>>>);

/// Resource для хранения player_id этого клиента
#[derive(Resource)]
pub struct LocalPlayerId(pub u8);

/// Плагин для клиентской части networking
pub struct NetClientPlugin;

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientTick>()
            .init_resource::<SnapshotBuffer>()
            .init_resource::<NetworkEntityMap>()
            .add_systems(Startup, setup_client_channels)
            .add_systems(
                Update,
                (
                    client_read_incoming,
                    client_capture_input,
                    client_apply_snapshot,
                )
                    .run_if(is_client),
            );
    }
}

/// Condition для запуска систем только на клиенте
fn is_client(mode: Option<Res<NetworkMode>>) -> bool {
    matches!(mode.as_deref(), Some(NetworkMode::Client))
}

/// Startup система для извлечения receivers из channels
fn setup_client_channels(
    channels_res: Option<ResMut<ClientChannelsResource>>,
    mut commands: Commands,
) {
    let Some(mut channels_res) = channels_res else {
        return;
    };

    // Получаем мутабельный доступ к внутреннему Arc через Deref
    let channels = &mut *channels_res;

    // SAFETY: Мы используем Arc::get_mut только если есть единственная ссылка
    // В startup системе это безопасно, так как мы еще не запустили другие системы
    if let Some(channels_mut) = Arc::get_mut(&mut channels.0) {
        if let Some(incoming_rx) = channels_mut.incoming_rx.take() {
            commands.insert_resource(ClientIncomingRx(Arc::new(Mutex::new(incoming_rx))));
            println!("[Client] Client channels initialized");
        }
    } else {
        // Если не получилось, значит есть другие ссылки - используем fallback
        println!("[Client] Warning: Could not extract incoming_rx - multiple Arc references");
    }
}

/// Система для чтения входящих сообщений от сервера
fn client_read_incoming(
    incoming_rx: Option<Res<ClientIncomingRx>>,
    mut tick: ResMut<ClientTick>,
    mut snapshot_buffer: ResMut<SnapshotBuffer>,
    channels: Option<Res<ClientChannelsResource>>,
    mut commands: Commands,
) {
    let Some(incoming_rx) = incoming_rx else {
        return;
    };

    let Some(channels) = channels else {
        return;
    };

    // Обрабатываем все доступные сообщения (non-blocking)
    let mut rx = incoming_rx.0.lock().unwrap();

    while let Ok(msg) = rx.try_recv() {
        match msg {
            S2C::HelloOk { protocol, tick_hz } => {
                println!(
                    "[Client] HelloOk received (protocol: {}, tick_hz: {})",
                    protocol, tick_hz
                );
            }

            S2C::JoinOk {
                player_id,
                world_seed,
                start_tick,
                snapshot,
            } => {
                println!(
                    "[Client] JoinOk received (player_id: {}, world_seed: {}, start_tick: {})",
                    player_id, world_seed, start_tick
                );

                // Сохраняем player_id
                commands.insert_resource(LocalPlayerId(player_id));

                // Обновляем tick
                tick.last_server_tick = start_tick;

                // Добавляем начальный снапшот в буфер
                snapshot_buffer.push(start_tick, snapshot);
            }

            S2C::Delta {
                server_tick,
                base_tick,
                delta,
            } => {
                // Обновляем последний server tick
                tick.last_server_tick = server_tick;

                // Отправляем Ack обратно серверу
                let _ = channels
                    .outgoing_tx
                    .send(C2S::Ack {
                        last_server_tick: server_tick,
                    });

                // Применяем delta к буферу
                // TODO: Реализовать полное применение delta
                // Пока просто логируем
                println!(
                    "[Client] Delta received (server_tick: {}, base_tick: {}, spawns: {}, despawns: {}, updates: {})",
                    server_tick,
                    base_tick,
                    delta.spawns.len(),
                    delta.despawns.len(),
                    delta.updates.len()
                );
            }

            S2C::Event {
                server_tick,
                event_id,
                kind,
            } => {
                println!(
                    "[Client] Event received (server_tick: {}, event_id: {}, kind: {:?})",
                    server_tick, event_id, kind
                );
                // TODO: Обработать события (FX, инвентарь, etc)
            }

            S2C::Pong { timestamp_ms } => {
                // Вычисляем RTT
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let rtt_ms = now_ms.saturating_sub(timestamp_ms);
                println!("[Client] Pong received, RTT: {} ms", rtt_ms);
            }

            S2C::Kick { reason } => {
                println!("[Client] Kicked from server: {}", reason);
                // TODO: Вернуться в главное меню
            }
        }
    }
}

/// Система для захвата input от локального игрока
fn client_capture_input(
    channels: Option<Res<ClientChannelsResource>>,
    mut tick: ResMut<ClientTick>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Some(channels) = channels else {
        return;
    };

    // Увеличиваем client tick (работает на frame rate, не fixed timestep)
    tick.client_tick = tick.client_tick.wrapping_add(1);

    // Собираем input
    let mut move_x: i8 = 0;
    let mut move_y: i8 = 0;

    if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
        move_y += 100;
    }
    if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        move_y -= 100;
    }
    if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        move_x -= 100;
    }
    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        move_x += 100;
    }

    // Отправляем input только если есть движение или время прошло
    // Для начала отправляем каждый кадр
    if move_x != 0 || move_y != 0 || time.elapsed().as_secs_f32() as u32 % 2 == 0 {
        let input = InputCmd {
            client_seq: tick.client_seq,
            client_tick: tick.client_tick,
            move_x,
            move_y,
            aim_x: 0,  // TODO: Читать из мыши
            aim_y: 0,
            buttons: 0, // TODO: Обрабатывать кнопки действий
        };

        let _ = channels.outgoing_tx.send(C2S::Input(input));

        tick.client_seq = tick.client_seq.wrapping_add(1);
    }
}

/// Система для применения снапшотов к сущностям
fn client_apply_snapshot(
    snapshot_buffer: Res<SnapshotBuffer>,
    mut commands: Commands,
    mut entity_map: ResMut<NetworkEntityMap>,
    mut entities: Query<(
        Entity,
        &NetworkId,
        &mut PhysicsPosition,
        Option<&mut Velocity>,
        Option<&mut Health>,
    )>,
    local_player_id: Option<Res<LocalPlayerId>>,
) {
    // Берем последний снапшот из буфера
    let Some(buffered) = snapshot_buffer.get_latest() else {
        return;
    };

    let snapshot = &buffered.snapshot;

    // Применяем состояние игроков
    for player_state in &snapshot.players {
        apply_player_state(
            player_state,
            &mut commands,
            &mut entity_map,
            &mut entities,
            local_player_id.as_deref(),
        );
    }

    // TODO: Применить остальные сущности (pets, enemies, projectiles, pickups)
    // Каждый тип требует свою логику создания компонентов
}

/// Применяет состояние одного игрока
fn apply_player_state(
    player_state: &PlayerStateNet,
    commands: &mut Commands,
    entity_map: &mut NetworkEntityMap,
    entities: &mut Query<(
        Entity,
        &NetworkId,
        &mut PhysicsPosition,
        Option<&mut Velocity>,
        Option<&mut Health>,
    )>,
    local_player_id: Option<&LocalPlayerId>,
) {
    let net_id = player_state.id;

    // Проверяем, существует ли сущность
    if let Some(&entity) = entity_map.entities.get(&net_id) {
        // Обновляем существующую сущность
        if let Ok((_, _, mut pos, vel_opt, hp_opt)) = entities.get_mut(entity) {
            pos.0 = player_state.pos.to_f32();

            if let Some(mut vel) = vel_opt {
                vel.0 = player_state.vel.to_f32();
            }

            if let Some(mut hp) = hp_opt {
                hp.current = player_state.hp as f32;
                hp.max = player_state.hp_max as f32;
            }
        }
    } else {
        // Создаем новую сущность игрока
        let is_local = local_player_id.map_or(false, |id| id.0 == player_state.player_id);

        let mut entity_commands = commands.spawn((
            NetworkId(net_id),
            PhysicsPosition(player_state.pos.to_f32()),
            Velocity(player_state.vel.to_f32()),
            Health {
                current: player_state.hp as f32,
                max: player_state.hp_max as f32,
            },
            PlayerId(player_state.player_id as u32),
        ));

        // Добавляем маркер локального или удаленного игрока
        if is_local {
            entity_commands.insert(LocalPlayer);
        } else {
            entity_commands.insert(RemotePlayer);
        }

        let entity = entity_commands.id();

        entity_map.entities.insert(net_id, entity);

        println!(
            "[Client] Created player entity (net_id: {}, player_id: {}, local: {})",
            net_id, player_state.player_id, is_local
        );
    }
}


/// Применяет WorldDelta к текущему состоянию мира
fn _apply_delta(
    delta: &WorldDelta,
    commands: &mut Commands,
    entity_map: &mut NetworkEntityMap,
    entities: &mut Query<(
        Entity,
        &NetworkId,
        &mut PhysicsPosition,
        Option<&mut Velocity>,
        Option<&mut Health>,
    )>,
) {
    // Spawns
    for _spawn in &delta.spawns {
        // TODO: Создать новую сущность на основе SpawnNet
    }

    // Despawns
    for &net_id in &delta.despawns {
        if let Some(entity) = entity_map.entities.remove(&net_id) {
            commands.entity(entity).despawn();
        }
    }

    // Updates
    for update in &delta.updates {
        if let Some(&entity) = entity_map.entities.get(&update.id) {
            if let Ok((_, _, mut pos, vel_opt, hp_opt)) = entities.get_mut(entity) {
                if let Some(new_pos) = update.pos {
                    pos.0 = new_pos.to_f32();
                }
                if let (Some(new_vel), Some(mut vel)) = (update.vel, vel_opt) {
                    vel.0 = new_vel.to_f32();
                }
                if let (Some(new_hp), Some(mut hp)) = (update.hp, hp_opt) {
                    hp.current = new_hp as f32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_tick_creation() {
        let tick = ClientTick::default();
        assert_eq!(tick.client_seq, 0);
        assert_eq!(tick.client_tick, 0);
        assert_eq!(tick.last_server_tick, 0);
    }

    #[test]
    fn test_snapshot_buffer_push() {
        let mut buffer = SnapshotBuffer::default();

        let snapshot1 = Snapshot {
            players: vec![],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        buffer.push(10, snapshot1.clone());
        assert_eq!(buffer.snapshots.len(), 1);

        buffer.push(20, snapshot1.clone());
        assert_eq!(buffer.snapshots.len(), 2);

        // Проверяем сортировку
        assert_eq!(buffer.snapshots[0].server_tick, 10);
        assert_eq!(buffer.snapshots[1].server_tick, 20);
    }

    #[test]
    fn test_snapshot_buffer_out_of_order() {
        let mut buffer = SnapshotBuffer::default();

        let snapshot = Snapshot {
            players: vec![],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        buffer.push(20, snapshot.clone());
        buffer.push(10, snapshot.clone());
        buffer.push(15, snapshot.clone());

        // Должны быть отсортированы
        assert_eq!(buffer.snapshots[0].server_tick, 10);
        assert_eq!(buffer.snapshots[1].server_tick, 15);
        assert_eq!(buffer.snapshots[2].server_tick, 20);
    }

    #[test]
    fn test_snapshot_buffer_overflow() {
        let mut buffer = SnapshotBuffer::default();

        let snapshot = Snapshot {
            players: vec![],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        // Добавляем больше, чем вмещается в буфер
        for i in 0..(SNAPSHOT_BUFFER_SIZE + 5) {
            buffer.push(i as u32, snapshot.clone());
        }

        // Размер не должен превышать лимит
        assert_eq!(buffer.snapshots.len(), SNAPSHOT_BUFFER_SIZE);

        // Первый снапшот должен быть tick=5 (первые 5 удалены)
        assert_eq!(buffer.snapshots[0].server_tick, 5);
    }

    #[test]
    fn test_snapshot_buffer_interpolation_pair() {
        let mut buffer = SnapshotBuffer::default();

        let snapshot = Snapshot {
            players: vec![],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        // Недостаточно снапшотов
        assert!(buffer.get_interpolation_pair().is_none());

        buffer.push(10, snapshot.clone());
        assert!(buffer.get_interpolation_pair().is_none());

        buffer.push(20, snapshot.clone());
        buffer.push(30, snapshot.clone());
        buffer.push(40, snapshot.clone());

        // Теперь должна быть пара
        let pair = buffer.get_interpolation_pair();
        assert!(pair.is_some());

        // С target_delay=2, должны получить снапшоты с индексами 1 и 2
        let (a, b) = pair.unwrap();
        assert_eq!(a.server_tick, 20);
        assert_eq!(b.server_tick, 30);
    }

    #[test]
    fn test_snapshot_buffer_get_latest() {
        let mut buffer = SnapshotBuffer::default();

        assert!(buffer.get_latest().is_none());

        let snapshot = Snapshot {
            players: vec![],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        buffer.push(10, snapshot.clone());
        buffer.push(20, snapshot.clone());

        let latest = buffer.get_latest();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().server_tick, 20);
    }
}
