// Game Server binary - headless dedicated server
// Минимальная версия для демонстрации QUIC сервера

use bevy::app::{App, ScheduleRunnerPlugin};
use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use roggy::network::sync::server::NetServerPlugin;
use roggy::network::transport::{QuicServer, ServerChannels, ServerChannelsResource};
use roggy::network::NetworkMode;

/// Адрес и порт для QUIC сервера
const SERVER_ADDR: &str = "0.0.0.0:25565";

/// Tick rate для fixed timestep (60 Hz)
const TICK_RATE: f64 = 1.0 / 60.0;

fn main() {
    println!("=== Roggy Game Server ===");
    println!("Starting headless dedicated server on {}", SERVER_ADDR);

    let mut app = App::new();

    // Минимальные плагины + ScheduleRunner для headless режима
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f64(TICK_RATE),
    )));

    // Создаем ServerChannels в Bevy контексте
    let channels = Arc::new(ServerChannels::new());
    let channels_clone = channels.clone();

    // Вставляем channels как Resource для networking систем
    app.insert_resource(ServerChannelsResource(channels));

    // Network mode: Host (authoritative server)
    app.insert_resource(NetworkMode::Host);

    // Добавляем серверный networking plugin
    app.add_plugins(NetServerPlugin);

    // TODO: Добавить геймплей системы после интеграции

    // Запускаем QUIC сервер в фоновом потоке
    println!("Binding QUIC server to {}...", SERVER_ADDR);

    // Создаем tokio runtime для async QUIC сервера
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.spawn(async move {
        match QuicServer::bind(SERVER_ADDR, channels_clone).await {
            Ok(server) => {
                println!("[Server] QUIC server started successfully");
                println!("[Server] Listening on {}", SERVER_ADDR);
                println!("[Server] Ready to accept client connections");

                let server = Arc::new(server);

                // Запускаем accept loop
                server.spawn_loops();

                // Держим runtime живым
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    println!("[Server] Server tick - uptime check");
                }
            }
            Err(e) => {
                eprintln!("[Server] Failed to bind QUIC server: {}", e);
                std::process::exit(1);
            }
        }
    });

    println!("Server initialized. Starting main loop...");
    println!("Press Ctrl+C to stop the server.");
    println!();

    // Запускаем главный цикл Bevy
    app.run();
}
