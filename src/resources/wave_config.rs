use bevy::prelude::*;

/// Конфигурация системы волн врагов
#[derive(Resource)]
pub struct WaveConfig {
    /// Таймер спавна врагов
    pub spawn_timer: Timer,
    /// Базовый интервал спавна (в секундах)
    pub base_spawn_interval: f32,
    /// Количество врагов за спавн
    pub enemies_per_spawn: u32,
    /// Множитель сложности (увеличивается со временем)
    pub difficulty_multiplier: f32,
    /// Общее время игры (для масштабирования сложности)
    pub game_time: f32,
    /// Флаги появления боссов
    pub boss_5min_spawned: bool,
    pub boss_10min_spawned: bool,
    pub boss_15min_spawned: bool,
}

impl Default for WaveConfig {
    fn default() -> Self {
        Self {
            spawn_timer: Timer::from_seconds(3.0, TimerMode::Repeating),
            base_spawn_interval: 5.0,
            enemies_per_spawn: 1,
            difficulty_multiplier: 1.0,
            game_time: 0.0,
            boss_5min_spawned: false,
            boss_10min_spawned: false,
            boss_15min_spawned: false,
        }
    }
}

impl WaveConfig {
    /// Обновление сложности на основе игрового времени
    pub fn update_difficulty(&mut self) {
        // Каждые 30 секунд увеличиваем сложность на 10%
        self.difficulty_multiplier = 1.0 + (self.game_time / 30.0) * 0.1;

        // Уменьшаем интервал спавна (быстрее спавнятся враги)
        let new_interval = (self.base_spawn_interval / self.difficulty_multiplier).max(0.5);
        self.spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(new_interval));

        // Увеличиваем количество врагов за спавн каждые 60 секунд
        self.enemies_per_spawn = 1 + (self.game_time / 60.0) as u32;
    }
}
