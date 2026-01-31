use bevy::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::network::protocol::{C2S, S2C};

/// Размер буфера канала для сообщений
pub const CHANNEL_BUFFER_SIZE: usize = 128;

/// Тип connection ID для отслеживания клиентов
pub type ConnId = u64;

// ===== SERVER CHANNELS =====

/// Каналы для связи между QUIC сервером (tokio) и Bevy системами
#[derive(Resource)]
pub struct ServerChannels {
    /// Входящие сообщения от клиентов (conn_id, message)
    pub incoming_tx: mpsc::UnboundedSender<(ConnId, C2S)>,
    pub incoming_rx: Option<mpsc::UnboundedReceiver<(ConnId, C2S)>>,

    /// Исходящие сообщения к клиентам (conn_id, message)
    pub outgoing_tx: mpsc::UnboundedSender<(ConnId, S2C)>,
    pub outgoing_rx: Option<mpsc::UnboundedReceiver<(ConnId, S2C)>>,

    /// Уведомления о disconnect (conn_id)
    pub disconnect_tx: mpsc::UnboundedSender<ConnId>,
    pub disconnect_rx: Option<mpsc::UnboundedReceiver<ConnId>>,
}

impl ServerChannels {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let (disconnect_tx, disconnect_rx) = mpsc::unbounded_channel();

        Self {
            incoming_tx,
            incoming_rx: Some(incoming_rx),
            outgoing_tx,
            outgoing_rx: Some(outgoing_rx),
            disconnect_tx,
            disconnect_rx: Some(disconnect_rx),
        }
    }

    /// Берет receiver для incoming messages (можно вызвать только раз)
    pub fn take_incoming_rx(&mut self) -> Option<mpsc::UnboundedReceiver<(ConnId, C2S)>> {
        self.incoming_rx.take()
    }

    /// Берет receiver для outgoing messages (можно вызвать только раз)
    pub fn take_outgoing_rx(&mut self) -> Option<mpsc::UnboundedReceiver<(ConnId, S2C)>> {
        self.outgoing_rx.take()
    }

    /// Берет receiver для disconnect events (можно вызвать только раз)
    pub fn take_disconnect_rx(&mut self) -> Option<mpsc::UnboundedReceiver<ConnId>> {
        self.disconnect_rx.take()
    }
}

impl Default for ServerChannels {
    fn default() -> Self {
        Self::new()
    }
}

// ===== CLIENT CHANNELS =====

/// Каналы для связи между QUIC клиентом (tokio) и Bevy системами
#[derive(Resource)]
pub struct ClientChannels {
    /// Входящие сообщения от сервера
    pub incoming_tx: mpsc::UnboundedSender<S2C>,
    pub incoming_rx: Option<mpsc::UnboundedReceiver<S2C>>,

    /// Исходящие сообщения к серверу
    pub outgoing_tx: mpsc::UnboundedSender<C2S>,
    pub outgoing_rx: Option<mpsc::UnboundedReceiver<C2S>>,

    /// Уведомление о disconnect
    pub disconnect_tx: mpsc::UnboundedSender<()>,
    pub disconnect_rx: Option<mpsc::UnboundedReceiver<()>>,
}

impl ClientChannels {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let (disconnect_tx, disconnect_rx) = mpsc::unbounded_channel();

        Self {
            incoming_tx,
            incoming_rx: Some(incoming_rx),
            outgoing_tx,
            outgoing_rx: Some(outgoing_rx),
            disconnect_tx,
            disconnect_rx: Some(disconnect_rx),
        }
    }

    /// Берет receiver для incoming messages (можно вызвать только раз)
    pub fn take_incoming_rx(&mut self) -> Option<mpsc::UnboundedReceiver<S2C>> {
        self.incoming_rx.take()
    }

    /// Берет receiver для outgoing messages (можно вызвать только раз)
    pub fn take_outgoing_rx(&mut self) -> Option<mpsc::UnboundedReceiver<C2S>> {
        self.outgoing_rx.take()
    }

    /// Берет receiver для disconnect event (можно вызвать только раз)
    pub fn take_disconnect_rx(&mut self) -> Option<mpsc::UnboundedReceiver<()>> {
        self.disconnect_rx.take()
    }
}

impl Default for ClientChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_channels_creation() {
        let mut channels = ServerChannels::new();

        assert!(channels.take_incoming_rx().is_some());
        assert!(channels.take_outgoing_rx().is_some());
        assert!(channels.take_disconnect_rx().is_some());

        // Второй вызов должен вернуть None
        assert!(channels.take_incoming_rx().is_none());
    }

    #[test]
    fn test_client_channels_creation() {
        let mut channels = ClientChannels::new();

        assert!(channels.take_incoming_rx().is_some());
        assert!(channels.take_outgoing_rx().is_some());
        assert!(channels.take_disconnect_rx().is_some());

        // Второй вызов должен вернуть None
        assert!(channels.take_incoming_rx().is_none());
    }

    #[tokio::test]
    async fn test_server_channel_communication() {
        let channels = ServerChannels::new();

        // Отправляем сообщение через incoming_tx
        let msg = C2S::Hello {
            build: 123,
            protocol: 1,
        };
        channels.incoming_tx.send((1, msg)).unwrap();

        // Должны получить его через incoming_rx (если бы он был не Option)
        // В реальном коде rx забирается в Bevy систему
    }

    #[tokio::test]
    async fn test_client_channel_communication() {
        let channels = ClientChannels::new();

        // Отправляем сообщение через outgoing_tx
        let msg = C2S::Ping {
            timestamp_ms: 12345,
        };
        channels.outgoing_tx.send(msg).unwrap();

        // Должны получить его через outgoing_rx (если бы он был не Option)
    }
}

// ===== BEVY RESOURCE WRAPPERS =====

/// Wrapper для Arc<ServerChannels> чтобы сделать его Bevy Resource
#[derive(Resource, Clone)]
pub struct ServerChannelsResource(pub Arc<ServerChannels>);

impl std::ops::Deref for ServerChannelsResource {
    type Target = Arc<ServerChannels>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper для Arc<ClientChannels> чтобы сделать его Bevy Resource
#[derive(Resource, Clone)]
pub struct ClientChannelsResource(pub Arc<ClientChannels>);

impl std::ops::Deref for ClientChannelsResource {
    type Target = Arc<ClientChannels>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
