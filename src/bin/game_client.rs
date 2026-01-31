// Game Client binary - полноценный клиент с UI и рендерингом
// Использует общий код из lib.rs

use bevy::prelude::*;
use roggy;
use roggy::network::sync::client::NetClientPlugin;

fn main() {
    println!("=== Roggy Game Client ===");

    let mut app = roggy::build_base_app();

    // TODO: Заменить legacy NetworkPlugin на новый NetClientPlugin
    // Пока используем build_base_app() как есть (с legacy networking)

    // В будущем:
    // 1. Убрать NetworkPlugin из build_base_app
    // 2. Добавить NetClientPlugin здесь
    // app.add_plugins(NetClientPlugin);

    println!("Starting client...");
    app.run();
}
