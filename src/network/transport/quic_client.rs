use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::network::protocol::{C2S, S2C, MAX_MESSAGE_SIZE};
use crate::network::transport::channels::ClientChannels;

/// QUIC клиент для подключения к серверу
pub struct QuicClient {
    pub endpoint: quinn::Endpoint,
    pub connection: quinn::Connection,
    pub channels: Arc<ClientChannels>,
}

impl QuicClient {
    /// Подключается к QUIC серверу
    ///
    /// # Arguments
    /// * `server_addr` - Адрес сервера (например "127.0.0.1:25565")
    /// * `channels` - ClientChannels для связи с Bevy (создаются в Bevy контексте)
    pub async fn connect(
        server_addr: &str,
        channels: Arc<ClientChannels>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client_config = configure_client();
        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        println!("[QuicClient] Connecting to {}...", server_addr);

        // Парсим адрес и подключаемся
        let server_addr_parsed = server_addr.parse()?;
        let connection = endpoint.connect(server_addr_parsed, "localhost")?.await?;

        println!("[QuicClient] Connected to {}", server_addr);

        Ok(Self {
            endpoint,
            connection,
            channels,
        })
    }

    /// Запускает IO tasks для чтения и записи
    pub fn spawn_io_tasks(self: Arc<Self>) {
        let connection = self.connection.clone();
        let channels = self.channels.clone();

        tokio::spawn(async move {
            // Открываем bi-directional stream
            match connection.open_bi().await {
                Ok((send, recv)) => {
                    // Spawn reader task
                    let incoming_tx = channels.incoming_tx.clone();
                    let disconnect_tx = channels.disconnect_tx.clone();
                    let reader_handle = tokio::spawn(async move {
                        if let Err(e) = read_loop(recv, incoming_tx.clone()).await {
                            eprintln!("[QuicClient] Read error: {e}");
                        }
                        // Уведомляем о disconnect
                        let _ = disconnect_tx.send(());
                        println!("[QuicClient] Reader closed");
                    });

                    // Spawn writer task (placeholder - используйте run_io_loop вместо spawn_io_tasks)
                    let writer_handle = tokio::spawn(async move {
                        // NOTE: outgoing_rx требует ownership, поэтому используйте run_io_loop
                        println!("[QuicClient] Writer task - use run_io_loop instead");
                    });

                    // Ждем завершения обоих tasks
                    let _ = tokio::try_join!(reader_handle, writer_handle);
                }
                Err(e) => {
                    eprintln!("[QuicClient] Failed to open bi-stream: {e}");
                }
            }
        });
    }

    /// Упрощенная версия с прямым доступом к каналам
    pub async fn run_io_loop(
        self: Arc<Self>,
        mut outgoing_rx: mpsc::UnboundedReceiver<C2S>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Открываем bi-directional stream
        let (send, recv) = self.connection.open_bi().await?;

        // Spawn reader
        let incoming_tx = self.channels.incoming_tx.clone();
        let disconnect_tx = self.channels.disconnect_tx.clone();
        let reader_handle = tokio::spawn(async move {
            if let Err(e) = read_loop(recv, incoming_tx.clone()).await {
                eprintln!("[QuicClient] Read error: {e}");
            }
            let _ = disconnect_tx.send(());
        });

        // Spawn writer
        let writer_handle = tokio::spawn(async move {
            if let Err(e) = write_loop(send, &mut outgoing_rx).await {
                eprintln!("[QuicClient] Write error: {e}");
            }
        });

        // Ждем завершения
        let _ = tokio::try_join!(reader_handle, writer_handle);

        Ok(())
    }
}

/// Reader loop: читает сообщения от сервера
async fn read_loop(
    mut recv: quinn::RecvStream,
    incoming_tx: mpsc::UnboundedSender<S2C>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match read_message(&mut recv).await {
            Ok(msg) => {
                if incoming_tx.send(msg).is_err() {
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

/// Writer loop: отправляет сообщения серверу
async fn write_loop(
    mut send: quinn::SendStream,
    outgoing_rx: &mut mpsc::UnboundedReceiver<C2S>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    while let Some(msg) = outgoing_rx.recv().await {
        write_message(&mut send, &msg).await?;
    }
    Ok(())
}

/// Читает одно сообщение с length-prefix framing
pub async fn read_message(
    recv: &mut quinn::RecvStream,
) -> Result<S2C, Box<dyn std::error::Error + Send + Sync>> {
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
    let msg: S2C = postcard::from_bytes(&buf)?;
    Ok(msg)
}

/// Пишет одно сообщение с length-prefix framing
pub async fn write_message(
    send: &mut quinn::SendStream,
    msg: &C2S,
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

/// Конфигурирует QUIC клиент (skip cert verification для self-signed)
fn configure_client() -> quinn::ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto).unwrap(),
    ))
}

/// Skip server certificate verification (для self-signed сертификатов)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer,
        _intermediates: &[rustls::pki_types::CertificateDer],
        _server_name: &rustls::pki_types::ServerName,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // Принимаем любой сертификат (НЕБЕЗОПАСНО для production!)
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_serialization() {
        let msg = C2S::Hello {
            build: 123,
            protocol: 1,
        };

        let payload = postcard::to_allocvec(&msg).unwrap();
        let len = payload.len() as u32;

        assert!(len < MAX_MESSAGE_SIZE as u32);

        let deserialized: C2S = postcard::from_bytes(&payload).unwrap();
        match deserialized {
            C2S::Hello { build, protocol } => {
                assert_eq!(build, 123);
                assert_eq!(protocol, 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    // Note: Полноценный integration test с сервером требует запущенного сервера
    // Для этого нужно создать отдельный integration test в tests/
}
