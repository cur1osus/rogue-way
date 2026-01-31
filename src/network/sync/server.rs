use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::components::{Boss, Health, PhysicsPosition, PlayerId, Velocity};
use crate::network::protocol::delta::{UpdateNet, WorldDelta};
use crate::network::protocol::messages::{C2S, InputCmd, S2C};
use crate::network::protocol::quantization::Quantize;
use crate::network::protocol::snapshots::{NetId, PlayerStateNet, Snapshot, SpawnNet};
use crate::network::transport::channels::{ConnId, ServerChannelsResource};
use crate::network::world::baseline::ClientBaseline;
use crate::network::world::interest::calculate_interest;
use crate::network::world::net_id::NetworkId;
use crate::network::NetworkMode;

/// Tick rate сервера (Hz)
pub const SERVER_TICK_HZ: u16 = 30;
pub const SERVER_TICK_INTERVAL: f32 = 1.0 / SERVER_TICK_HZ as f32;

/// Максимальный размер буфера входящих input команд
const INPUT_BUFFER_SIZE: usize = 64;

/// Resource для отслеживания server tick
#[derive(Resource)]
pub struct ServerTick {
    pub tick: u32,
    pub tick_hz: u16,
    pub accumulator: f32,
}

impl Default for ServerTick {
    fn default() -> Self {
        Self {
            tick: 0,
            tick_hz: SERVER_TICK_HZ,
            accumulator: 0.0,
        }
    }
}

/// Сессия одного подключенного клиента
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub conn_id: ConnId,
    pub player_id: u8,
    pub baseline: ClientBaseline,
    pub last_input: Option<InputCmd>,
    pub interest_set: crate::network::world::interest::InterestSet,
}

impl ClientSession {
    pub fn new(conn_id: ConnId, player_id: u8) -> Self {
        Self {
            conn_id,
            player_id,
            baseline: ClientBaseline::new(player_id),
            last_input: None,
            interest_set: crate::network::world::interest::InterestSet::default(),
        }
    }
}

/// Resource для управления сессиями клиентов
#[derive(Resource, Default)]
pub struct ClientSessions {
    pub sessions: HashMap<ConnId, ClientSession>,
    next_player_id: u8,
}

impl ClientSessions {
    pub fn add_session(&mut self, conn_id: ConnId) -> u8 {
        let player_id = self.next_player_id;
        self.next_player_id = (self.next_player_id + 1) % 4; // Максимум 4 игрока

        self.sessions
            .insert(conn_id, ClientSession::new(conn_id, player_id));

        player_id
    }

    pub fn remove_session(&mut self, conn_id: ConnId) -> Option<ClientSession> {
        self.sessions.remove(&conn_id)
    }

    pub fn get_session(&self, conn_id: ConnId) -> Option<&ClientSession> {
        self.sessions.get(&conn_id)
    }

    pub fn get_session_mut(&mut self, conn_id: ConnId) -> Option<&mut ClientSession> {
        self.sessions.get_mut(&conn_id)
    }
}

/// Resource для буферизации input команд от клиентов
#[derive(Resource, Default)]
pub struct InputBuffers {
    pub buffers: HashMap<u8, VecDeque<InputCmd>>,
}

impl InputBuffers {
    pub fn push_input(&mut self, player_id: u8, input: InputCmd) {
        let buffer = self.buffers.entry(player_id).or_insert_with(VecDeque::new);

        if buffer.len() >= INPUT_BUFFER_SIZE {
            buffer.pop_front();
        }

        buffer.push_back(input);
    }

    pub fn pop_input(&mut self, player_id: u8) -> Option<InputCmd> {
        self.buffers.get_mut(&player_id)?.pop_front()
    }
}

/// Wrapper для incoming receiver (нужен Mutex для мутабельности из системы)
#[derive(Resource)]
pub struct ServerIncomingRx(pub Arc<Mutex<mpsc::UnboundedReceiver<(ConnId, C2S)>>>);

/// Плагин для серверной части networking
pub struct NetServerPlugin;

impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServerTick>()
            .init_resource::<ClientSessions>()
            .init_resource::<InputBuffers>()
            .add_systems(Startup, setup_server_channels)
            .add_systems(
                FixedUpdate,
                server_fixed_tick.run_if(is_host),
            )
            .add_systems(
                Update,
                (
                    server_read_incoming,
                    server_send_outgoing,
                    server_handle_disconnects,
                )
                    .run_if(is_host),
            );
    }
}

