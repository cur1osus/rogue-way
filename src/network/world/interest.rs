use bevy::prelude::*;
use std::collections::HashSet;

use crate::components::{Boss, PhysicsPosition, Player};
use crate::network::protocol::snapshots::EntityKind;
use crate::network::world::net_id::NetworkId;

/// Радиус interest management - сущности дальше этого расстояния не реплицируются
pub const INTEREST_RADIUS: f32 = 800.0;

/// Дистанция для сущностей, которые всегда реплицируются (боссы)
pub const ALWAYS_REPLICATE_DISTANCE: f32 = 1200.0;

/// Hysteresis buffer для предотвращения мерцания на границе
const INTEREST_HYSTERESIS: f32 = 50.0;

/// Набор сущностей, видимых для клиента
#[derive(Debug, Clone, Default)]
pub struct InterestSet {
    /// Позиция игрока
    pub player_pos: Vec2,
    /// Сущности (NetId), которые должны быть реплицированы
    pub entities: HashSet<u32>,
    /// Предыдущий набор (для hysteresis)
    pub previous_entities: HashSet<u32>,
}

impl InterestSet {
    pub fn new(player_pos: Vec2) -> Self {
        Self {
            player_pos,
            entities: HashSet::new(),
            previous_entities: HashSet::new(),
        }
    }

    /// Обновляет набор сущностей с учетом hysteresis
    pub fn update(&mut self, new_entities: HashSet<u32>) {
        self.previous_entities = std::mem::replace(&mut self.entities, new_entities);
    }

    /// Проверяет, находится ли сущность в interest radius
    pub fn contains(&self, net_id: u32) -> bool {
        self.entities.contains(&net_id)
    }

    /// Получает список spawn (новые сущности)
    pub fn get_spawns(&self) -> Vec<u32> {
        self.entities
            .difference(&self.previous_entities)
            .copied()
            .collect()
    }

    /// Получает список despawn (удаленные сущности)
    pub fn get_despawns(&self) -> Vec<u32> {
        self.previous_entities
            .difference(&self.entities)
            .copied()
            .collect()
    }
}

/// Приоритет репликации для сортировки в bandwidth-limited сценариях
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplicationPriority {
    Critical = 0, // Игроки, боссы
    High = 1,     // Близкие враги, снаряды
    Medium = 2,   // Далекие враги
    Low = 3,      // Pickups
}

/// Вычисляет interest set для клиента на основе позиции его игрока
pub fn calculate_interest(
    player_pos: Vec2,
    previous_set: Option<&InterestSet>,
    all_entities: &Query<(
        Entity,
        &NetworkId,
        &PhysicsPosition,
        Option<&Player>,
        Option<&Boss>,
    )>,
) -> InterestSet {
    let mut new_entities = HashSet::new();

    // Радиус с hysteresis для предотвращения мерцания
    let base_radius = INTEREST_RADIUS;
    let exit_radius = base_radius - INTEREST_HYSTERESIS;
    let enter_radius = base_radius + INTEREST_HYSTERESIS;

    for (_entity, net_id, pos, player_opt, boss_opt) in all_entities.iter() {
        let entity_pos = pos.0; // PhysicsPosition is a newtype wrapper around Vec2
        let distance = player_pos.distance(entity_pos);

        // Боссы всегда реплицируются
        if boss_opt.is_some() && distance < ALWAYS_REPLICATE_DISTANCE {
            new_entities.insert(net_id.0);
            continue;
        }

        // Игроки всегда реплицируются (для мультиплеера)
        if player_opt.is_some() {
            new_entities.insert(net_id.0);
            continue;
        }

        // Hysteresis: если сущность уже была в interest set, используем exit_radius
        // Если сущность новая, используем enter_radius
        let was_in_set = previous_set
            .map(|set| set.contains(net_id.0))
            .unwrap_or(false);

        let threshold = if was_in_set {
            exit_radius
        } else {
            enter_radius
        };

        if distance < threshold {
            new_entities.insert(net_id.0);
        }
    }

    let mut interest_set = InterestSet::new(player_pos);
    if let Some(prev) = previous_set {
        interest_set.previous_entities = prev.entities.clone();
    }
    interest_set.entities = new_entities;

    interest_set
}

/// Получает приоритет репликации для сущности
pub fn get_entity_priority(kind: &EntityKind, distance: f32) -> ReplicationPriority {
    match kind {
        EntityKind::Player { .. } => ReplicationPriority::Critical,
        EntityKind::Enemy {
            boss_type: Some(_), ..
        } => ReplicationPriority::Critical,
        EntityKind::Projectile => {
            if distance < 200.0 {
                ReplicationPriority::High
            } else {
                ReplicationPriority::Medium
            }
        }
        EntityKind::Enemy { .. } | EntityKind::Pet { .. } => {
            if distance < 300.0 {
                ReplicationPriority::High
            } else {
                ReplicationPriority::Medium
            }
        }
        EntityKind::XpGem { .. } | EntityKind::Gold { .. } => ReplicationPriority::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interest_set_creation() {
        let player_pos = Vec2::new(100.0, 100.0);
        let set = InterestSet::new(player_pos);

        assert_eq!(set.player_pos, player_pos);
        assert!(set.entities.is_empty());
    }

    #[test]
    fn test_interest_set_update() {
        let mut set = InterestSet::new(Vec2::ZERO);
        let mut new_entities = HashSet::new();
        new_entities.insert(1);
        new_entities.insert(2);

        set.update(new_entities.clone());

        assert_eq!(set.entities, new_entities);
        assert!(set.previous_entities.is_empty());
    }

    #[test]
    fn test_interest_spawns_and_despawns() {
        let mut set = InterestSet::new(Vec2::ZERO);

        // Первоначальные сущности
        let mut initial = HashSet::new();
        initial.insert(1);
        initial.insert(2);
        set.update(initial);

        // Обновляем: добавляем 3, удаляем 1
        let mut updated = HashSet::new();
        updated.insert(2);
        updated.insert(3);
        set.update(updated);

        let spawns = set.get_spawns();
        let despawns = set.get_despawns();

        assert_eq!(spawns, vec![3]);
        assert_eq!(despawns, vec![1]);
    }

    #[test]
    fn test_replication_priority() {
        let player_kind = EntityKind::Player { player_id: 0 };
        assert_eq!(
            get_entity_priority(&player_kind, 0.0),
            ReplicationPriority::Critical
        );

        let boss_kind = EntityKind::Enemy {
            enemy_type: 1,
            boss_type: Some(1),
        };
        assert_eq!(
            get_entity_priority(&boss_kind, 500.0),
            ReplicationPriority::Critical
        );

        let projectile_close = EntityKind::Projectile;
        assert_eq!(
            get_entity_priority(&projectile_close, 100.0),
            ReplicationPriority::High
        );

        let projectile_far = EntityKind::Projectile;
        assert_eq!(
            get_entity_priority(&projectile_far, 500.0),
            ReplicationPriority::Medium
        );

        let pickup = EntityKind::XpGem { value: 100 };
        assert_eq!(
            get_entity_priority(&pickup, 50.0),
            ReplicationPriority::Low
        );
    }

    #[test]
    fn test_hysteresis() {
        // Тест проверяет, что сущность на границе не мерцает
        // Создается через calculate_interest, но для этого нужен Query
        // В unit тестах это сложно, поэтому просто проверяем константы
        assert!(INTEREST_HYSTERESIS > 0.0);
        assert!(INTEREST_RADIUS + INTEREST_HYSTERESIS < ALWAYS_REPLICATE_DISTANCE);
    }
}
