# Networking System Status

## ✅ COMPLETED (Phases 1-6)

### Phase 1: Foundation
- ✅ Cargo.toml с новыми зависимостями (quinn, postcard, tokio)
- ✅ src/lib.rs с `build_base_app()`
- ✅ Модульная структура network/

### Phase 2: Protocol (19 tests ✅)
- ✅ `quantization.rs` - Vec2i16 для компактных позиций (scale=32.0)
- ✅ `messages.rs` - C2S/S2C протокол с postcard
- ✅ `snapshots.rs` - Типы снапшотов с конверсией из legacy
- ✅ `delta.rs` - Delta replication с битовыми флагами

### Phase 3: Transport (7 tests ✅)
- ✅ `channels.rs` - mpsc каналы между tokio и Bevy
- ✅ `quic_server.rs` - QUIC сервер (quinn + DashMap)
- ✅ `quic_client.rs` - QUIC клиент с certificate skipping

### Phase 4: World (15 tests ✅)
- ✅ `net_id.rs` - Реэкспорт NetworkId с helper методами
- ✅ `interest.rs` - Interest management (radius=800.0, hysteresis)
- ✅ `baseline.rs` - Baseline tracking для delta replication

### Phase 5: Sync (12 tests ✅)
- ✅ `server.rs` - NetServerPlugin с ServerTick, ClientSessions, InputBuffers
- ✅ `client.rs` - NetClientPlugin с ClientTick, SnapshotBuffer
- ✅ `interpolation.rs` - Реэкспорт из legacy

### Phase 6: Binaries
- ✅ `bin/game_server.rs` - Headless dedicated server (MinimalPlugins + QUIC)
- ✅ `bin/game_client.rs` - Полноценный клиент с UI

**ИТОГО: 53 теста проходит! Вся инфраструктура готова.**

---

## 🚧 IN PROGRESS (Phase 7)

### UI Integration
- ✅ `quic_integration.rs` - Helper функции start_quic_host/client
- ⏳ Обновление main_menu.rs для использования QUIC
- ⏳ Debug UI для networking stats

---

## ❌ TODO (Phase 8+)

### Реализация логики networking систем

**Критические заглушки (помечены `// TODO`):**

1. **server.rs::server_read_incoming** (строка 156)
   - Нужно правильно обрабатывать `incoming_rx` из ServerChannels
   - Парсить C2S сообщения (Hello, Join, Input, Ack, Ping)
   - Обновлять ClientSessions и InputBuffers

2. **server.rs::server_send_outgoing** (строка 188)
   - Реализовать `collect_delta_for_client()` (строка 211)
   - Вычислять interest set на основе позиции игрока
   - Собирать delta updates (spawns, despawns, updates)
   - Отправлять S2C::Delta через outgoing_tx

3. **client.rs::client_read_incoming** (строка 139)
   - Обрабатывать `incoming_rx` из ClientChannels
   - Парсить S2C сообщения (HelloOk, JoinOk, Delta, Event, Pong, Kick)
   - Обновлять SnapshotBuffer и ClientTick

4. **client.rs::client_capture_input** (строка 148)
   - Считывать PlayerInputState (клавиатура/геймпад)
   - Создавать InputCmd с квантизованными значениями
   - Отправлять C2S::Input через outgoing_tx

5. **client.rs::client_apply_snapshot** (строка 156)
   - Применять снапшоты из SnapshotBuffer к ECS entities
   - Обновлять/создавать/удалять сущности на основе дельт
   - Интерполировать между снапшотами

6. **quic_integration.rs** (строки 36, 71)
   - Вставлять ServerChannels/ClientChannels как Bevy Resource
   - Правильно передавать channels между tokio и Bevy
   - Использовать Arc<Mutex<>> для thread-safety

### Интеграция с геймплеем

7. **Authoritative server системы**
   - Адаптировать player_input_system для server authority
   - Применять InputCmd к игрокам на сервере
   - Проверять легитимность клиентского input

8. **Fixed server tick**
   - Добавить server_fixed_tick систему (30 Hz)
   - Интегрировать с существующим PhysicsAccumulator

9. **FX события синхронизация**
   - Отправлять NetFxEvent через S2C::Event
   - Дедупликация на клиенте (NetworkFxDeduper)

### Тестирование

10. **Локальное тестирование**
    - Запустить game_server
    - Подключить 2-4 game_client
    - Проверить синхронизацию движения, damage, enemies

11. **Bandwidth profiling**
    - Использовать wireshark/tcpdump
    - Цель: < 50 KB/s на клиента
    - Оптимизировать delta compression

12. **Soak test**
    - 10+ минут работы
    - Проверить memory leaks (heaptrack)
    - Проверить стабильность tick rate

---

## 📊 Архитектура

```
┌─────────────────────────────────────────────────────────────┐
│                        Bevy App                              │
│  ┌────────────┐              ┌────────────┐                 │
│  │  Server    │              │  Client    │                 │
│  │  Systems   │              │  Systems   │                 │
│  │            │              │            │                 │
│  │ - read_in  │              │ - read_in  │                 │
│  │ - send_out │              │ - capture  │                 │
│  │ - fixed_tk │              │ - apply    │                 │
│  └─────┬──────┘              └──────┬─────┘                 │
│        │                            │                       │
│        │ ServerChannels             │ ClientChannels        │
│        │ (mpsc)                     │ (mpsc)                │
└────────┼────────────────────────────┼───────────────────────┘
         │                            │
         │ Bevy ↔ Tokio Bridge        │
         │                            │
┌────────┼────────────────────────────┼───────────────────────┐
│        │                            │                       │
│  ┌─────▼──────┐              ┌──────▼─────┐                │
│  │ QuicServer │              │ QuicClient │                │
│  │            │              │            │                │
│  │ - accept() │◄────QUIC────►│ - connect()│                │
│  │ - DashMap  │              │ - bi_stream│                │
│  └────────────┘              └────────────┘                │
│                    Tokio Runtime                           │
└─────────────────────────────────────────────────────────────┘
```

## 🎯 Acceptance Criteria

- [ ] Join flow: Клиент подключается, получает Snapshot, видит мир
- [ ] 4 players movement: Все видят друг друга, smooth interpolation
- [ ] Bandwidth < 50 KB/s: Interest management + delta работают
- [ ] FX синхронизированы: Damage numbers, particles корректны
- [ ] Disconnect handling: Graceful cleanup
- [ ] Debug UI: RTT, tick, entity count отображаются
- [ ] Soak test: 10 min без crashes/leaks

## 📝 Notes

- Legacy networking (TCP/UDP + bincode) все еще активен в `build_base_app()`
- Новая QUIC система сосуществует параллельно
- Переключение через `quic_integration::use_legacy_networking()`
- После завершения TODO - удалить legacy код

## 🚀 Команды

```bash
# Dedicated server (headless)
cargo run --bin game_server

# Client (с UI)
cargo run --bin game_client

# Обычный режим (legacy)
cargo run

# Тесты
cargo test --lib network::
```
