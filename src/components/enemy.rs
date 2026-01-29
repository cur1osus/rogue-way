use std::time::Duration;

use bevy::prelude::*;

use crate::constants::{boss_attack_ranges, boss_sizes, enemy_attack_ranges, enemy_sizes};

/// Маркер компонент врага
#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
}

/// Маркер индикатора HP над врагом
#[derive(Component)]
pub struct EnemyHpIndicator;

/// Секция индикатора HP над врагом
#[derive(Component)]
pub struct EnemyHpSegment {
    pub index: u8,
}

/// Текст HP внутри индикатора
#[derive(Component)]
pub struct EnemyHpText;

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
    #[allow(dead_code)]
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

    /// Получить профиль AI врага
    pub fn get_ai_profile(&self) -> EnemyAIProfile {
        match self {
            EnemyType::Bandit => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 420.0,
                    alert_range: 360.0,
                    lose_sight_after: 3.0,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.15,
                    action_hold_time: 0.35,
                    alert_duration: 0.35,
                    search_duration: 2.5,
                    preferred_distance: 55.0,
                    approach_bias: 1.0,
                    keep_distance_bias: 0.45,
                    flee_hp_ratio: 0.2,
                    flank_bias: 0.2,
                    heavy_attack_weight: 0.35,
                    heavy_attack_windup_mult: 1.6,
                    heavy_attack_damage_mult: 1.5,
                    heavy_attack_cooldown: 3.5,
                    heavy_attack_range_mult: 1.1,
                },
            },
            EnemyType::Thief => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 520.0,
                    alert_range: 420.0,
                    lose_sight_after: 2.2,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.12,
                    action_hold_time: 0.25,
                    alert_duration: 0.25,
                    search_duration: 2.0,
                    preferred_distance: 85.0,
                    approach_bias: 0.8,
                    keep_distance_bias: 1.1,
                    flee_hp_ratio: 0.35,
                    flank_bias: 0.45,
                    heavy_attack_weight: 0.2,
                    heavy_attack_windup_mult: 1.4,
                    heavy_attack_damage_mult: 1.2,
                    heavy_attack_cooldown: 2.5,
                    heavy_attack_range_mult: 1.0,
                },
            },
            EnemyType::Brute => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 380.0,
                    alert_range: 320.0,
                    lose_sight_after: 3.6,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.2,
                    action_hold_time: 0.45,
                    alert_duration: 0.45,
                    search_duration: 3.0,
                    preferred_distance: 50.0,
                    approach_bias: 1.3,
                    keep_distance_bias: 0.2,
                    flee_hp_ratio: 0.15,
                    flank_bias: 0.1,
                    heavy_attack_weight: 0.6,
                    heavy_attack_windup_mult: 2.1,
                    heavy_attack_damage_mult: 2.0,
                    heavy_attack_cooldown: 4.5,
                    heavy_attack_range_mult: 1.2,
                },
            },
        }
    }
}

/// Цель для AI (указывает на Entity игрока)
#[derive(Component)]
pub struct Target(#[allow(dead_code)] pub Entity);

/// Состояния AI врага (FSM)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    Idle,
    Alert,
    Combat,
    Search,
    Flee,
}

/// Действия AI врага (Utility AI)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAction {
    Idle,
    ApproachTarget,
    KeepDistance,
    BasicAttack,
    HeavyAttack,
    Retreat,
    SearchLastSeen,
}

/// Параметры восприятия
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyPerception {
    pub vision_range: f32,
    pub alert_range: f32,
    pub lose_sight_after: f32,
}

/// Конфиг поведения AI
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyAIConfig {
    pub decision_interval: f32,
    pub action_hold_time: f32,
    pub alert_duration: f32,
    pub search_duration: f32,
    pub preferred_distance: f32,
    pub approach_bias: f32,
    pub keep_distance_bias: f32,
    pub flee_hp_ratio: f32,
    pub flank_bias: f32,
    pub heavy_attack_weight: f32,
    pub heavy_attack_windup_mult: f32,
    pub heavy_attack_damage_mult: f32,
    pub heavy_attack_cooldown: f32,
    pub heavy_attack_range_mult: f32,
}

/// Память/blackboard врага
#[derive(Component, Debug)]
pub struct EnemyBlackboard {
    pub last_seen_pos: Option<Vec2>,
    pub last_seen_time: f32,
    pub current_action: EnemyAction,
    pub current_score: f32,
    pub decision_timer: Timer,
    pub action_lock: Timer,
    pub search_timer: Timer,
    pub alert_timer: Timer,
    pub heavy_cooldown: Timer,
}