/// Condition для запуска систем только на хосте
fn is_host(mode: Option<Res<NetworkMode>>) -> bool {
    matches!(mode.as_deref(), Some(NetworkMode::Host))
}

/// Fixed timestep система для server tick (30 Hz)
fn server_fixed_tick(
    mut tick: ResMut<ServerTick>,
    mut input_buffers: ResMut<InputBuffers>,
    time: Res<Time>,
    mut players: Query<(
        &NetworkId,
        &PlayerId,
        &mut PhysicsPosition,
        &mut Velocity,
        &crate::components::MovementSpeed,
    )>,
) {
    // Накапливаем время
    tick.accumulator += time.delta().as_secs_f32();

    // Выполняем fixed timestep обновления
    while tick.accumulator >= SERVER_TICK_INTERVAL {
        tick.accumulator -= SERVER_TICK_INTERVAL;
        tick.tick = tick.tick.wrapping_add(1);

        // Применяем inputs для каждого игрока
        for (_, player_id, mut pos, mut vel, move_speed) in players.iter_mut() {
            let pid = player_id.0 as u8;

            // Получаем input для этого игрока
            if let Some(input) = input_buffers.pop_input(pid) {
                // Применяем движение
                let move_dir = Vec2::new(
                    input.move_x as f32 / 100.0, // Нормализуем от -100..100 к -1..1
                    input.move_y as f32 / 100.0,
                );

                // Обновляем скорость
                vel.0 = move_dir * move_speed.0;

                // Обновляем позицию (физическая интеграция)
                pos.0 += vel.0 * SERVER_TICK_INTERVAL;

                // TODO: Обработать buttons (атаки, способности)
            } else {
                // Нет input - останавливаем игрока
                vel.0 = Vec2::ZERO;
            }
        }

        // TODO: Запускать gameplay системы (враги, коллизии, etc)
    }
}

/// Startup система для извлечения receivers из channels
fn setup_server_channels(
    channels_res: Option<ResMut<ServerChannelsResource>>,
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
            commands.insert_resource(ServerIncomingRx(Arc::new(Mutex::new(incoming_rx))));
            println!("[Server] Server channels initialized");
        }
    } else {
        // Если не получилось, значит есть другие ссылки - используем fallback
        println!("[Server] Warning: Could not extract incoming_rx - multiple Arc references");
    }
}

/// Система для чтения входящих сообщений от клиентов
fn server_read_incoming(
    incoming_rx: Option<Res<ServerIncomingRx>>,
    mut sessions: ResMut<ClientSessions>,
    mut input_buffers: ResMut<InputBuffers>,
    tick: Res<ServerTick>,
    channels: Option<Res<ServerChannelsResource>>,
) {
    let Some(incoming_rx) = incoming_rx else {
        return;
    };

    let Some(channels) = channels else {
        return;
    };

    // Обрабатываем все доступные сообщения (non-blocking)
    let mut rx = incoming_rx.0.lock().unwrap();

    while let Ok((conn_id, msg)) = rx.try_recv() {
        match msg {
            C2S::Hello { build, protocol } => {
                println!("[Server] Hello from conn {} (build: {}, protocol: {})", conn_id, build, protocol);

                // Отправляем HelloOk
                let response = S2C::HelloOk {
                    protocol,
                    tick_hz: tick.tick_hz,
                };

                let _ = channels.outgoing_tx.send((conn_id, response));
            }

            C2S::Join { name } => {
                println!("[Server] Join request from conn {}: {}", conn_id, name);

                // Создаем новую сессию
                let player_id = sessions.add_session(conn_id);

                println!("[Server] Assigned player_id {} to conn {}", player_id, conn_id);

                // Отправляем JoinOk с текущим snapshot
                // TODO: Собрать реальный snapshot из ECS
                let snapshot = Snapshot {
                    players: vec![],
                    pets: vec![],
                    enemies: vec![],
                    projectiles: vec![],
                    xp_gems: vec![],
                    gold: vec![],
                };

                let response = S2C::JoinOk {
                    player_id,
                    world_seed: 12345, // TODO: использовать реальный seed
                    start_tick: tick.tick,
                    snapshot,
                };

                let _ = channels.outgoing_tx.send((conn_id, response));
            }

            C2S::Input(input) => {
                // Находим player_id для этого connection
                if let Some(session) = sessions.get_session_mut(conn_id) {
                    let player_id = session.player_id;

                    // Сохраняем input в буфер
                    input_buffers.push_input(player_id, input.clone());

                    // Обновляем last_input в сессии
                    session.last_input = Some(input);
                }
            }

            C2S::Ack { last_server_tick } => {
                // Обновляем baseline клиента
                if let Some(session) = sessions.get_session_mut(conn_id) {
                    session.baseline.update_ack(last_server_tick);
                }
            }

            C2S::Ping { timestamp_ms } => {
                // Отправляем Pong обратно
                let response = S2C::Pong { timestamp_ms };
                let _ = channels.outgoing_tx.send((conn_id, response));
            }
        }
    }
}

