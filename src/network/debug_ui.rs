use bevy::prelude::*;
use std::collections::VecDeque;

use crate::network::sync::client::{ClientTick, SnapshotBuffer};
use crate::network::sync::server::{ClientSessions, ServerTick};
use crate::network::world::net_id::NetworkEntityMap;

/// Размер буфера для истории RTT
const RTT_HISTORY_SIZE: usize = 60;

/// Resource для отслеживания networking метрик
#[derive(Resource)]
pub struct NetworkDebugMetrics {
    /// Round Trip Time в миллисекундах
    pub rtt_ms: f32,
    /// История RTT для отображения
    pub rtt_history: VecDeque<f32>,
    /// Количество bytes отправлено (за последнюю секунду)
    pub bytes_sent: usize,
    /// Количество bytes получено (за последнюю секунду)
    pub bytes_received: usize,
    /// Accumulator для reset счётчиков
    pub stats_accumulator: f32,
}

impl Default for NetworkDebugMetrics {
    fn default() -> Self {
        Self {
            rtt_ms: 0.0,
            rtt_history: VecDeque::with_capacity(RTT_HISTORY_SIZE),
            bytes_sent: 0,
            bytes_received: 0,
            stats_accumulator: 0.0,
        }
    }
}

impl NetworkDebugMetrics {
    /// Добавляет RTT измерение в историю
    pub fn add_rtt_sample(&mut self, rtt: f32) {
        self.rtt_ms = rtt;

        if self.rtt_history.len() >= RTT_HISTORY_SIZE {
            self.rtt_history.pop_front();
        }
        self.rtt_history.push_back(rtt);
    }

    /// Получает среднее RTT
    pub fn average_rtt(&self) -> f32 {
        if self.rtt_history.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.rtt_history.iter().sum();
        sum / self.rtt_history.len() as f32
    }

    /// Получает min RTT
    pub fn min_rtt(&self) -> f32 {
        self.rtt_history
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    /// Получает max RTT
    pub fn max_rtt(&self) -> f32 {
        self.rtt_history
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Форматирует все метрики в строку для отображения
    pub fn format_stats(
        &self,
        server_tick: Option<&ServerTick>,
        sessions: Option<&ClientSessions>,
        client_tick: Option<&ClientTick>,
        snapshot_buffer: Option<&SnapshotBuffer>,
        entity_map: Option<&NetworkEntityMap>,
    ) -> String {
        let mut output = String::from("=== Network Debug ===\n\n");

        let is_server = server_tick.is_some();
        let is_client = client_tick.is_some();

        if !is_server && !is_client {
            return output + "Networking not active";
        }

        let mode = if is_server { "Host/Server" } else { "Client" };
        output += &format!("Mode: {}\n\n", mode);

        // Server stats
        if let Some(tick) = server_tick {
            output += "▶ Server\n";
            output += &format!("  Server Tick: {}\n", tick.tick);
            output += &format!("  Tick Hz: {}\n", tick.tick_hz);

            if let Some(sessions) = sessions {
                output += &format!("  Clients: {}\n", sessions.sessions.len());
            }
            output += "\n";
        }

        // Client stats
        if let Some(tick) = client_tick {
            output += "▶ Client\n";
            output += &format!("  Client Seq: {}\n", tick.client_seq);
            output += &format!("  Last Server Tick: {}\n", tick.last_server_tick);

            if let Some(buffer) = snapshot_buffer {
                output += &format!("  Snapshots: {}\n", buffer.snapshots.len());
            }
            output += "\n";
        }

        // Network metrics
        output += "▶ Network Metrics\n";
        output += &format!("  RTT: {:.1} ms\n", self.rtt_ms);
        output += &format!("  Avg RTT: {:.1} ms\n", self.average_rtt());

        if !self.rtt_history.is_empty() {
            output += &format!("  Min RTT: {:.1} ms\n", self.min_rtt());
            output += &format!("  Max RTT: {:.1} ms\n", self.max_rtt());
        }

        let kb_sent = self.bytes_sent as f32 / 1024.0;
        let kb_received = self.bytes_received as f32 / 1024.0;
        output += &format!("  ⬆ Out: {:.2} KB/s\n", kb_sent);
        output += &format!("  ⬇ In: {:.2} KB/s\n\n", kb_received);

        // Entities
        if let Some(map) = entity_map {
            output += "▶ Entities\n";
            output += &format!("  Replicated: {}\n\n", map.entities.len());
        }

        output += "Press F3 to toggle";
        output
    }
}

/// Marker component для Debug UI text
#[derive(Component)]
pub struct NetworkDebugUiText;

/// Marker component для Debug UI root node
#[derive(Component)]
pub struct NetworkDebugUiRoot;

/// Resource для состояния видимости Debug UI
#[derive(Resource, Default)]
pub struct NetworkDebugUiVisible(pub bool);

/// Плагин для Debug UI
pub struct NetworkDebugUiPlugin;

impl Plugin for NetworkDebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkDebugMetrics>()
            .init_resource::<NetworkDebugUiVisible>()
            .add_systems(Startup, spawn_debug_ui)
            .add_systems(Update, (toggle_debug_ui, update_debug_ui, update_bandwidth_stats));
    }
}

