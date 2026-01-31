use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::network::protocol::{C2S, S2C, MAX_MESSAGE_SIZE};
use crate::network::transport::channels::{ConnId, ServerChannels};

/// QUIC сервер для приема подключений от клиентов
pub struct QuicServer {
    pub endpoint: quinn::Endpoint,
    pub channels: Arc<ServerChannels>,
    /// Per-connection sender для outgoing messages
    pub connection_senders: Arc<DashMap<ConnId, mpsc::UnboundedSender<S2C>>>,
}

impl QuicServer {
    /// Создает и запускает QUIC сервер на указанном адресе
    ///
    /// # Arguments
    /// * `addr` - Адрес для прослушивания (например "0.0.0.0:25565")
    /// * `channels` - ServerChannels для связи с Bevy (создаются в Bevy контексте)
    pub async fn bind(
        addr: &str,
        channels: Arc<ServerChannels>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let server_config = configure_server()?;
        let endpoint = quinn::Endpoint::server(server_config, addr.parse()?)?;

        let connection_senders = Arc::new(DashMap::new());

        println!("[QuicServer] Listening on {}", addr);

        Ok(Self {
            endpoint,
            channels,
            connection_senders,
        })
    }

    /// Запускает accept loop и outgoing dispatcher
    pub fn spawn_loops(self: Arc<Self>) {
        // Accept loop
        let endpoint = self.endpoint.clone();
        let channels = self.channels.clone();
        let connection_senders = self.connection_senders.clone();

        tokio::spawn(async move {
            loop {
                match endpoint.accept().await {
                    Some(incoming) => {
                        let connecting = incoming;
                        let channels = channels.clone();
                        let connection_senders = connection_senders.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_connection(connecting, channels, connection_senders).await
                            {
                                eprintln!("[QuicServer] Connection error: {e}");
                            }
                        });
                    }
                    None => {
                        println!("[QuicServer] Endpoint closed");
                        break;
                    }
                }
            }
        });

        // Outgoing dispatcher: распределяет сообщения по connections
        let mut outgoing_rx = self
            .channels
            .outgoing_rx
            .as_ref()
            .expect("outgoing_rx already taken - call take_outgoing_rx first");

        // FIXME: Нужно вызвать take_outgoing_rx() перед spawn_loops
        // Временно используем clone через Arc<Mutex<>>

        // Упрощенная версия: dispatcher будет создан в другом месте
        // Пока просто оставим заглушку
    }
}

/// Обрабатывает одно подключение клиента
async fn handle_connection(
    connecting: quinn::Incoming,
    channels: Arc<ServerChannels>,
    connection_senders: Arc<DashMap<ConnId, mpsc::UnboundedSender<S2C>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection = connecting.await?;
    let conn_id = connection.stable_id() as ConnId;
    let remote_addr = connection.remote_address();

    println!(
        "[QuicServer] Client {} connected from {}",
        conn_id, remote_addr
    );

    // Принимаем bi-directional stream
    let (send, recv) = connection.accept_bi().await?;

    // Создаем канал для outgoing messages этого connection
    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel::<S2C>();
    connection_senders.insert(conn_id, conn_tx);

    // Spawn reader task
    let incoming_tx = channels.incoming_tx.clone();
    let disconnect_tx = channels.disconnect_tx.clone();
    let conn_id_clone = conn_id;
    let reader_handle = tokio::spawn(async move {
        if let Err(e) = read_loop(conn_id_clone, recv, incoming_tx.clone()).await {
            eprintln!("[QuicServer] Read error conn {conn_id_clone}: {e}");
        }
        // Уведомляем о disconnect
        let _ = disconnect_tx.send(conn_id_clone);
        println!("[QuicServer] Client {} reader closed", conn_id_clone);
    });

    // Spawn writer task
    let conn_id_clone = conn_id;
    let writer_handle = tokio::spawn(async move {
        if let Err(e) = write_loop(conn_id_clone, send, &mut conn_rx).await {
            eprintln!("[QuicServer] Write error conn {conn_id_clone}: {e}");
        }
        println!("[QuicServer] Client {} writer closed", conn_id_clone);
    });

    // Ждем завершения обоих tasks
    let _ = tokio::try_join!(reader_handle, writer_handle);

    // Cleanup
    connection_senders.remove(&conn_id);

    Ok(())
}

/// Reader loop: читает сообщения от клиента
async fn read_loop(
    conn_id: ConnId,
    mut recv: quinn::RecvStream,
    incoming_tx: mpsc::UnboundedSender<(ConnId, C2S)>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match read_message(&mut recv).await {
            Ok(msg) => {
                if incoming_tx.send((conn_id, msg)).is_err() {
                    break; // Channel closed
                }
            }
            Err(e) => {
                // Проверяем graceful close
                if e.to_string().contains("connection closed")
                    || e.to_string().contains("stream closed")
                {
                    break;
                }
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Writer loop: отправляет сообщения клиенту
async fn write_loop(
    _conn_id: ConnId,
    mut send: quinn::SendStream,
    conn_rx: &mut mpsc::UnboundedReceiver<S2C>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    while let Some(msg) = conn_rx.recv().await {
        write_message(&mut send, &msg).await?;
    }
    Ok(())
}

/// Читает одно сообщение с length-prefix framing
async fn read_message(
    recv: &mut quinn::RecvStream,
) -> Result<C2S, Box<dyn std::error::Error + Send + Sync>> {
    // Читаем длину (4 bytes)
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        return Err(format!("Message too large: {} bytes", len).into());
    }

    // Читаем payload
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await?;

    // Десериализуем с postcard
    let msg: C2S = postcard::from_bytes(&buf)?;
    Ok(msg)
}

/// Пишет одно сообщение с length-prefix framing
async fn write_message(
    send: &mut quinn::SendStream,
    msg: &S2C,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Сериализуем с postcard
    let payload = postcard::to_allocvec(msg)?;
    let len = payload.len() as u32;

    if len as usize > MAX_MESSAGE_SIZE {
        return Err(format!("Message too large: {} bytes", len).into());
    }

    // Пишем длину
    send.write_all(&len.to_le_bytes()).await?;
    // Пишем payload
    send.write_all(&payload).await?;

    Ok(())
}

/// Конфигурирует QUIC сервер с self-signed сертификатом
fn configure_server() -> Result<quinn::ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
    // Генерируем self-signed сертификат
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert.cert.der().to_vec();
    let priv_key_der = cert.key_pair.serialize_der();

    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let priv_key = rustls::pki_types::PrivateKeyDer::try_from(priv_key_der)?;

    let mut server_config = quinn::ServerConfig::with_single_cert(cert_chain, priv_key)?;

    // Настройки транспорта
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());
    transport_config.max_concurrent_bidi_streams(1_u8.into());

    Ok(server_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let channels = Arc::new(ServerChannels::new());
        let result = QuicServer::bind("127.0.0.1:0", channels).await;
        assert!(result.is_ok());

        let server = result.unwrap();
        assert!(server.endpoint.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_message_framing() {
        let msg = S2C::HelloOk {
            protocol: 1,
            tick_hz: 30,
        };

        let payload = postcard::to_allocvec(&msg).unwrap();
        let len = payload.len() as u32;

        assert!(len < MAX_MESSAGE_SIZE as u32);

        let deserialized: S2C = postcard::from_bytes(&payload).unwrap();
        match deserialized {
            S2C::HelloOk { protocol, tick_hz } => {
                assert_eq!(protocol, 1);
                assert_eq!(tick_hz, 30);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
