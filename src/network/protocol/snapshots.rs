use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::quantization::{Dequantize, Quantize, Vec2i16};
use crate::network::legacy;

pub type NetId = u32;

// ===== FULL SNAPSHOT (для JoinOk) =====

/// Полный снапшот мира - отправляется при присоединении
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Snapshot {
    pub players: Vec<PlayerStateNet>,
    pub pets: Vec<PetStateNet>,
    pub enemies: Vec<EnemyStateNet>,
    pub projectiles: Vec<ProjectileStateNet>,
    pub xp_gems: Vec<XpGemStateNet>,
    pub gold: Vec<GoldStateNet>,
}

// ===== PLAYER STATE =====

/// Состояние игрока для сети (квантизованное)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerStateNet {
    pub id: NetId,
    pub player_id: u8,
    pub pos: Vec2i16,
    pub vel: Vec2i16,
    pub hp: u16,
    pub hp_max: u16,
    pub move_speed: u16, // speed * 10 для точности
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub gold: u32,
}

/// Конверсия из legacy::PlayerSnapshot
impl From<legacy::PlayerSnapshot> for PlayerStateNet {
    fn from(old: legacy::PlayerSnapshot) -> Self {
        Self {
            id: old.id,
            player_id: old.player_id as u8,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            vel: Vec2::new(old.vel[0], old.vel[1]).to_i16(),
            hp: old.hp as u16,
            hp_max: old.hp_max as u16,
            move_speed: (old.move_speed * 10.0) as u16,
            level: old.level,
            xp: old.xp,
            xp_next: old.xp_next,
            gold: old.gold,
        }
    }
}

// ===== PET STATE =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PetStateNet {
    pub id: NetId,
    pub owner_id: u8,
    pub pet_type: u8,
    pub pos: Vec2i16,
    pub vel: Vec2i16,
}

impl From<legacy::PetSnapshot> for PetStateNet {
    fn from(old: legacy::PetSnapshot) -> Self {
        Self {
            id: old.id,
            owner_id: old.owner_id as u8,
            pet_type: old.pet_type,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            vel: Vec2::new(old.vel[0], old.vel[1]).to_i16(),
        }
    }
}

// ===== ENEMY STATE =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnemyStateNet {
    pub id: NetId,
    pub enemy_type: u8,
    pub boss_type: Option<u8>,
    pub pos: Vec2i16,
    pub vel: Vec2i16,
    pub hp: u16,
    pub hp_max: u16,
}

impl From<legacy::EnemySnapshot> for EnemyStateNet {
    fn from(old: legacy::EnemySnapshot) -> Self {
        Self {
            id: old.id,
            enemy_type: old.enemy_type,
            boss_type: old.boss_type,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            vel: Vec2::new(old.vel[0], old.vel[1]).to_i16(),
            hp: old.hp as u16,
            hp_max: old.hp_max as u16,
        }
    }
}

// ===== PROJECTILE STATE =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectileStateNet {
    pub id: NetId,
    pub pos: Vec2i16,
    pub vel: Vec2i16,
}

impl From<legacy::ProjectileSnapshot> for ProjectileStateNet {
    fn from(old: legacy::ProjectileSnapshot) -> Self {
        Self {
            id: old.id,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            vel: Vec2::new(old.vel[0], old.vel[1]).to_i16(),
        }
    }
}

// ===== XP GEM STATE =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XpGemStateNet {
    pub id: NetId,
    pub pos: Vec2i16,
    pub value: u32,
}

impl From<legacy::XpSnapshot> for XpGemStateNet {
    fn from(old: legacy::XpSnapshot) -> Self {
        Self {
            id: old.id,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            value: old.value,
        }
    }
}

// ===== GOLD STATE =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldStateNet {
    pub id: NetId,
    pub pos: Vec2i16,
    pub value: u32,
}

impl From<legacy::GoldSnapshot> for GoldStateNet {
    fn from(old: legacy::GoldSnapshot) -> Self {
        Self {
            id: old.id,
            pos: Vec2::new(old.pos[0], old.pos[1]).to_i16(),
            value: old.value,
        }
    }
}

