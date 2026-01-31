use serde::{Deserialize, Serialize};

use super::delta::WorldDelta;
use super::snapshots::Snapshot;

/// Версия протокола - увеличивается при breaking changes
pub const PROTOCOL_VERSION: u16 = 1;

/// Максимальный размер сообщения (64 KB)
pub const MAX_MESSAGE_SIZE: usize = 65536;

// ===== CLIENT → SERVER =====

/// Сообщения от клиента к серверу
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum C2S {
    /// Первое сообщение при подключении
    Hello {
        build: u32,
        protocol: u16,
    },
    /// Запрос на присоединение к игре
    Join {
        name: String,
    },
    /// Ввод игрока (отправляется каждый кадр ~60 Hz)
    Input(InputCmd),
    /// Подтверждение получения server tick
    Ack {
        last_server_tick: u32,
    },
    /// Ping для измерения RTT
    Ping {
        timestamp_ms: u64,
    },
}

/// Команда ввода от клиента
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct InputCmd {
    /// Последовательный номер для дедупликации
    pub client_seq: u32,
    /// Текущий tick клиента
    pub client_tick: u32,
    /// Движение X: -100..100 (нормализовано)
    pub move_x: i8,
    /// Движение Y: -100..100 (нормализовано)
    pub move_y: i8,
    /// Прицеливание X: -100..100
    pub aim_x: i8,
    /// Прицеливание Y: -100..100
    pub aim_y: i8,
    /// Битовые флаги для кнопок
    pub buttons: u16,
}

impl InputCmd {
    /// Кнопка pushback (пробел)
    pub const BTN_PUSHBACK: u16 = 1 << 0;
    /// Кнопка взаимодействия (E)
    pub const BTN_INTERACT: u16 = 1 << 1;
    /// Навык 1 (Q)
    pub const BTN_SKILL1: u16 = 1 << 2;
    /// Навык 2 (R)
    pub const BTN_SKILL2: u16 = 1 << 3;
    /// Навык 3 (F)
    pub const BTN_SKILL3: u16 = 1 << 4;
    /// Навык 4 (X)
    pub const BTN_SKILL4: u16 = 1 << 5;

    /// Проверяет, нажата ли кнопка
    pub fn is_pressed(&self, button: u16) -> bool {
        (self.buttons & button) != 0
    }

    /// Устанавливает кнопку
    pub fn set_button(&mut self, button: u16, pressed: bool) {
        if pressed {
            self.buttons |= button;
        } else {
            self.buttons &= !button;
        }
    }
}

// ===== SERVER → CLIENT =====

/// Сообщения от сервера к клиенту
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum S2C {
    /// Подтверждение Hello
    HelloOk {
        protocol: u16,
        tick_hz: u16,
    },
    /// Подтверждение Join с начальным состоянием
    JoinOk {
        player_id: u8,
        world_seed: u64,
        start_tick: u32,
        snapshot: Snapshot,
    },
    /// Delta обновление мира (отправляется каждый tick ~30 Hz)
    Delta {
        server_tick: u32,
        base_tick: u32,
        delta: WorldDelta,
    },
    /// Событие (надежная доставка)
    Event {
        server_tick: u32,
        event_id: u64,
        kind: EventKind,
    },
    /// Pong ответ на Ping
    Pong {
        timestamp_ms: u64,
    },
    /// Kick игрока с сервера
    Kick {
        reason: String,
    },
}

// ===== События =====

/// Типы событий для надежной доставки
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventKind {
    /// Визуальные эффекты (переиспользуем из legacy)
    FxEvent(FxEventData),
    /// Обновление инвентаря
    InventoryUpdate {
        player_id: u8,
        items: Vec<ItemUpdate>,
    },
    /// Повышение уровня
    LevelUp {
        player_id: u8,
        new_level: u32,
    },
    /// Подбор предмета
    ItemPickup {
        player_id: u8,
        item_id: u32,
    },
}

/// Обновление предмета в инвентаре
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemUpdate {
    pub item_id: u32,
    pub stack_count: u32,
}

/// FX события (временно своя структура, потом объединим с legacy::NetFxEvent)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FxEventData {
    Hit {
        target_id: u32,
        damage: f32,
    },
    HitParticles {
        pos: [f32; 2],
        color: [f32; 4],
        count: u32,
    },
    SlimeHeal {
        pos: [f32; 2],
    },
    AreaDamage {
        pos: [f32; 2],
        radius: f32,
        dir: [f32; 2],
        cone_angle: f32,
    },
    DeathAnimation {
        target_id: u32,
        duration: f32,
    },
    ScreenShake {
        player_id: u32,
        intensity: f32,
        duration: f32,
    },
    PlayerDamageFlash {
        player_id: u32,
        intensity: f32,
        duration: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_cmd_buttons() {
        let mut input = InputCmd::default();

        assert!(!input.is_pressed(InputCmd::BTN_PUSHBACK));

        input.set_button(InputCmd::BTN_PUSHBACK, true);
        assert!(input.is_pressed(InputCmd::BTN_PUSHBACK));

        input.set_button(InputCmd::BTN_SKILL1, true);
        assert!(input.is_pressed(InputCmd::BTN_PUSHBACK));
        assert!(input.is_pressed(InputCmd::BTN_SKILL1));

        input.set_button(InputCmd::BTN_PUSHBACK, false);
        assert!(!input.is_pressed(InputCmd::BTN_PUSHBACK));
        assert!(input.is_pressed(InputCmd::BTN_SKILL1));
    }

    #[test]
    fn test_c2s_serialization() {
        let msg = C2S::Hello {
            build: 12345,
            protocol: PROTOCOL_VERSION,
        };

        let serialized = postcard::to_allocvec(&msg).unwrap();
        let deserialized: C2S = postcard::from_bytes(&serialized).unwrap();

        match deserialized {
            C2S::Hello { build, protocol } => {
                assert_eq!(build, 12345);
                assert_eq!(protocol, PROTOCOL_VERSION);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_s2c_serialization() {
        let msg = S2C::HelloOk {
            protocol: PROTOCOL_VERSION,
            tick_hz: 30,
        };

        let serialized = postcard::to_allocvec(&msg).unwrap();
        let deserialized: S2C = postcard::from_bytes(&serialized).unwrap();

        match deserialized {
            S2C::HelloOk { protocol, tick_hz } => {
                assert_eq!(protocol, PROTOCOL_VERSION);
                assert_eq!(tick_hz, 30);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_input_cmd_serialization() {
        let mut input = InputCmd {
            client_seq: 100,
            client_tick: 50,
            move_x: 75,
            move_y: -50,
            aim_x: 10,
            aim_y: 20,
            buttons: 0,
        };

        input.set_button(InputCmd::BTN_PUSHBACK, true);

        let serialized = postcard::to_allocvec(&input).unwrap();
        let deserialized: InputCmd = postcard::from_bytes(&serialized).unwrap();

        assert_eq!(deserialized.client_seq, 100);
        assert_eq!(deserialized.move_x, 75);
        assert!(deserialized.is_pressed(InputCmd::BTN_PUSHBACK));
    }

    #[test]
    fn test_message_size_reasonable() {
        // Проверяем, что базовые сообщения не слишком большие
        let input = InputCmd {
            client_seq: 1000,
            client_tick: 500,
            move_x: 100,
            move_y: -100,
            aim_x: 0,
            aim_y: 0,
            buttons: 0xFF,
        };

        let serialized = postcard::to_allocvec(&C2S::Input(input)).unwrap();
        assert!(serialized.len() < 50, "Input message too large: {} bytes", serialized.len());
    }
}
