use bevy::prelude::*;

/// Temporary hit flash marker.
#[derive(Component)]
pub struct HitFlash {
    pub timer: Timer,
    pub original_color: Color,
}

impl HitFlash {
    pub fn new(duration: f32, original_color: Color) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
            original_color,
        }
    }
}

/// Floating combat text (damage numbers).
#[derive(Component)]
pub struct FloatingText {
    pub timer: Timer,
    pub velocity: Vec2,
}

/// Simple particle for hit effects.
#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub timer: Timer,
    pub start_color: Color,
}

/// Marker for auto-animated effect sprites.
#[derive(Component)]
pub struct EffectSprite;

/// Generic timed despawn component.
#[derive(Component)]
pub struct TimedDespawn {
    pub timer: Timer,
}

/// Эффект отталкивания: применяет импульс к сущности (враги и т.п.)
#[derive(Component)]
pub struct KnockbackEffect {
    pub remaining: Vec2,
    /// Затухание в единицах/сек (линейное уменьшение длины remaining)
    pub decay_per_second: f32,
}