// ===== SPAWN NETWORK (для delta spawns) =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpawnNet {
    pub id: NetId,
    pub kind: EntityKind,
    pub pos: Vec2i16,
    pub vel: Vec2i16,
    pub hp: Option<u16>,
    pub hp_max: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum EntityKind {
    Player { player_id: u8 },
    Pet { pet_type: u8, owner_id: u8 },
    Enemy { enemy_type: u8, boss_type: Option<u8> },
    Projectile,
    XpGem { value: u32 },
    Gold { value: u32 },
}

// ===== HELPER FUNCTIONS =====

/// Конвертирует legacy snapshot в новый формат
pub fn convert_legacy_snapshot_to_net(
    players: Vec<legacy::PlayerSnapshot>,
    pets: Vec<legacy::PetSnapshot>,
    enemies: Vec<legacy::EnemySnapshot>,
    projectiles: Vec<legacy::ProjectileSnapshot>,
    xp_gems: Vec<legacy::XpSnapshot>,
    gold: Vec<legacy::GoldSnapshot>,
) -> Snapshot {
    Snapshot {
        players: players.into_iter().map(Into::into).collect(),
        pets: pets.into_iter().map(Into::into).collect(),
        enemies: enemies.into_iter().map(Into::into).collect(),
        projectiles: projectiles.into_iter().map(Into::into).collect(),
        xp_gems: xp_gems.into_iter().map(Into::into).collect(),
        gold: gold.into_iter().map(Into::into).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_snapshot_conversion() {
        let legacy = legacy::PlayerSnapshot {
            id: 123,
            player_id: 1,
            pos: [10.5, 20.75],
            vel: [1.0, -2.0],
            hp: 100.0,
            hp_max: 150.0,
            move_speed: 5.5,
            level: 10,
            xp: 500,
            xp_next: 1000,
            gold: 250,
        };

        let net: PlayerStateNet = legacy.into();

        assert_eq!(net.id, 123);
        assert_eq!(net.player_id, 1);
        assert_eq!(net.level, 10);
        assert_eq!(net.gold, 250);

        // Проверяем квантизацию
        let pos_restored = net.pos.to_f32();
        assert!((pos_restored.x - 10.5).abs() < 0.1);
        assert!((pos_restored.y - 20.75).abs() < 0.1);
    }

    #[test]
    fn test_enemy_snapshot_conversion() {
        let legacy = legacy::EnemySnapshot {
            id: 456,
            enemy_type: 2,
            boss_type: Some(1),
            pos: [100.0, 200.0],
            vel: [0.5, -0.5],
            hp: 500.0,
            hp_max: 1000.0,
        };

        let net: EnemyStateNet = legacy.into();

        assert_eq!(net.id, 456);
        assert_eq!(net.enemy_type, 2);
        assert_eq!(net.boss_type, Some(1));
        assert_eq!(net.hp, 500);
        assert_eq!(net.hp_max, 1000);
    }

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = Snapshot {
            players: vec![PlayerStateNet {
                id: 1,
                player_id: 1,
                pos: Vec2::new(10.0, 20.0).to_i16(),
                vel: Vec2::ZERO.to_i16(),
                hp: 100,
                hp_max: 100,
                move_speed: 50,
                level: 1,
                xp: 0,
                xp_next: 100,
                gold: 0,
            }],
            pets: vec![],
            enemies: vec![],
            projectiles: vec![],
            xp_gems: vec![],
            gold: vec![],
        };

        let serialized = postcard::to_allocvec(&snapshot).unwrap();
        let deserialized: Snapshot = postcard::from_bytes(&serialized).unwrap();

        assert_eq!(deserialized.players.len(), 1);
        assert_eq!(deserialized.players[0].id, 1);
    }

    #[test]
    fn test_snapshot_size() {
        // Один игрок должен быть компактным
        let player = PlayerStateNet {
            id: 1,
            player_id: 1,
            pos: Vec2::new(100.0, 100.0).to_i16(),
            vel: Vec2::ZERO.to_i16(),
            hp: 100,
            hp_max: 100,
            move_speed: 50,
            level: 5,
            xp: 250,
            xp_next: 500,
            gold: 100,
        };

        let serialized = postcard::to_allocvec(&player).unwrap();

        // Должен быть < 50 байт благодаря квантизации
        assert!(
            serialized.len() < 50,
            "Player snapshot too large: {} bytes",
            serialized.len()
        );
    }
}
