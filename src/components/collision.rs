use bevy::prelude::*;

/// Хитбокс сущности (ось-ориентированный прямоугольник)
#[derive(Component, Clone, Copy)]
pub struct Hitbox {
    pub half_size: Vec2,
}

impl Hitbox {
    pub fn from_full_size(size: Vec2) -> Self {
        Self {
            half_size: size * 0.5,
        }
    }
}

/// Слой коллизии и маска взаимодействия
#[derive(Component, Clone, Copy)]
pub struct CollisionLayer {
    pub group: u32,
    pub mask: u32,
}

impl CollisionLayer {
    pub const PLAYER: u32 = 1 << 0;
    pub const PET: u32 = 1 << 1;
    pub const ENEMY: u32 = 1 << 2;

    pub fn new(group: u32, mask: u32) -> Self {
        Self { group, mask }
    }

    pub fn player() -> Self {
        Self::new(Self::PLAYER, Self::ENEMY)
    }

    pub fn pet() -> Self {
        Self::new(Self::PET, Self::ENEMY)
    }

    pub fn enemy() -> Self {
        Self::new(Self::ENEMY, Self::PLAYER | Self::PET | Self::ENEMY)
    }

    pub fn collides_with(self, other: Self) -> bool {
        (self.mask & other.group) != 0 && (other.mask & self.group) != 0
    }
}
