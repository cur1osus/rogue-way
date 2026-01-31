/// Helper функции для интеграции QUIC networking с Bevy UI
/// Упрощают запуск клиента и сервера из main menu

use bevy::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;

use super::transport::{
    ClientChannels, ClientChannelsResource, QuicClient, QuicServer, ServerChannels,
    ServerChannelsResource,
};
use super::NetworkMode;

/// Запускает QUIC сервер (Host mode)
/// Возвращает join code (адрес сервера) для отображения в UI
pub fn start_quic_host(commands: &mut Commands, port: u16) -> Result<String, String> {
    // Создаем ServerChannels в Bevy контексте
    let channels = Arc::new(ServerChannels::new());
    let channels_clone = channels.clone();

    // Вставляем channels как Bevy Resource для использования в системах
    commands.insert_resource(ServerChannelsResource(channels));

    // Создаем tokio runtime для async операций
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let addr = format!("0.0.0.0:{}", port);
    let join_code = format!("127.0.0.1:{}", port); // Для локального подключения
    let join_code_clone = join_code.clone(); // Клонируем для closure

    // Запускаем QUIC сервер в фоновом потоке
    rt.spawn(async move {
        match QuicServer::bind(&addr, channels_clone).await {
            Ok(server) => {
                println!("[QUIC Host] Server started on {}", addr);
                println!("[QUIC Host] Join code: {}", join_code_clone);
                let server = Arc::new(server);

                // Запускаем accept loop
                server.spawn_loops();

                // Держим runtime живым
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            }
            Err(e) => {
                eprintln!("[QUIC Host] Failed to start server: {}", e);
            }
        }
    });

    // Устанавливаем Network Mode
    commands.insert_resource(NetworkMode::Host);

    Ok(join_code)
}

/// Подключается к QUIC серверу (Client mode)
pub fn start_quic_client(commands: &mut Commands, server_addr: SocketAddr) -> Result<(), String> {
    // Создаем ClientChannels в Bevy контексте
    let channels = Arc::new(ClientChannels::new());
    let channels_clone = channels.clone();

    // Вставляем channels как Bevy Resource
    commands.insert_resource(ClientChannelsResource(channels));

    // Создаем tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let addr_str = server_addr.to_string();

    // Подключаемся к серверу в фоновом потоке
    rt.spawn(async move {
        match QuicClient::connect(&addr_str, channels_clone).await {
            Ok(client) => {
                println!("[QUIC Client] Connected to {}", addr_str);
                let client = Arc::new(client);

                // Запускаем IO loop
                // FIXME: Нужно получить outgoing_rx из channels
                // Но мы уже передали channels в Bevy как Resource
                // Решение: разделить rx на два Arc или использовать другой подход

                println!("[QUIC Client] IO loop placeholder - waiting for full implementation");

                // Держим runtime живым
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            }
            Err(e) => {
                eprintln!("[QUIC Client] Failed to connect: {}", e);
            }
        }
    });

    // Устанавливаем Network Mode
    commands.insert_resource(NetworkMode::Client);

    Ok(())
}

/// Проверяет, нужно ли использовать legacy networking
/// Возвращает true если QUIC еще не готов для production
pub fn use_legacy_networking() -> bool {
    // FIXME: Пока true, так как системы server_read_incoming/etc еще не реализованы
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_legacy_flag() {
        assert!(use_legacy_networking());
    }
}
