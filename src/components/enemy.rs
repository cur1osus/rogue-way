use bevy::prelude::*;

use crate::constants::{boss_attack_ranges, boss_sizes, enemy_attack_ranges, enemy_sizes};

/// Маркер компонент врага
#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
}

/// Типы врагов
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnemyType {
    Bandit, // HP 20, Speed 80, Damage 10 - базовый враг
    Thief,  // HP 10, Speed 150, Damage 5 - быстрый враг
    Brute,  // HP 50, Speed 50, Damage 20 - танк
}

impl EnemyType {
    /// Получить базовые характеристики врага
    pub fn get_stats(&self) -> (f32, f32, f32, u32) {
        // (HP, скорость, урон, XP награда)
        match self {
            EnemyType::Bandit => (20.0, 80.0, 10.0, 10),
            EnemyType::Thief => (10.0, 150.0, 5.0, 15), // Больше XP за сложность
            EnemyType::Brute => (50.0, 50.0, 20.0, 25), // Еще больше XP
        }
    }

    /// Получить цвет спрайта врага (временно вместо текстур)
    pub fn get_color(&self) -> Color {
        match self {
            EnemyType::Bandit => Color::srgb(0.8, 0.2, 0.2), // Красный
            EnemyType::Thief => Color::srgb(0.6, 0.4, 0.8),  // Фиолетовый
            EnemyType::Brute => Color::srgb(0.4, 0.3, 0.2),  // Темно-коричневый
        }
    }

    /// Получить размер спрайта врага
    pub fn get_size(&self) -> f32 {
        match self {
            EnemyType::Bandit => enemy_sizes::BANDIT,
            EnemyType::Thief => enemy_sizes::THIEF,
            EnemyType::Brute => enemy_sizes::BRUTE,
        }
    }

    /// Получить награду в золоте
    pub fn get_gold_reward(&self) -> u32 {
        match self {
            EnemyType::Bandit => 5,
            EnemyType::Thief => 8,
            EnemyType::Brute => 15,
        }
    }

    /// Скорость атак (атак в секунду)
    pub fn get_attack_speed(&self) -> f32 {
        match self {
            EnemyType::Bandit => 1.0,
            EnemyType::Thief => 1.5,
            EnemyType::Brute => 0.7,
        }
    }

    /// Радиус атаки ближнего боя
    pub fn get_attack_range(&self) -> f32 {
        match self {
            EnemyType::Bandit => enemy_attack_ranges::BANDIT,
            EnemyType::Thief => enemy_attack_ranges::THIEF,
            EnemyType::Brute => enemy_attack_ranges::BRUTE,
        }
    }
}

/// Цель для AI (указывает на Entity игрока)
#[derive(Component)]
pub struct Target(pub Entity);

/// Маркер босса
#[derive(Component)]
pub struct Boss {
    pub boss_type: BossType,
}

/// Типы боссов
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BossType {
    BanditLeader,   // 5 минут - лидер бандитов
    ThiefKing,      // 10 минут - король воров
    BruteChieftain, // 15 минут - вождь громил
}

impl BossType {
    /// Получить базовые характеристики босса
    pub fn get_stats(&self) -> (f32, f32, f32, u32) {
        // (HP, скорость, урон, XP награда)
        match self {
            BossType::BanditLeader => (200.0, 60.0, 15.0, 100),
            BossType::ThiefKing => (150.0, 120.0, 20.0, 200),
            BossType::BruteChieftain => (400.0, 40.0, 30.0, 300),
        }
    }

    /// Получить цвет спрайта босса
    pub fn get_color(&self) -> Color {
        match self {
            BossType::BanditLeader => Color::srgb(0.9, 0.1, 0.1), // Ярко-красный
            BossType::ThiefKing => Color::srgb(0.8, 0.2, 0.9),    // Ярко-фиолетовый
            BossType::BruteChieftain => Color::srgb(0.5, 0.2, 0.1), // Темно-красный
        }
    }

    /// Получить размер спрайта босса
    pub fn get_size(&self) -> f32 {
        match self {
            BossType::BanditLeader => boss_sizes::BANDIT_LEADER,
            BossType::ThiefKing => boss_sizes::THIEF_KING,
            BossType::BruteChieftain => boss_sizes::BRUTE_CHIEFTAIN,
        }
    }

    /// Получить награду в золоте
    pub fn get_gold_reward(&self) -> u32 {
        match self {
            BossType::BanditLeader => 50,
            BossType::ThiefKing => 100,
            BossType::BruteChieftain => 200,
        }
    }

    /// Получить имя босса
    pub fn get_name(&self) -> &'static str {
        match self {
            BossType::BanditLeader => "Лидер Бандитов",
            BossType::ThiefKing => "Король Воров",
            BossType::BruteChieftain => "Вождь Громил",
        }
    }

    /// Скорость атак босса (атак в секунду)
    pub fn get_attack_speed(&self) -> f32 {
        match self {
            BossType::BanditLeader => 0.9,
            BossType::ThiefKing => 1.2,
            BossType::BruteChieftain => 0.6,
        }
    }

    /// Радиус атаки босса
    pub fn get_attack_range(&self) -> f32 {
        match self {
            BossType::BanditLeader => boss_attack_ranges::BANDIT_LEADER,
            BossType::ThiefKing => boss_attack_ranges::THIEF_KING,
            BossType::BruteChieftain => boss_attack_ranges::BRUTE_CHIEFTAIN,
        }
    }
}
