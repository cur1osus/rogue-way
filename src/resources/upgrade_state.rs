use bevy::prelude::*;

/// Global upgrade modifiers applied to pets and economy.
#[derive(Resource, Debug, Clone)]
pub struct UpgradeState {
    pub pet_damage_mult: f32,
    pub pet_attack_speed_mult: f32,
    pub pet_range_bonus: f32,
    pub pet_detection_range_bonus: f32,
    pub projectile_extra_shots: u32,
    pub projectile_pierce_bonus: u32,
    pub area_damage_radius: f32,
    pub gold_drop_mult: f32,
    pub is_choosing_upgrade: bool, // Флаг паузы во время выбора апгрейда
}

impl Default for UpgradeState {
    fn default() -> Self {
        Self {
            pet_damage_mult: 1.0,
            pet_attack_speed_mult: 1.0,
            pet_range_bonus: 0.0,
            pet_detection_range_bonus: 0.0,
            projectile_extra_shots: 0,
            projectile_pierce_bonus: 0,
            area_damage_radius: 0.0,
            gold_drop_mult: 1.0,
            is_choosing_upgrade: false,
        }
    }
}
