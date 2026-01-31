use bevy::prelude::*;

/// Реэкспорт существующих типов NetworkId из legacy модуля
/// Эти типы уже хорошо работают, поэтому переиспользуем их
pub use crate::network::legacy::{NetworkEntityMap, NetworkId, NetworkIdAllocator};

/// Helper методы для удобной работы с NetworkEntityMap
pub trait NetworkEntityMapExt {
    /// Получить Entity по NetworkId
    fn get_entity(&self, net_id: u32) -> Option<Entity>;

    /// Получить NetworkId по Entity
    fn get_net_id(&self, entity: Entity) -> Option<u32>;
}

impl NetworkEntityMapExt for NetworkEntityMap {
    fn get_entity(&self, net_id: u32) -> Option<Entity> {
        self.entities.get(&net_id).copied()
    }

    fn get_net_id(&self, entity: Entity) -> Option<u32> {
        self.entities
            .iter()
            .find(|(_, &e)| e == entity)
            .map(|(&net_id, _)| net_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_entity_map_ext() {
        let mut map = NetworkEntityMap::default();
        let entity = Entity::from_bits(42); // Create test entity from bits

        map.entities.insert(100, entity);

        assert_eq!(map.get_entity(100), Some(entity));
        assert_eq!(map.get_net_id(entity), Some(100));
        assert_eq!(map.get_entity(999), None);
    }
}
