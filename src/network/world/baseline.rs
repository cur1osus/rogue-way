use std::collections::HashMap;

use crate::network::protocol::quantization::Vec2i16;
use crate::network::protocol::snapshots::NetId;

/// Baseline состояние одной сущности для вычисления дельт
#[derive(Debug, Clone)]
pub struct EntityBaseline {
    /// Network ID сущности
    pub net_id: NetId,
    /// Последняя реплицированная позиция (квантизованная)
    pub pos: Vec2i16,
    /// Последняя реплицированная скорость (квантизованная)
    pub vel: Vec2i16,
    /// Последнее реплицированное HP
    pub hp: u16,
    /// Tick, на котором было последнее обновление
    pub last_tick: u32,
}

impl EntityBaseline {
    /// Создает новый baseline для сущности
    pub fn new(net_id: NetId, pos: Vec2i16, vel: Vec2i16, hp: u16, tick: u32) -> Self {
        Self {
            net_id,
            pos,
            vel,
            hp,
            last_tick: tick,
        }
    }

    /// Проверяет, изменилось ли состояние сущности с последнего baseline
    pub fn has_changed(&self, pos: Vec2i16, vel: Vec2i16, hp: u16) -> bool {
        self.pos.x != pos.x || self.pos.y != pos.y || self.vel.x != vel.x || self.vel.y != vel.y || self.hp != hp
    }

    /// Обновляет baseline новыми значениями
    pub fn update(&mut self, pos: Vec2i16, vel: Vec2i16, hp: u16, tick: u32) {
        self.pos = pos;
        self.vel = vel;
        self.hp = hp;
        self.last_tick = tick;
    }
}

/// Baseline для одного клиента, содержит все сущности, которые он видел
#[derive(Debug, Clone)]
pub struct ClientBaseline {
    /// Player ID клиента
    pub player_id: u8,
    /// Последний tick, который клиент подтвердил (Ack)
    pub last_acked_tick: u32,
    /// Baseline для каждой сущности (NetId -> EntityBaseline)
    pub entities: HashMap<NetId, EntityBaseline>,
}

impl ClientBaseline {
    /// Создает новый ClientBaseline
    pub fn new(player_id: u8) -> Self {
        Self {
            player_id,
            last_acked_tick: 0,
            entities: HashMap::new(),
        }
    }

    /// Обновляет или создает baseline для сущности
    pub fn update_entity(&mut self, net_id: NetId, pos: Vec2i16, vel: Vec2i16, hp: u16, tick: u32) {
        if let Some(baseline) = self.entities.get_mut(&net_id) {
            baseline.update(pos, vel, hp, tick);
        } else {
            self.entities
                .insert(net_id, EntityBaseline::new(net_id, pos, vel, hp, tick));
        }
    }

    /// Проверяет, изменилась ли сущность с последнего baseline
    pub fn has_changed(&self, net_id: NetId, pos: Vec2i16, vel: Vec2i16, hp: u16) -> bool {
        match self.entities.get(&net_id) {
            Some(baseline) => baseline.has_changed(pos, vel, hp),
            None => true, // Новая сущность - всегда changed
        }
    }

    /// Удаляет baseline для сущности (когда она despawn)
    pub fn remove_entity(&mut self, net_id: NetId) -> Option<EntityBaseline> {
        self.entities.remove(&net_id)
    }

    /// Получает baseline для сущности
    pub fn get_entity(&self, net_id: NetId) -> Option<&EntityBaseline> {
        self.entities.get(&net_id)
    }

    /// Обновляет last_acked_tick
    pub fn update_ack(&mut self, tick: u32) {
        if tick > self.last_acked_tick {
            self.last_acked_tick = tick;
        }
    }

    /// Очищает старые baselines (для сущностей, которые давно не обновлялись)
    pub fn cleanup_old_entities(&mut self, current_tick: u32, max_age_ticks: u32) {
        self.entities
            .retain(|_, baseline| current_tick - baseline.last_tick <= max_age_ticks);
    }

    /// Получает количество отслеживаемых сущностей
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec2;
    use crate::network::protocol::quantization::Quantize;

    #[test]
    fn test_entity_baseline_creation() {
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();
        let baseline = EntityBaseline::new(123, pos, vel, 100, 1);

        assert_eq!(baseline.net_id, 123);
        assert_eq!(baseline.hp, 100);
        assert_eq!(baseline.last_tick, 1);
    }

