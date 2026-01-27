use bevy::prelude::*;

/// Маркер компонент игрока
#[derive(Component)]
pub struct Player;

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
