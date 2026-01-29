use bevy::prelude::*;
use std::time::Duration;

use crate::components::Team;

use crate::constants::{pet_attack_ranges, pet_detection_ranges, pet_sizes};

/// Маркер компонент питомца
#[derive(Component)]
pub struct Pet {
    pub pet_type: PetType,
}

/// Типы питомцев
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetType {
    GuardDog,       // Damage 10, Speed 1.0/sec, Range 150, кружит вокруг игрока
    FireSprite,     // Damage 5, Speed 2.0/sec, Range 200, пробивающие снаряды
    SlimeCompanion, // Damage 15, Speed 0.5/sec, Range 100, замедляет врагов
    CrowScout,      // Damage 8, Speed 1.5/sec, Range 250, летает над препятствиями
    XpCollector,    // Собирает XP гемы, не атакует
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetRole {
    Melee,
    Ranged,
    Support,
    Collector,
}

impl PetRole {
    pub fn decision_interval(&self) -> f32 {
        match self {
            PetRole::Melee => 0.14,
            PetRole::Ranged => 0.16,
            PetRole::Support => 0.18,
            PetRole::Collector => 0.25,
        }
    }

    pub fn action_hold_time(&self) -> f32 {
        match self {
            PetRole::Melee => 0.35,
            PetRole::Ranged => 0.4,
            PetRole::Support => 0.45,
            PetRole::Collector => 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Follow,
    Engage,
    Regroup,
    Retreat,
    Assist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetAction {
    FollowFormation,
    Regroup,
    EngageTarget,
    PeelThreat,
    KeepRange,
    Fallback,
    HoldPosition,
    CollectXp,
}

#[derive(Component, Debug)]
pub struct PetBlackboard {
    pub state: PetState,
    pub action: PetAction,
    pub target: Option<Entity>,
    pub decision_timer: Timer,
    pub action_lock: Timer,
}

impl PetBlackboard {
    pub fn new(role: PetRole) -> Self {
        let decision_interval = role.decision_interval();
        let mut decision_timer = Timer::from_seconds(decision_interval, TimerMode::Repeating);
        decision_timer.tick(Duration::from_secs_f32(decision_interval));
        let mut action_lock = Timer::from_seconds(0.0, TimerMode::Once);
        action_lock.tick(Duration::from_secs_f32(0.0));
        Self {
            state: PetState::Follow,
            action: PetAction::FollowFormation,
            target: None,
            decision_timer,
            action_lock,
        }
    }
}

impl PetType {
    /// Получить отображаемое имя питомца
    pub fn get_name(&self) -> &'static str {
        match self {
            PetType::GuardDog => "Сторожевой пёс",
            PetType::FireSprite => "Огненная фея",
            PetType::SlimeCompanion => "Слизень-спутник",
            PetType::CrowScout => "Ворон-разведчик",
            PetType::XpCollector => "Пёс-собиратель опыта",
        }
    }

    /// Получить целевой размер спрайта питомца (в пикселях)
    pub fn get_size(&self) -> f32 {
        match self {
            PetType::GuardDog => pet_sizes::GUARD_DOG,
            PetType::FireSprite => pet_sizes::FIRE_SPRITE,
            PetType::SlimeCompanion => pet_sizes::SLIME_COMPANION,
            PetType::CrowScout => pet_sizes::CROW_SCOUT,
            PetType::XpCollector => pet_sizes::XP_COLLECTOR,
        }
    }

    pub fn get_role(&self) -> PetRole {
        match self {
            PetType::GuardDog => PetRole::Melee,
            PetType::SlimeCompanion => PetRole::Melee,
            PetType::FireSprite => PetRole::Ranged,
            PetType::CrowScout => PetRole::Ranged,
            PetType::XpCollector => PetRole::Collector,
        }
    }

    /// Получить строковый ID питомца для системы разблокировки
    pub fn to_string(&self) -> String {
        match self {
            PetType::GuardDog => "guard_dog".to_string(),
            PetType::FireSprite => "fire_sprite".to_string(),
            PetType::SlimeCompanion => "slime".to_string(),
            PetType::CrowScout => "crow_scout".to_string(),
            PetType::XpCollector => "xp_dog".to_string(),
        }
    }

    /// Получить базовые характеристики питомца
    pub fn get_stats(&self) -> (f32, f32, f32, f32, f32) {
        // (урон, скорость атаки, дальность атаки, радиус обнаружения, скорость движения)
        match self {
            PetType::GuardDog => (
                10.0,
                1.2,
                pet_attack_ranges::GUARD_DOG,
                pet_detection_ranges::GUARD_DOG,
                250.0,
            ),
            PetType::FireSprite => (
                5.0,
                2.0,
                pet_attack_ranges::FIRE_SPRITE,
                pet_detection_ranges::FIRE_SPRITE,
                180.0,
            ),
            PetType::SlimeCompanion => (
                15.0,
                0.6,
                pet_attack_ranges::SLIME_COMPANION,
                pet_detection_ranges::SLIME_COMPANION,
                140.0,
            ),
            PetType::CrowScout => (
                8.0,
                1.5,
                pet_attack_ranges::CROW_SCOUT,
                pet_detection_ranges::CROW_SCOUT,
                220.0,
            ),
            PetType::XpCollector => (
                0.0,
                1.0,
                pet_attack_ranges::XP_COLLECTOR,
                pet_detection_ranges::XP_COLLECTOR,
                140.0,
            ),
        }
    }

    /// Получить цвет спрайта питомца (временно вместо текстур)
    #[allow(dead_code)]
    pub fn get_color(&self) -> Color {
        match self {
            PetType::GuardDog => Color::srgb(0.6, 0.4, 0.2), // Коричневый
            PetType::FireSprite => Color::srgb(1.0, 0.3, 0.0), // Оранжевый
            PetType::SlimeCompanion => Color::srgb(0.2, 0.8, 0.3), // Зеленый
            PetType::CrowScout => Color::srgb(0.1, 0.1, 0.2), // Темно-синий
            PetType::XpCollector => Color::srgb(0.7, 0.5, 0.3), // Песочный
        }
    }
}

/// Урон атаки
#[derive(Component)]
pub struct Damage(pub f32);

/// Скорость атаки (атак в секунду)
#[derive(Component)]
pub struct AttackSpeed(pub f32);

/// Дальность атаки
#[derive(Component)]
pub struct AttackRange(pub f32);

/// Радиус обнаружения врагов
#[derive(Component)]
pub struct DetectionRange(pub f32);

/// Скорость движения питомца
#[derive(Component)]
pub struct PetMovementSpeed(pub f32);

/// Радиус подбора (для визуализации хитбоксов)
#[derive(Component)]
pub struct PickupRadius(pub f32);

/// Таймер кулдауна атаки
#[derive(Component)]
pub struct AttackTimer {
    pub timer: Timer,
}

impl AttackTimer {
    pub fn from_attack_speed(attacks_per_second: f32) -> Self {
        Self {
            timer: Timer::from_seconds(1.0 / attacks_per_second, TimerMode::Repeating),
        }
    }
}

#[derive(Component)]
pub struct AttackAnimation {
    pub timer: Timer,
}

impl AttackAnimation {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

#[derive(Component)]
pub struct PendingAttack {
    pub target: Entity,
    pub damage: f32,
    pub timer: Timer,
    pub area_cone_angle_deg: f32,
    pub attack_range: f32,
    pub team: Team,
    pub hit_particles: u32,
    pub hit_color: Color,
    pub apply_slow: bool,
    pub slime_heal_effect: bool,
    pub screen_shake: Option<(f32, f32)>,
}

/// Угол вращения вокруг игрока (для Guard Dog)
#[allow(dead_code)]
#[derive(Component)]
pub struct OrbitAngle(pub f32);

/// Радиус орбиты вокруг игрока (для Guard Dog)
#[allow(dead_code)]
#[derive(Component)]
pub struct OrbitRadius(pub f32);

/// Компонент для снарядов питомцев
#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec2,
    pub damage: f32,
    pub lifetime: Timer,
    pub piercing: bool,     // Пробивает ли врагов (для Fire Sprite)
    pub pierced_count: u32, // Сколько врагов пробито
    pub max_pierce: u32,    // Максимум пробиваний
    #[allow(dead_code)]
    pub area_radius: f32, // Радиус урона по площади (0 = без AoE)
}

/// Компонент замедления (для Slime Companion)
#[derive(Component)]
pub struct SlowEffect {
    pub slow_amount: f32, // Множитель скорости (0.5 = -50% скорости)
    pub duration: Timer,
}

/// Компонент цели атаки (для систем атаки питомцев)
#[derive(Component)]
pub struct AttackTarget {
    pub target_entity: Entity,
}
