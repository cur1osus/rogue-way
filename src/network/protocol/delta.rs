use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::quantization::Vec2i16;
use super::snapshots::{EntityKind, NetId, SpawnNet};

/// Delta обновление мира - содержит только изменения с последнего snapshot
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WorldDelta {
    /// Новые сущности, которые нужно создать
    pub spawns: Vec<SpawnNet>,
    /// ID сущностей, которые нужно удалить
    pub despawns: Vec<NetId>,
    /// Обновления существующих сущностей
    pub updates: Vec<UpdateNet>,
}

impl WorldDelta {
    pub fn new() -> Self {
        Self::default()
    }

    /// Проверяет, пустая ли дельта
    pub fn is_empty(&self) -> bool {
        self.spawns.is_empty() && self.despawns.is_empty() && self.updates.is_empty()
    }

    /// Возвращает общее количество изменений
    pub fn total_changes(&self) -> usize {
        self.spawns.len() + self.despawns.len() + self.updates.len()
    }

    /// Добавляет spawn
    pub fn add_spawn(&mut self, spawn: SpawnNet) {
        self.spawns.push(spawn);
    }

    /// Добавляет despawn
    pub fn add_despawn(&mut self, net_id: NetId) {
        self.despawns.push(net_id);
    }

    /// Добавляет update
    pub fn add_update(&mut self, update: UpdateNet) {
        self.updates.push(update);
    }
}

/// Обновление отдельной сущности
/// Использует битовые флаги для отправки только измененных полей
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateNet {
    pub id: NetId,
    /// Битовая маска показывающая какие поля присутствуют
    pub flags: u16,
    /// Позиция (если FLAG_POS установлен)
    pub pos: Option<Vec2i16>,
    /// Скорость (если FLAG_VEL установлен)
    pub vel: Option<Vec2i16>,
    /// HP (если FLAG_HP установлен)
    pub hp: Option<u16>,
}

impl UpdateNet {
    /// Флаг: позиция изменилась
    pub const FLAG_POS: u16 = 1 << 0;
    /// Флаг: скорость изменилась
    pub const FLAG_VEL: u16 = 1 << 1;
    /// Флаг: HP изменилось
    pub const FLAG_HP: u16 = 1 << 2;

    /// Создает новое пустое обновление
    pub fn new(id: NetId) -> Self {
        Self {
            id,
            flags: 0,
            pos: None,
            vel: None,
            hp: None,
        }
    }

    /// Builder: устанавливает позицию
    pub fn with_pos(mut self, pos: Vec2i16) -> Self {
        self.pos = Some(pos);
        self.flags |= Self::FLAG_POS;
        self
    }

    /// Builder: устанавливает скорость
    pub fn with_vel(mut self, vel: Vec2i16) -> Self {
        self.vel = Some(vel);
        self.flags |= Self::FLAG_VEL;
        self
    }

    /// Builder: устанавливает HP
    pub fn with_hp(mut self, hp: u16) -> Self {
        self.hp = Some(hp);
        self.flags |= Self::FLAG_HP;
        self
    }

    /// Проверяет, содержит ли обновление какие-либо данные
    pub fn has_updates(&self) -> bool {
        self.flags != 0
    }

    /// Проверяет, установлен ли флаг позиции
    pub fn has_pos(&self) -> bool {
        (self.flags & Self::FLAG_POS) != 0
    }

    /// Проверяет, установлен ли флаг скорости
    pub fn has_vel(&self) -> bool {
        (self.flags & Self::FLAG_VEL) != 0
    }

    /// Проверяет, установлен ли флаг HP
    pub fn has_hp(&self) -> bool {
        (self.flags & Self::FLAG_HP) != 0
    }
}

/// Builder для создания WorldDelta
pub struct WorldDeltaBuilder {
    delta: WorldDelta,
}

impl WorldDeltaBuilder {
    pub fn new() -> Self {
        Self {
            delta: WorldDelta::new(),
        }
    }

    pub fn spawn(mut self, spawn: SpawnNet) -> Self {
        self.delta.add_spawn(spawn);
        self
    }

    pub fn despawn(mut self, net_id: NetId) -> Self {
        self.delta.add_despawn(net_id);
        self
    }

    pub fn update(mut self, update: UpdateNet) -> Self {
        if update.has_updates() {
            self.delta.add_update(update);
        }
        self
    }

    pub fn build(self) -> WorldDelta {
        self.delta
    }
}

impl Default for WorldDeltaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_net_flags() {
        let update = UpdateNet::new(123)
            .with_pos(Vec2i16::new(100, 200))
            .with_hp(50);

        assert!(update.has_pos());
        assert!(!update.has_vel());
        assert!(update.has_hp());
        assert!(update.has_updates());
        assert_eq!(update.id, 123);
        assert_eq!(update.pos.unwrap().x, 100);
        assert_eq!(update.hp.unwrap(), 50);
    }

    #[test]
    fn test_empty_update() {
        let update = UpdateNet::new(123);
        assert!(!update.has_updates());
    }

    #[test]
    fn test_world_delta_builder() {
        let delta = WorldDeltaBuilder::new()
            .spawn(SpawnNet {
                id: 1,
                kind: EntityKind::Projectile,
                pos: Vec2i16::ZERO,
                vel: Vec2i16::ZERO,
                hp: None,
                hp_max: None,
            })
            .update(UpdateNet::new(2).with_pos(Vec2i16::new(10, 20)))
            .despawn(3)
            .build();

        assert_eq!(delta.spawns.len(), 1);
        assert_eq!(delta.updates.len(), 1);
        assert_eq!(delta.despawns.len(), 1);
        assert!(!delta.is_empty());
        assert_eq!(delta.total_changes(), 3);
    }

    #[test]
    fn test_delta_serialization() {
        let delta = WorldDelta {
            spawns: vec![],
            despawns: vec![1, 2, 3],
            updates: vec![UpdateNet::new(100).with_pos(Vec2i16::new(50, 75))],
        };

        let serialized = postcard::to_allocvec(&delta).unwrap();
        let deserialized: WorldDelta = postcard::from_bytes(&serialized).unwrap();

        assert_eq!(deserialized.despawns.len(), 3);
        assert_eq!(deserialized.updates.len(), 1);
        assert_eq!(deserialized.updates[0].id, 100);
    }

    #[test]
    fn test_delta_size_efficiency() {
        // Пустая дельта должна быть очень маленькой
        let empty_delta = WorldDelta::new();
        let serialized = postcard::to_allocvec(&empty_delta).unwrap();
        assert!(
            serialized.len() < 10,
            "Empty delta too large: {} bytes",
            serialized.len()
        );

        // Один despawn должен быть крошечным
        let delta_with_despawn = WorldDelta {
            spawns: vec![],
            despawns: vec![123],
            updates: vec![],
        };
        let serialized = postcard::to_allocvec(&delta_with_despawn).unwrap();
        assert!(
            serialized.len() < 20,
            "Despawn delta too large: {} bytes",
            serialized.len()
        );

        // Обновление только позиции должно быть компактным
        let delta_with_pos_update = WorldDelta {
            spawns: vec![],
            despawns: vec![],
            updates: vec![UpdateNet::new(456).with_pos(Vec2i16::new(100, 200))],
        };
        let serialized = postcard::to_allocvec(&delta_with_pos_update).unwrap();
        assert!(
            serialized.len() < 30,
            "Position update too large: {} bytes",
            serialized.len()
        );
    }
}
