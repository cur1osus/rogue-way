use bevy::prelude::*;

/// Screen shake state updated each frame.
#[derive(Resource, Debug, Clone)]
pub struct ScreenShake {
    pub timer: Timer,
    pub intensity: f32,
    pub offset: Vec2,
}

impl Default for ScreenShake {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            intensity: 0.0,
            offset: Vec2::ZERO,
        }
    }
}

impl ScreenShake {
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        self.intensity = self.intensity.max(intensity);
        self.timer = Timer::from_seconds(duration, TimerMode::Once);
    }
}

/// Player damage flash state for UI overlay.
#[derive(Resource, Debug, Clone)]
pub struct PlayerDamageFlash {
    pub timer: Timer,
    pub intensity: f32,
}

impl Default for PlayerDamageFlash {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            intensity: 0.0,
        }
    }
}

impl PlayerDamageFlash {
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        self.intensity = self.intensity.max(intensity);
        self.timer = Timer::from_seconds(duration, TimerMode::Once);
    }
}
