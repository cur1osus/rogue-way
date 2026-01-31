/// Helper функции для интеграции QUIC networking с Bevy UI
/// Упрощают запуск клиента и сервера из main menu

use bevy::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;

use super::protocol::messages::{S2C, C2S};
use super::transport::{
    ClientChannels, ClientChannelsResource, QuicClient, QuicServer, ServerChannels,
    ServerChannelsResource,
};
use super::NetworkMode;
use tokio::sync::mpsc;

/// Запускает QUIC сервер (Host mode)
/// Возвращает join code (адрес сервера) для отображения в UI
pub fn start_quic_host(commands: &mut Commands, port: u16) -> Result<String, String> {
    // Создаем ServerChannels в Bevy контексте
    let mut channels = ServerChannels::new();

    // Извлекаем outgoing_rx для QUIC dispatcher ДО создания Arc
    let outgoing_rx = channels
        .outgoing_rx
        .take()
        .expect("outgoing_rx already taken");

    // Теперь создаем Arc и вставляем как Resource
    let channels = Arc::new(channels);
    commands.insert_resource(ServerChannelsResource(channels.clone()));

    // Создаем tokio runtime для async операций
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let addr = format!("0.0.0.0:{}", port);
    let join_code = format!("127.0.0.1:{}", port); // Для локального подключения
    let join_code_clone = join_code.clone(); // Клонируем для closure

    // Запускаем QUIC сервер в фоновом потоке
    rt.spawn(async move {
        match QuicServer::bind(&addr, channels).await {
            Ok(server) => {
                println!("[QUIC Host] Server started on {}", addr);
                println!("[QUIC Host] Join code: {}", join_code_clone);
                let server = Arc::new(server);

                // Запускаем accept loop и outgoing dispatcher
                spawn_server_outgoing_dispatcher(server.clone(), outgoing_rx);
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
    let mut channels = ClientChannels::new();

    // Извлекаем outgoing_rx для QUIC writer ДО создания Arc
    let outgoing_rx = channels
        .outgoing_rx
        .take()
        .expect("outgoing_rx already taken");

    // Теперь создаем Arc и вставляем как Resource
    let channels = Arc::new(channels);
    commands.insert_resource(ClientChannelsResource(channels.clone()));

    // Создаем tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let addr_str = server_addr.to_string();

    // Подключаемся к серверу в фоновом потоке
    rt.spawn(async move {
        match QuicClient::connect(&addr_str, channels).await {
            Ok(client) => {
                println!("[QUIC Client] Connected to {}", addr_str);
                let client = Arc::new(client);

                // Запускаем IO loops для чтения и записи
                spawn_client_io_loops(client, outgoing_rx);

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
    // QUIC networking готов к использованию
    false
}

/// Запускает dispatcher для отправки сообщений клиентам
fn spawn_server_outgoing_dispatcher(
    server: Arc<QuicServer>,
    mut outgoing_rx: mpsc::UnboundedReceiver<(u64, S2C)>,
) {
    tokio::spawn(async move {
        while let Some((conn_id, msg)) = outgoing_rx.recv().await {
            // Находим sender для этого соединения
            if let Some(sender) = server.connection_senders.get(&conn_id) {
                if let Err(_e) = sender.value().send(msg) {
                    eprintln!("[Server Dispatcher] Failed to send to {}", conn_id);
                }
            } else {
                eprintln!("[Server Dispatcher] Unknown connection: {}", conn_id);
            }
        }
        println!("[Server Dispatcher] Outgoing dispatcher closed");
    });
}

/// Запускает IO loops для клиента (чтение и запись)
fn spawn_client_io_loops(
    client: Arc<QuicClient>,
    mut outgoing_rx: mpsc::UnboundedReceiver<C2S>,
) {
    let connection = client.connection.clone();
    let incoming_tx = client.channels.incoming_tx.clone();

    // Reader task: читает сообщения от сервера
    let connection_reader = connection.clone();
    tokio::spawn(async move {
        let connection = connection_reader;
        loop {
            match connection.accept_bi().await {
                Ok((mut send, mut recv)) => {
                    let incoming_tx = incoming_tx.clone();

                    // Spawn отдельный task для этого stream
                    tokio::spawn(async move {
                        loop {
                            match super::transport::quic_client::read_message(&mut recv).await {
                                Ok(msg) => {
                                    let _ = incoming_tx.send(msg);
                                }
                                Err(e) => {
                                    eprintln!("[Client Reader] Error reading message: {}", e);
                                    break;
                                }
                            }
                        }
                    });

                    // NOTE: send половина stream не используется в этой архитектуре
                    drop(send);
                }
                Err(e) => {
                    eprintln!("[Client Reader] Connection closed: {}", e);
                    break;
                }
            }
        }
        println!("[Client Reader] Stopped");
    });

    // Writer task: отправляет сообщения серверу
    tokio::spawn(async move {
        // Открываем би-directional stream для отправки
        match connection.open_bi().await {
            Ok((mut send, _recv)) => {
                while let Some(msg) = outgoing_rx.recv().await {
                    if let Err(e) =
                        super::transport::quic_client::write_message(&mut send, &msg).await
                    {
                        eprintln!("[Client Writer] Error writing message: {}", e);
                        break;
                    }
                }
                let _ = send.finish();
            }
            Err(e) => {
                eprintln!("[Client Writer] Failed to open stream: {}", e);
            }
        }
        println!("[Client Writer] Stopped");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_legacy_flag() {
        // QUIC networking готов к использованию
        assert!(!use_legacy_networking());
    }
}