    #[test]
    fn test_entity_baseline_has_changed() {
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();
        let baseline = EntityBaseline::new(123, pos, vel, 100, 1);

        // Те же значения - не изменилось
        assert!(!baseline.has_changed(pos, vel, 100));

        // Изменилась позиция
        let new_pos = Vec2::new(11.0, 20.0).to_i16();
        assert!(baseline.has_changed(new_pos, vel, 100));

        // Изменилась скорость
        let new_vel = Vec2::new(2.0, 2.0).to_i16();
        assert!(baseline.has_changed(pos, new_vel, 100));

        // Изменилось HP
        assert!(baseline.has_changed(pos, vel, 90));
    }

    #[test]
    fn test_entity_baseline_update() {
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();
        let mut baseline = EntityBaseline::new(123, pos, vel, 100, 1);

        let new_pos = Vec2::new(15.0, 25.0).to_i16();
        let new_vel = Vec2::new(2.0, 3.0).to_i16();
        baseline.update(new_pos, new_vel, 80, 2);

        assert_eq!(baseline.pos, new_pos);
        assert_eq!(baseline.vel, new_vel);
        assert_eq!(baseline.hp, 80);
        assert_eq!(baseline.last_tick, 2);
    }

    #[test]
    fn test_client_baseline_creation() {
        let baseline = ClientBaseline::new(1);

        assert_eq!(baseline.player_id, 1);
        assert_eq!(baseline.last_acked_tick, 0);
        assert_eq!(baseline.entity_count(), 0);
    }

    #[test]
    fn test_client_baseline_update_entity() {
        let mut baseline = ClientBaseline::new(1);
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();

        baseline.update_entity(123, pos, vel, 100, 1);

        assert_eq!(baseline.entity_count(), 1);
        assert!(baseline.get_entity(123).is_some());
    }

    #[test]
    fn test_client_baseline_has_changed() {
        let mut baseline = ClientBaseline::new(1);
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();

        // Новая сущность - всегда changed
        assert!(baseline.has_changed(123, pos, vel, 100));

        baseline.update_entity(123, pos, vel, 100, 1);

        // Те же значения - не changed
        assert!(!baseline.has_changed(123, pos, vel, 100));

        // Изменилась позиция
        let new_pos = Vec2::new(15.0, 25.0).to_i16();
        assert!(baseline.has_changed(123, new_pos, vel, 100));
    }

    #[test]
    fn test_client_baseline_remove_entity() {
        let mut baseline = ClientBaseline::new(1);
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();

        baseline.update_entity(123, pos, vel, 100, 1);
        assert_eq!(baseline.entity_count(), 1);

        let removed = baseline.remove_entity(123);
        assert!(removed.is_some());
        assert_eq!(baseline.entity_count(), 0);
    }

    #[test]
    fn test_client_baseline_update_ack() {
        let mut baseline = ClientBaseline::new(1);

        baseline.update_ack(10);
        assert_eq!(baseline.last_acked_tick, 10);

        baseline.update_ack(15);
        assert_eq!(baseline.last_acked_tick, 15);

        // Старый tick не обновляет
        baseline.update_ack(12);
        assert_eq!(baseline.last_acked_tick, 15);
    }

    #[test]
    fn test_client_baseline_cleanup() {
        let mut baseline = ClientBaseline::new(1);
        let pos = Vec2::new(10.0, 20.0).to_i16();
        let vel = Vec2::new(1.0, 2.0).to_i16();

        // Добавляем сущности на разных тиках
        baseline.update_entity(1, pos, vel, 100, 1);
        baseline.update_entity(2, pos, vel, 100, 5);
        baseline.update_entity(3, pos, vel, 100, 10);

        assert_eq!(baseline.entity_count(), 3);

        // Cleanup с current_tick=15 и max_age=5
        // Должна остаться только сущность 3 (tick=10, age=5)
        baseline.cleanup_old_entities(15, 5);

        assert_eq!(baseline.entity_count(), 1);
        assert!(baseline.get_entity(3).is_some());
        assert!(baseline.get_entity(1).is_none());
        assert!(baseline.get_entity(2).is_none());
    }
}