/// Система для отправки исходящих сообщений клиентам
fn server_send_outgoing(
    mut sessions: ResMut<ClientSessions>,
    tick: Res<ServerTick>,
    channels: Option<Res<ServerChannelsResource>>,
    all_entities: Query<(
        Entity,
        &NetworkId,
        &PhysicsPosition,
        Option<&PlayerId>,
        Option<&Velocity>,
        Option<&Health>,
        Option<&Boss>,
    )>,
    players: Query<(&NetworkId, &PhysicsPosition), With<PlayerId>>,
    entities_for_interest: Query<(
        Entity,
        &NetworkId,
        &PhysicsPosition,
        Option<&crate::components::Player>,
        Option<&Boss>,
    )>,
) {
    let Some(channels) = channels else {
        return;
    };

    // Отправляем delta каждому клиенту
    for session in sessions.sessions.values_mut() {
        // Находим позицию игрока этого клиента
        let player_pos = players
            .iter()
            .find(|(net_id, _)| {
                // Ищем игрока с соответствующим player_id
                all_entities
                    .iter()
                    .any(|(_, nid, _, pid_opt, _, _, _)| {
                        nid.0 == net_id.0
                            && pid_opt.map_or(false, |pid| pid.0 == session.player_id as u32)
                    })
            })
            .map(|(_, pos)| pos.0);

        let Some(player_pos) = player_pos else {
            // Игрок не найден, пропускаем
            continue;
        };

        // Вычисляем interest set для этого клиента
        let interest_set = calculate_interest(
            player_pos,
            Some(&session.interest_set),
            &entities_for_interest,
        );

        // Собираем delta на основе baseline и interest set
        let delta = collect_delta_for_client(
            &interest_set,
            &mut session.baseline,
            &all_entities,
            tick.tick,
        );

        // Обновляем interest set клиента
        session.interest_set = interest_set;

        // Отправляем только если есть изменения
        if !delta.spawns.is_empty() || !delta.despawns.is_empty() || !delta.updates.is_empty() {
            let msg = S2C::Delta {
                server_tick: tick.tick,
                base_tick: session.baseline.last_acked_tick,
                delta,
            };

            let _ = channels.outgoing_tx.send((session.conn_id, msg));
        }
    }
}

/// Система для обработки disconnects
fn server_handle_disconnects(
    channels: Option<Res<ServerChannelsResource>>,
    mut sessions: ResMut<ClientSessions>,
) {
    let Some(channels) = channels else {
        return;
    };

    // Проверяем disconnect события
    // NOTE: disconnect_rx также нужно извлечь в startup, но пока упростим
    // TODO: Добавить ServerDisconnectRx Resource и обрабатывать disconnects
}

