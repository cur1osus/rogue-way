use bevy::prelude::*;

use crate::constants::{PUSHBACK_CONE_ANGLE_DEG, PUSHBACK_FORCE, PUSHBACK_RADIUS};

/// Маркер компонент игрока
#[derive(Component)]
pub struct Player;

/// Идентификатор игрока для кооператива
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u32);

/// Маркер локального игрока (для камеры и ввода)
#[derive(Component)]
pub struct LocalPlayer;

/// Маркер удаленного игрока (сетевой клиент)
#[derive(Component)]
pub struct RemotePlayer;

/// Состояние ввода игрока (движение и действия)
#[derive(Component, Debug, Clone, Copy)]
pub struct PlayerInputState {
    pub movement: Vec2,
    pub pushback: bool,
}

impl Default for PlayerInputState {
    fn default() -> Self {
        Self {
            movement: Vec2::ZERO,
            pushback: false,
        }
    }
}

/// Здоровье сущности (базовое значение 100 HP для игрока)
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

/// Скорость движения (базовое значение 100 units/sec для игрока)
#[derive(Component)]
pub struct MovementSpeed(pub f32);

/// Вектор движения
#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

/// Физическая позиция (используется для физики и коллизий)
#[derive(Component)]
pub struct PhysicsPosition(pub Vec2);

impl Default for PhysicsPosition {
    fn default() -> Self {
        Self(Vec2::ZERO)
    }
}

/// Предыдущая физическая позиция (для интерполяции)
#[derive(Component)]
pub struct PreviousPhysicsPosition(pub Vec2);

impl Default for PreviousPhysicsPosition {
    fn default() -> Self {
        Self(Vec2::ZERO)
    }
}

/// Маркер команды (0 = игрок, 1 = враг)
#[derive(Component, PartialEq, Clone, Copy)]
pub struct Team(pub u8);

impl Team {
    pub const PLAYER: u8 = 0;
    pub const ENEMY: u8 = 1;
}

/// Диапазон кадров анимации в текстурном атласе.
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

/// Таймер анимации спрайта.
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/// Настройки анимации игрока.
#[derive(Component)]
pub struct PlayerAnimation {
    pub right_start: usize,
    pub left_start: usize,
    pub frames: usize,
    pub facing: i8,
}

/// Компонент атаки отталкивания
#[derive(Component)]
pub struct PushbackAttack {
    pub radius: f32,
    pub cone_angle: f32,
    pub pushback_force: f32,
    pub animation_timer: Timer,
    pub current_frame: usize,
    pub is_attacking: bool,
    /// Направление взгляда для атаки (нормализованный Vec2), обновляется из Velocity
    pub last_direction: Vec2,
}

impl Default for PushbackAttack {
    fn default() -> Self {
        Self {
            radius: PUSHBACK_RADIUS,
            cone_angle: PUSHBACK_CONE_ANGLE_DEG.to_radians(),
            pushback_force: PUSHBACK_FORCE,
            animation_timer: Timer::from_seconds(0.08, TimerMode::Repeating),
            current_frame: 0,
            is_attacking: false,
            last_direction: Vec2::X,
        }
    }
}

/// Маркер визуала конуса атаки отталкивания (дочерняя сущность игрока)
#[derive(Component)]
pub struct PushbackConeVisual;

/// Кулдаун атаки отталкивания
#[derive(Component)]
pub struct PushbackAttackCooldown {
    pub timer: Timer,
}

impl Default for PushbackAttackCooldown {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(1.5, TimerMode::Once);
        timer.tick(std::time::Duration::from_secs_f32(1.5));
        Self { timer }
    }
}

/// Маркер оверлея атаки отталкивания (дочерняя сущность игрока)
#[derive(Component)]
pub struct PlayerPushbackOverlay;

/// Маркер синего свечения, когда атака отталкивания готова
#[derive(Component)]
pub struct PushbackReadyGlow;

/// Маркер ожидания генерации текстуры свечения
#[derive(Component)]
pub struct PushbackReadyGlowPending;