/// Spawn Debug UI overlay
fn spawn_debug_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            NetworkDebugUiRoot,
            Visibility::Hidden, // Скрыт по умолчанию
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("=== Network Debug ===\n\nInitializing..."),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                NetworkDebugUiText,
            ));
        });
}

/// Переключает видимость Debug UI по нажатию F3
fn toggle_debug_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<NetworkDebugUiVisible>,
    mut ui_query: Query<&mut Visibility, With<NetworkDebugUiRoot>>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        visible.0 = !visible.0;

        for mut visibility in ui_query.iter_mut() {
            *visibility = if visible.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        println!("[Debug UI] {}", if visible.0 { "Shown" } else { "Hidden" });
    }
}

/// Обновляет Debug UI с текущими метриками
fn update_debug_ui(
    metrics: Res<NetworkDebugMetrics>,
    server_tick: Option<Res<ServerTick>>,
    sessions: Option<Res<ClientSessions>>,
    client_tick: Option<Res<ClientTick>>,
    snapshot_buffer: Option<Res<SnapshotBuffer>>,
    entity_map: Option<Res<NetworkEntityMap>>,
    mut text_query: Query<&mut Text, With<NetworkDebugUiText>>,
) {
    for mut text in text_query.iter_mut() {
        let stats = metrics.format_stats(
            server_tick.as_deref(),
            sessions.as_deref(),
            client_tick.as_deref(),
            snapshot_buffer.as_deref(),
            entity_map.as_deref(),
        );
        text.0 = stats;
    }
}

/// Система для обновления bandwidth статистики
fn update_bandwidth_stats(mut metrics: ResMut<NetworkDebugMetrics>, time: Res<Time>) {
    metrics.stats_accumulator += time.delta().as_secs_f32();

    if metrics.stats_accumulator >= 1.0 {
        metrics.stats_accumulator -= 1.0;
        // NOTE: В реальной реализации обновлять из channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = NetworkDebugMetrics::default();
        assert_eq!(metrics.rtt_ms, 0.0);
        assert_eq!(metrics.rtt_history.len(), 0);
    }

    #[test]
    fn test_rtt_sample() {
        let mut metrics = NetworkDebugMetrics::default();

        metrics.add_rtt_sample(50.0);
        assert_eq!(metrics.rtt_ms, 50.0);
        assert_eq!(metrics.rtt_history.len(), 1);
    }

    #[test]
    fn test_average_rtt() {
        let mut metrics = NetworkDebugMetrics::default();
        assert_eq!(metrics.average_rtt(), 0.0);

        metrics.add_rtt_sample(50.0);
        metrics.add_rtt_sample(60.0);
        metrics.add_rtt_sample(70.0);

        assert_eq!(metrics.average_rtt(), 60.0);
    }

    #[test]
    fn test_min_max_rtt() {
        let mut metrics = NetworkDebugMetrics::default();

        metrics.add_rtt_sample(50.0);
        metrics.add_rtt_sample(60.0);
        metrics.add_rtt_sample(70.0);

        assert_eq!(metrics.min_rtt(), 50.0);
        assert_eq!(metrics.max_rtt(), 70.0);
    }

    #[test]
    fn test_format_stats() {
        let metrics = NetworkDebugMetrics::default();
        let stats = metrics.format_stats(None, None, None, None, None);
        assert!(stats.contains("Network Debug"));
        assert!(stats.contains("Networking not active"));
    }
}