/// Helper функция для сбора delta для клиента на основе interest set и baseline
fn collect_delta_for_client(
    interest_set: &crate::network::world::interest::InterestSet,
    baseline: &mut ClientBaseline,
    all_entities: &Query<(
        Entity,
        &NetworkId,
        &PhysicsPosition,
        Option<&PlayerId>,
        Option<&Velocity>,
        Option<&Health>,
        Option<&Boss>,
    )>,
    current_tick: u32,
) -> WorldDelta {
    let mut delta = WorldDelta {
        spawns: vec![],
        despawns: vec![],
        updates: vec![],
    };

    // 1. Собираем despawns - сущности, которые вышли из interest set
    for net_id in interest_set.get_despawns() {
        delta.despawns.push(net_id);
        baseline.remove_entity(net_id);
    }

    // 2. Собираем spawns и updates для сущностей в interest set
    for net_id in &interest_set.entities {
        // Находим сущность в ECS
        let entity_data = all_entities
            .iter()
            .find(|(_, nid, _, _, _, _, _)| nid.0 == *net_id);

        let Some((_, _, pos, player_id_opt, vel_opt, hp_opt, _)) = entity_data else {
            continue;
        };

        let pos_q = pos.0.to_i16();
        let vel_q = vel_opt.map(|v| v.0.to_i16()).unwrap_or_default();
        let hp = hp_opt.map(|h| h.current as u16).unwrap_or(0);

        // Проверяем, новая это сущность или обновление
        if baseline.has_changed(*net_id, pos_q, vel_q, hp) {
            // Сущность изменилась
            if baseline.get_entity(*net_id).is_none() {
                // Новая сущность - spawn
                let hp_max = hp_opt.map(|h| h.max as u16).unwrap_or(0);
                let spawn = SpawnNet {
                    id: *net_id,
                    kind: determine_entity_kind(player_id_opt),
                    pos: pos_q,
                    vel: vel_q,
                    hp: Some(hp),
                    hp_max: Some(hp_max),
                };
                delta.spawns.push(spawn);
            } else {
                // Существующая сущность - update
                let mut update = UpdateNet::new(*net_id);

                // Добавляем только изменённые поля
                let old_baseline = baseline.get_entity(*net_id).unwrap();

                if old_baseline.pos != pos_q {
                    update = update.with_pos(pos_q);
                }
                if old_baseline.vel != vel_q {
                    update = update.with_vel(vel_q);
                }
                if old_baseline.hp != hp {
                    update = update.with_hp(hp);
                }

                delta.updates.push(update);
            }

            // Обновляем baseline
            baseline.update_entity(*net_id, pos_q, vel_q, hp, current_tick);
        }
    }

    // 3. Cleanup старых baselines (для сущностей, которые давно не обновлялись)
    baseline.cleanup_old_entities(current_tick, 300); // 300 ticks = 10 sec at 30 Hz

    delta
}

/// Определяет EntityKind на основе компонентов сущности
fn determine_entity_kind(player_id_opt: Option<&PlayerId>) -> crate::network::protocol::snapshots::EntityKind {
    if let Some(player_id) = player_id_opt {
        return crate::network::protocol::snapshots::EntityKind::Player {
            player_id: player_id.0 as u8,
        };
    }

    // TODO: Определять другие типы (Pet, Enemy, Projectile, XpGem, Gold)
    // Пока возвращаем Projectile как fallback
    crate::network::protocol::snapshots::EntityKind::Projectile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_tick_creation() {
        let tick = ServerTick::default();
        assert_eq!(tick.tick, 0);
        assert_eq!(tick.tick_hz, SERVER_TICK_HZ);
        assert_eq!(tick.accumulator, 0.0);
    }

    #[test]
    fn test_client_session_creation() {
        let session = ClientSession::new(123, 1);
        assert_eq!(session.conn_id, 123);
        assert_eq!(session.player_id, 1);
        assert!(session.last_input.is_none());
    }

    #[test]
    fn test_client_sessions_add_remove() {
        let mut sessions = ClientSessions::default();

        let player_id1 = sessions.add_session(100);
        let player_id2 = sessions.add_session(200);

        assert_eq!(sessions.sessions.len(), 2);
        assert_eq!(player_id1, 0);
        assert_eq!(player_id2, 1);

        let removed = sessions.remove_session(100);
        assert!(removed.is_some());
        assert_eq!(sessions.sessions.len(), 1);
    }

    #[test]
    fn test_input_buffers_push_pop() {
        let mut buffers = InputBuffers::default();

        let input1 = InputCmd {
            client_seq: 1,
            client_tick: 10,
            move_x: 10,
            move_y: 0,
            aim_x: 0,
            aim_y: 0,
            buttons: 0,
        };

        buffers.push_input(0, input1.clone());

        let popped = buffers.pop_input(0);
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().client_seq, 1);
    }

    #[test]
    fn test_input_buffers_overflow() {
        let mut buffers = InputBuffers::default();

        for i in 0..INPUT_BUFFER_SIZE + 10 {
            let input = InputCmd {
                client_seq: i as u32,
                client_tick: i as u32,
                move_x: 0,
                move_y: 0,
                aim_x: 0,
                aim_y: 0,
                buttons: 0,
            };
            buffers.push_input(0, input);
        }

        assert_eq!(buffers.buffers.get(&0).unwrap().len(), INPUT_BUFFER_SIZE);

        let first = buffers.pop_input(0).unwrap();
        assert_eq!(first.client_seq, 10);
    }
}