impl EnemyBlackboard {
    pub fn new(config: &EnemyAIConfig) -> Self {
        let mut decision_timer =
            Timer::from_seconds(config.decision_interval, TimerMode::Repeating);
        decision_timer.reset();

        let mut action_lock = Timer::from_seconds(config.action_hold_time, TimerMode::Once);
        if config.action_hold_time > 0.0 {
            action_lock.tick(Duration::from_secs_f32(config.action_hold_time));
        }

        let search_timer = Timer::from_seconds(config.search_duration, TimerMode::Once);
        let alert_timer = Timer::from_seconds(config.alert_duration, TimerMode::Once);

        let mut heavy_cooldown = Timer::from_seconds(config.heavy_attack_cooldown, TimerMode::Once);
        if config.heavy_attack_cooldown > 0.0 {
            heavy_cooldown.tick(Duration::from_secs_f32(config.heavy_attack_cooldown));
        }

        Self {
            last_seen_pos: None,
            last_seen_time: 0.0,
            current_action: EnemyAction::Idle,
            current_score: 0.0,
            decision_timer,
            action_lock,
            search_timer,
            alert_timer,
            heavy_cooldown,
        }
    }
}

/// Профиль AI (перцепция + конфиг)
#[derive(Debug, Clone, Copy)]
pub struct EnemyAIProfile {
    pub perception: EnemyPerception,
    pub config: EnemyAIConfig,
}

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
            BossType::BanditLeader => (2000.0, 60.0, 15.0, 100),
            BossType::ThiefKing => (1500.0, 120.0, 20.0, 200),
            BossType::BruteChieftain => (4000.0, 40.0, 30.0, 300),
        }
    }

    /// Получить цвет спрайта босса
    #[allow(dead_code)]
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

    /// Получить профиль AI босса
    pub fn get_ai_profile(&self) -> EnemyAIProfile {
        match self {
            BossType::BanditLeader => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 500.0,
                    alert_range: 450.0,
                    lose_sight_after: 3.5,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.18,
                    action_hold_time: 0.45,
                    alert_duration: 0.4,
                    search_duration: 3.0,
                    preferred_distance: 55.0,
                    approach_bias: 1.2,
                    keep_distance_bias: 0.3,
                    flee_hp_ratio: 0.1,
                    flank_bias: 0.2,
                    heavy_attack_weight: 0.5,
                    heavy_attack_windup_mult: 2.0,
                    heavy_attack_damage_mult: 2.0,
                    heavy_attack_cooldown: 4.0,
                    heavy_attack_range_mult: 1.2,
                },
            },
            BossType::ThiefKing => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 560.0,
                    alert_range: 500.0,
                    lose_sight_after: 2.6,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.15,
                    action_hold_time: 0.35,
                    alert_duration: 0.3,
                    search_duration: 2.4,
                    preferred_distance: 90.0,
                    approach_bias: 0.9,
                    keep_distance_bias: 1.0,
                    flee_hp_ratio: 0.2,
                    flank_bias: 0.5,
                    heavy_attack_weight: 0.35,
                    heavy_attack_windup_mult: 1.6,
                    heavy_attack_damage_mult: 1.6,
                    heavy_attack_cooldown: 3.0,
                    heavy_attack_range_mult: 1.1,
                },
            },
            BossType::BruteChieftain => EnemyAIProfile {
                perception: EnemyPerception {
                    vision_range: 480.0,
                    alert_range: 420.0,
                    lose_sight_after: 4.0,
                },
                config: EnemyAIConfig {
                    decision_interval: 0.22,
                    action_hold_time: 0.5,
                    alert_duration: 0.5,
                    search_duration: 3.5,
                    preferred_distance: 60.0,
                    approach_bias: 1.4,
                    keep_distance_bias: 0.15,
                    flee_hp_ratio: 0.1,
                    flank_bias: 0.1,
                    heavy_attack_weight: 0.7,
                    heavy_attack_windup_mult: 2.3,
                    heavy_attack_damage_mult: 2.4,
                    heavy_attack_cooldown: 5.0,
                    heavy_attack_range_mult: 1.3,
                },
            },
        }
    }
}

/// Компонент анимации смерти врага
#[derive(Component)]
pub struct DeathAnimation {
    pub timer: Timer,
    pub animation_started: bool,
}

impl DeathAnimation {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
            animation_started: false,
        }
    }
}
